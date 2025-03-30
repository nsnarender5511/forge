pub mod core;
pub mod events;
pub mod state;
pub mod utils;

// Re-export core components for convenience
pub use core::agent::{Agent, AgentError};
pub use core::behavior::{BehaviorModule, BehaviorModuleError};
pub use core::config::{AgentConfig, BehaviorModuleConfig};
pub use core::factory::AgentFactory;
pub use core::prompt::PromptTemplateManager;
pub use core::role::AgentRole;
