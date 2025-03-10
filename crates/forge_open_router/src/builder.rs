// Context trait is needed for error handling in the provider implementations

use anyhow::{Context as _, Result};
use forge_domain::{
    ChatCompletionMessage, Context, Model, ModelId, Parameters, Provider, ProviderService,
    ResultStream,
};

use crate::anthropic::Anthropic;
use crate::open_router::OpenRouter;

pub enum Client {
    OpenAICompat(OpenRouter),
    Anthropic(Anthropic),
}

impl Client {
    pub fn new(provider: Provider) -> Result<Self> {
        let client = reqwest::Client::builder().build()?;

        match &provider {
            Provider::OpenAI { url, .. } => Ok(Client::OpenAICompat(
                OpenRouter::builder()
                    .client(client)
                    .provider(provider.clone())
                    .build()
                    .with_context(|| format!("Failed to initialize: {}", url))?,
            )),

            Provider::Anthropic { key } => Ok(Client::Anthropic(
                Anthropic::builder()
                    .client(client)
                    .api_key(key.to_string())
                    .build()
                    .with_context(|| {
                        format!("Failed to initialize: {}", Provider::ANTHROPIC_URL)
                    })?,
            )),
        }
    }
}

#[async_trait::async_trait]
impl ProviderService for Client {
    async fn chat(
        &self,
        id: &ModelId,
        context: Context,
    ) -> ResultStream<ChatCompletionMessage, anyhow::Error> {
        match self {
            Client::OpenAICompat(provider) => provider.chat(id, context).await,
            Client::Anthropic(provider) => provider.chat(id, context).await,
        }
    }

    async fn models(&self) -> anyhow::Result<Vec<Model>> {
        match self {
            Client::OpenAICompat(provider) => provider.models().await,
            Client::Anthropic(provider) => provider.models().await,
        }
    }

    async fn parameters(&self, model: &ModelId) -> anyhow::Result<Parameters> {
        match self {
            Client::OpenAICompat(provider) => provider.parameters(model).await,
            Client::Anthropic(provider) => provider.parameters(model).await,
        }
    }
}
