use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::events::{Event, EventError, EventSystem};
use crate::state::StateManager;
use super::behavior::{BehaviorModule, BehaviorModuleError};
use super::role::AgentRole;
use super::prompt::PromptTemplateManager;

/// Errors that can occur during Agent operations
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Event system error: {0}")]
    EventSystemError(#[from] EventError),
    
    #[error("Behavior module error: {0}")]
    BehaviorModuleError(#[from] BehaviorModuleError),
    
    #[error("State error: {0}")]
    StateError(String),
    
    #[error("Invalid event payload: {0}")]
    InvalidPayload(String),
    
    #[error("Agent configuration error: {0}")]
    ConfigurationError(String),
}

/// Context provided to behavior modules
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub agent_id: String,
    pub role: AgentRole,
    pub state_manager: Arc<StateManager>,
    pub event_system: Arc<EventSystem>,
    pub metadata: HashMap<String, String>,
}

/// Common trait for all agents
pub trait Agent: Send + Sync {
    /// Get the agent's identifier
    fn id(&self) -> &str;
    
    /// Get the agent's role
    fn role(&self) -> AgentRole;
    
    /// Initialize the agent by registering event handlers
    fn initialize(&self) -> Result<(), AgentError>;
    
    /// Process an event
    fn process_event(&self, event: Event) -> Result<(), AgentError>;
    
    /// Get the metadata for the agent
    fn metadata(&self) -> HashMap<String, String>;
}

/// Unified agent implementation that delegates to behavior modules
pub struct UnifiedAgent {
    id: String,
    role: AgentRole,
    behavior_modules: Vec<Box<dyn BehaviorModule>>,
    state_manager: Arc<StateManager>,
    event_system: Arc<EventSystem>,
    prompt_manager: PromptTemplateManager,
    metadata: HashMap<String, String>,
    event_subscriptions: Vec<String>,
}

impl UnifiedAgent {
    /// Create a new UnifiedAgent
    pub fn new(
        id: String,
        role: AgentRole,
        state_manager: Arc<StateManager>,
        event_system: Arc<EventSystem>,
    ) -> Self {
        Self {
            id,
            role,
            behavior_modules: Vec::new(),
            state_manager,
            event_system,
            prompt_manager: PromptTemplateManager::new(),
            metadata: HashMap::new(),
            event_subscriptions: Vec::new(),
        }
    }
    
    /// Add a behavior module to the agent
    pub fn add_behavior_module(&mut self, module: Box<dyn BehaviorModule>) {
        self.behavior_modules.push(module);
    }
    
    /// Add an event subscription
    pub fn add_subscription(&mut self, event_name: &str) {
        self.event_subscriptions.push(event_name.to_string());
    }
    
    /// Set the prompt template manager
    pub fn set_prompt_manager(&mut self, prompt_manager: PromptTemplateManager) {
        self.prompt_manager = prompt_manager;
    }
    
    /// Set metadata for the agent
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }
    
    /// Register event handlers for all subscriptions
    fn register_event_handlers(&self) -> Result<(), AgentError> {
        for event_name in &self.event_subscriptions {
            let agent_clone = Arc::new(self.clone());
            let event_name_clone = event_name.clone();
            
            self.event_system.register_handler(
                &event_name,
                Box::new(move |event| {
                    let agent = agent_clone.clone();
                    if let Err(e) = agent.process_event(event) {
                        log::error!("Error handling event {}: {}", event_name_clone, e);
                    }
                    Ok(())
                }),
            )?;
        }
        
        Ok(())
    }
    
    /// Create a context for behavior modules
    fn create_context(&self) -> AgentContext {
        AgentContext {
            agent_id: self.id.clone(),
            role: self.role.clone(),
            state_manager: self.state_manager.clone(),
            event_system: self.event_system.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl Agent for UnifiedAgent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn role(&self) -> AgentRole {
        self.role.clone()
    }
    
    fn initialize(&self) -> Result<(), AgentError> {
        self.register_event_handlers()?;
        Ok(())
    }
    
    fn process_event(&self, event: Event) -> Result<(), AgentError> {
        let context = self.create_context();
        
        // Find a behavior module that can handle this event
        for module in &self.behavior_modules {
            if module.can_handle_event(&event) {
                return module.process_event(&event, &context).map_err(AgentError::from);
            }
        }
        
        // No module could handle the event
        log::warn!("No behavior module could handle event: {}", event.name());
        Ok(())
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }
}

impl Clone for UnifiedAgent {
    fn clone(&self) -> Self {
        // Note: behavior_modules can't be cloned directly due to Box<dyn Trait>
        // In a real implementation, you'd need a more sophisticated approach
        // Here we're creating a new agent without modules
        let mut new_agent = Self::new(
            self.id.clone(),
            self.role.clone(),
            self.state_manager.clone(),
            self.event_system.clone(),
        );
        
        new_agent.prompt_manager = self.prompt_manager.clone();
        new_agent.metadata = self.metadata.clone();
        new_agent.event_subscriptions = self.event_subscriptions.clone();
        
        new_agent
    }
} 