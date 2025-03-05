use std::sync::Arc;
use std::fmt;

use forge_domain::App;

use crate::conversation::ForgeConversationService;
use crate::provider::ForgeProviderService;
use crate::template::ForgeTemplateService;
use crate::tool_service::ForgeToolService;
use crate::Infrastructure;

/// ForgeApp is the main application container that implements the App trait.
/// It provides access to all core services required by the application.
///
/// Type Parameters:
/// - F: The infrastructure implementation that provides core services like
///   environment, file reading, vector indexing, and embedding.
pub struct ForgeApp<F> {
    infra: Arc<F>,
    tool_service: Arc<ForgeToolService>,
    provider_service: ForgeProviderService,
    conversation_service: ForgeConversationService,
    prompt_service: ForgeTemplateService<F, ForgeToolService>,
}

// Manual implementation of Debug for ForgeApp
impl<F: Infrastructure> fmt::Debug for ForgeApp<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForgeApp")
            .field("infra", &"Arc<Infrastructure>")
            .field("tool_service", &"Arc<ForgeToolService>")
            .field("provider_service", &"ForgeProviderService")
            .field("conversation_service", &"ForgeConversationService")
            .field("prompt_service", &"ForgeTemplateService")
            .finish()
    }
}

impl<F: Infrastructure> ForgeApp<F> {
    pub fn new(infra: Arc<F>) -> Self {
        let tool_service = Arc::new(ForgeToolService::new(infra.clone()));
        Self {
            infra: infra.clone(),
            provider_service: ForgeProviderService::new(infra.clone()),
            conversation_service: ForgeConversationService::new(),
            prompt_service: ForgeTemplateService::new(infra.clone(), tool_service.clone()),
            tool_service,
        }
    }
}

impl<F: Infrastructure> App for ForgeApp<F> {
    type ToolService = ForgeToolService;
    type ProviderService = ForgeProviderService;
    type ConversationService = ForgeConversationService;
    type TemplateService = ForgeTemplateService<F, ForgeToolService>;

    fn tool_service(&self) -> &Self::ToolService {
        &self.tool_service
    }

    fn provider_service(&self) -> &Self::ProviderService {
        &self.provider_service
    }

    fn conversation_service(&self) -> &Self::ConversationService {
        &self.conversation_service
    }

    fn template_service(&self) -> &Self::TemplateService {
        &self.prompt_service
    }
}

impl<F: Infrastructure> Infrastructure for ForgeApp<F> {
    type EnvironmentService = F::EnvironmentService;
    type FileReadService = F::FileReadService;
    type VectorIndex = F::VectorIndex;
    type EmbeddingService = F::EmbeddingService;

    fn environment_service(&self) -> &Self::EnvironmentService {
        self.infra.environment_service()
    }

    fn file_read_service(&self) -> &Self::FileReadService {
        self.infra.file_read_service()
    }

    fn vector_index(&self) -> &Self::VectorIndex {
        self.infra.vector_index()
    }

    fn embedding_service(&self) -> &Self::EmbeddingService {
        self.infra.embedding_service()
    }
}
