use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::events::{Event, EventSystem};
use crate::state::StateManager;
use super::agent::AgentContext;

/// Errors that can occur in behavior modules
#[derive(Debug, Error)]
pub enum BehaviorModuleError {
    #[error("Event error: {0}")]
    EventError(String),
    
    #[error("State error: {0}")]
    StateError(String),
    
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Behavior implementation error: {0}")]
    ImplementationError(String),
}

/// Trait for pluggable behavior modules
pub trait BehaviorModule: Send + Sync {
    /// Process an event
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError>;
    
    /// Check if this module can handle the given event
    fn can_handle_event(&self, event: &Event) -> bool;
    
    /// Get the required tools for this module
    fn get_required_tools(&self) -> Vec<String>;
}

/// Base implementation for common behavior module functionality
pub struct BaseBehaviorModule {
    name: String,
    event_patterns: Vec<String>,
    required_tools: Vec<String>,
    config: HashMap<String, String>,
}

impl BaseBehaviorModule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            event_patterns: Vec::new(),
            required_tools: Vec::new(),
            config: HashMap::new(),
        }
    }
    
    pub fn add_event_pattern(&mut self, pattern: &str) {
        self.event_patterns.push(pattern.to_string());
    }
    
    pub fn add_required_tool(&mut self, tool: &str) {
        self.required_tools.push(tool.to_string());
    }
    
    pub fn set_config(&mut self, key: &str, value: &str) {
        self.config.insert(key.to_string(), value.to_string());
    }
    
    pub fn get_config(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }
    
    pub fn matches_event_pattern(&self, event_name: &str) -> bool {
        self.event_patterns.iter().any(|pattern| {
            if pattern.ends_with("*") {
                let prefix = &pattern[0..pattern.len() - 1];
                event_name.starts_with(prefix)
            } else {
                event_name == pattern
            }
        })
    }
}

/// A behavior module that coordinates other modules and agents
pub struct CoordinatorModule {
    base: BaseBehaviorModule,
    coordination_map: HashMap<String, String>,
}

impl CoordinatorModule {
    pub fn new(name: &str) -> Self {
        let mut base = BaseBehaviorModule::new(name);
        
        // Common tools for coordinators
        base.add_required_tool("tool_forge_fs_read");
        base.add_required_tool("tool_forge_fs_list");
        base.add_required_tool("tool_forge_fs_search");
        base.add_required_tool("tool_forge_event_dispatch");
        
        Self {
            base,
            coordination_map: HashMap::new(),
        }
    }
    
    pub fn add_coordination_mapping(&mut self, event: &str, next_event: &str) {
        self.coordination_map.insert(event.to_string(), next_event.to_string());
    }
    
    pub fn get_next_event(&self, event: &str) -> Option<&String> {
        self.coordination_map.get(event)
    }
    
    pub fn dispatch_next_event(
        &self,
        event_system: &EventSystem,
        current_event: &str,
        payload: serde_json::Value,
    ) -> Result<(), BehaviorModuleError> {
        if let Some(next_event) = self.get_next_event(current_event) {
            let event = Event::Custom {
                name: next_event.clone(),
                payload,
            };
            
            event_system
                .emit(event)
                .map_err(|e| BehaviorModuleError::EventError(e.to_string()))?;
        }
        
        Ok(())
    }
}

impl BehaviorModule for CoordinatorModule {
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError> {
        // Basic implementation - should be extended in concrete coordinator modules
        if let Some(next_event) = self.get_next_event(&event.name()) {
            let event = Event::Custom {
                name: next_event.clone(),
                payload: event.payload(),
            };
            
            context.event_system
                .emit(event)
                .map_err(|e| BehaviorModuleError::EventError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    fn can_handle_event(&self, event: &Event) -> bool {
        self.base.matches_event_pattern(&event.name())
    }
    
    fn get_required_tools(&self) -> Vec<String> {
        self.base.required_tools.clone()
    }
}

/// A behavior module that analyzes content and produces results
pub struct AnalyzerModule {
    base: BaseBehaviorModule,
    analysis_depth: String,
}

impl AnalyzerModule {
    pub fn new(name: &str) -> Self {
        let mut base = BaseBehaviorModule::new(name);
        
        // Common tools for analyzers
        base.add_required_tool("tool_forge_fs_read");
        base.add_required_tool("tool_forge_fs_list");
        base.add_required_tool("tool_forge_fs_search");
        
        Self {
            base,
            analysis_depth: "standard".to_string(),
        }
    }
    
    pub fn set_analysis_depth(&mut self, depth: &str) {
        self.analysis_depth = depth.to_string();
    }
    
    pub fn get_analysis_depth(&self) -> &str {
        &self.analysis_depth
    }
}

impl BehaviorModule for AnalyzerModule {
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError> {
        // Basic implementation - should be extended in concrete analyzer modules
        Ok(())
    }
    
    fn can_handle_event(&self, event: &Event) -> bool {
        self.base.matches_event_pattern(&event.name())
    }
    
    fn get_required_tools(&self) -> Vec<String> {
        self.base.required_tools.clone()
    }
}

/// A behavior module that executes operations
pub struct ExecutorModule {
    base: BaseBehaviorModule,
}

impl ExecutorModule {
    pub fn new(name: &str) -> Self {
        let mut base = BaseBehaviorModule::new(name);
        
        // Common tools for executors
        base.add_required_tool("tool_forge_fs_read");
        base.add_required_tool("tool_forge_fs_create");
        base.add_required_tool("tool_forge_fs_patch");
        base.add_required_tool("tool_forge_process_shell");
        
        Self {
            base,
        }
    }
}

impl BehaviorModule for ExecutorModule {
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError> {
        // Basic implementation - should be extended in concrete executor modules
        Ok(())
    }
    
    fn can_handle_event(&self, event: &Event) -> bool {
        self.base.matches_event_pattern(&event.name())
    }
    
    fn get_required_tools(&self) -> Vec<String> {
        self.base.required_tools.clone()
    }
} 