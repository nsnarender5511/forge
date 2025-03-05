use std::sync::Arc;

use forge_app::{EnvironmentService, Infrastructure};
use forge_domain::{
    AgentMessage, App, ChatRequest, ChatResponse, Orchestrator, SystemContext, ToolService,
};
use forge_stream::MpscStream;
use forge_walker::Walker;

#[derive(Debug)]
pub struct ForgeExecutorService<F> {
    infra: Arc<F>,
}
impl<F: Infrastructure + App> ForgeExecutorService<F> {
    pub fn new(infra: Arc<F>) -> Self {
        Self { infra }
    }
}

impl<F: Infrastructure + App> ForgeExecutorService<F> {
    pub async fn chat(
        &self,
        request: ChatRequest,
    ) -> anyhow::Result<MpscStream<anyhow::Result<AgentMessage<ChatResponse>>>> {
        println!("chat request ::  {:?}", request);
        let env = self.infra.environment_service().get_environment();
        let mut files = Walker::max_all()
            .max_depth(4)
            .cwd(env.cwd.clone())
            .get()
            .await?
            .iter()
            .map(|f| f.path.to_string())
            .collect::<Vec<_>>();

        // Sort the files alphabetically to ensure consistent ordering
        files.sort();

        //println!("files {:?}", files);

        let ctx = SystemContext {
            env: Some(env),
            tool_information: Some(self.infra.tool_service().usage_prompt()),
            tool_supported: Some(true),
            files,
        };
        //println!("ctx {:?}", ctx);

        let app = self.infra.clone();
        
        Ok(MpscStream::spawn(move |tx| async move {
            let tx = Arc::new(tx);
            let orch = Orchestrator::new(app, request, ctx, Some(tx.clone()));
            match orch.execute().await {
                Ok(_) => {}
                Err(err) => tx.send(Err(err)).await.unwrap(),
            }
        }))
    }
}
