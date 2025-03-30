use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use std::time::SystemTime;

use crate::events::{Event, EventSystem};
use crate::state::StateManager;
use crate::state::doc_sync_state::{DocSyncState, StateError, IterationState};
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
    feedback_loop_enabled: bool,
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
            feedback_loop_enabled: true, // Enable feedback loop by default
        }
    }
    
    pub fn add_coordination_mapping(&mut self, event: &str, next_event: &str) {
        self.coordination_map.insert(event.to_string(), next_event.to_string());
    }
    
    pub fn get_next_event(&self, event: &str) -> Option<&String> {
        self.coordination_map.get(event)
    }
    
    pub fn set_feedback_loop_enabled(&mut self, enabled: bool) {
        self.feedback_loop_enabled = enabled;
    }
    
    pub fn is_feedback_loop_enabled(&self) -> bool {
        self.feedback_loop_enabled
    }
    
    pub fn setup_feedback_loop_mappings(&mut self) {
        // Standard flow mappings
        self.add_coordination_mapping("docs-verification-complete", "docs-plan-changes");
        self.add_coordination_mapping("docs-plan-changes", "docs-changes-planned");
        self.add_coordination_mapping("docs-changes-planned", "docs-implement-changes");
        self.add_coordination_mapping("docs-implement-changes", "docs-implementation-complete");
        self.add_coordination_mapping("docs-implementation-complete", "docs-verify");
        
        // Decision points
        self.add_coordination_mapping("docs-iteration-decision", "docs-iteration-complete");
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
    
    pub fn handle_verification_complete(
        &self, 
        context: &AgentContext,
        payload: serde_json::Value,
        state_manager: &StateManager
    ) -> Result<(), BehaviorModuleError> {
        if !self.is_feedback_loop_enabled() {
            // If feedback loop is disabled, follow the original flow
            return self.dispatch_next_event(
                &context.event_system, 
                "docs-verification-complete", 
                payload
            );
        }
        
        // Get the correlation ID from the payload
        let correlation_id = payload
            .get("correlation_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BehaviorModuleError::InvalidPayload("Missing correlation_id".to_string()))?
            .to_string();
        
        // First read current state
        let mut state = match state_manager.read_state(&correlation_id) {
            Ok(state) => state,
            Err(StateError::NotFound(_)) => DocSyncState::default(),
            Err(e) => return Err(BehaviorModuleError::StateError(format!("Failed to read state: {}", e))),
        };
        
        // Extract verification results from payload
        if let Some(verification_results) = payload.get("verification_results") {
            // Decision logic: check if requirements are satisfied or max iterations reached
            let decision_module = IterationDecisionModule::new("iteration_decision");
            let (continue_iteration, requirements_satisfied) = decision_module.evaluate_requirements(
                verification_results,
                &state.requirements_satisfied,
                state.current_iteration,
                state.max_iterations
            );
            
            // Update state with requirement satisfaction results
            state.requirements_satisfied = requirements_satisfied;
            
            // Create iteration state record
            let iteration_state = IterationState {
                iteration_number: state.current_iteration,
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                quality_scores: state.quality_scores.clone(),
                verification_results: state.requirements_satisfied.clone(),
                tasks_completed: state.completed_tasks.len(),
                tasks_pending: state.pending_tasks.len(),
                issues_found: state.error_log.len(),
                issues_resolved: 0, // This would be calculated more intelligently in practice
            };
            
            // Add to iteration history
            state.iteration_history.push(iteration_state);
            
            if continue_iteration {
                // Increment iteration counter for next cycle
                state.current_iteration += 1;
            }
            
            // Save updated state
            if let Err(e) = state_manager.write_state(&correlation_id, &state) {
                return Err(BehaviorModuleError::StateError(format!("Failed to write state: {}", e)));
            }
            
            // Create iteration decision payload
            let mut decision_payload = payload.clone();
            decision_payload["continue_iteration"] = serde_json::Value::Bool(continue_iteration);
            decision_payload["iteration_number"] = serde_json::Value::Number(
                serde_json::Number::from(state.current_iteration)
            );
            
            // Dispatch to appropriate next step based on decision
            if continue_iteration {
                // Go to planning step in the feedback loop
                self.dispatch_next_event(
                    &context.event_system,
                    "docs-verification-complete",
                    decision_payload
                )
                .map_err(|e| BehaviorModuleError::EventError(format!("Failed to dispatch next event: {}", e)))?;
            } else {
                // Complete the process
                self.dispatch_next_event(
                    &context.event_system,
                    "docs-iteration-decision",
                    decision_payload
                )
                .map_err(|e| BehaviorModuleError::EventError(format!("Failed to dispatch completion event: {}", e)))?;
            }
        }
        
        Ok(())
    }
}

impl BehaviorModule for CoordinatorModule {
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError> {
        let event_name = event.name();
        
        // Handle verification complete specially for feedback loop
        if event_name == "doc_sync.docs-verification-complete" {
            // Create a state manager to track iteration state
            let state_manager = match StateManager::new("/tmp/forge_doc_sync_state") {
                Ok(manager) => manager,
                Err(e) => return Err(BehaviorModuleError::StateError(format!("Failed to create state manager: {}", e))),
            };
                
            return self.handle_verification_complete(context, event.payload(), &state_manager);
        }
        
        // Basic implementation for other events
        if let Some(next_event) = self.get_next_event(&event_name) {
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
        base.add_required_tool("tool_forge_event_dispatch");
        
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

/// A behavior module that implements changes from planning in the feedback loop
pub struct ImplementerModule {
    base: BaseBehaviorModule,
}

impl ImplementerModule {
    pub fn new(name: &str) -> Self {
        let mut base = BaseBehaviorModule::new(name);
        
        // Common tools for implementers
        base.add_required_tool("tool_forge_fs_read");
        base.add_required_tool("tool_forge_fs_create");
        base.add_required_tool("tool_forge_fs_patch");
        base.add_required_tool("tool_forge_fs_remove");
        base.add_required_tool("tool_forge_event_dispatch");
        
        // Add specific event patterns
        base.add_event_pattern("docs-implement-changes");
        
        Self {
            base,
        }
    }
}

impl BehaviorModule for ImplementerModule {
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError> {
        // Implementation will be provided in a specialized module
        Ok(())
    }
    
    fn can_handle_event(&self, event: &Event) -> bool {
        self.base.matches_event_pattern(&event.name())
    }
    
    fn get_required_tools(&self) -> Vec<String> {
        self.base.required_tools.clone()
    }
}

/// A behavior module that makes decisions about iterations in the feedback loop
pub struct IterationDecisionModule {
    base: BaseBehaviorModule,
}

impl IterationDecisionModule {
    pub fn new(name: &str) -> Self {
        let mut base = BaseBehaviorModule::new(name);
        
        // Common tools for decision modules
        base.add_required_tool("tool_forge_fs_read");
        base.add_required_tool("tool_forge_event_dispatch");
        
        // Add specific event patterns
        base.add_event_pattern("docs-verification-complete");
        base.add_event_pattern("docs-iteration-decision");
        
        Self {
            base,
        }
    }
    
    /// Evaluate if requirements are satisfied and determine if another iteration is needed
    pub fn evaluate_requirements(
        &self,
        verification_results: &serde_json::Value,
        requirements: &HashMap<String, bool>,
        current_iteration: u32,
        max_iterations: u32
    ) -> (bool, HashMap<String, bool>) {
        let mut satisfied = HashMap::new();
        let mut all_satisfied = true;
        
        // Check each requirement (simplified version - would be more complex in real implementation)
        for (req_key, required_value) in requirements {
            // Simple requirement evaluation logic
            let is_satisfied = if let Some(value) = verification_results.get(req_key) {
                // For boolean requirements
                if let Some(bool_value) = value.as_bool() {
                    bool_value == *required_value
                } else {
                    // For numeric requirements - simplified
                    if let Some(num_value) = value.as_f64() {
                        num_value > 0.7 // Simple threshold
                    } else {
                        false
                    }
                }
            } else {
                false
            };
            
            satisfied.insert(req_key.clone(), is_satisfied);
            
            if !is_satisfied {
                all_satisfied = false;
            }
        }
        
        // If all requirements are satisfied or we've reached max iterations, don't continue
        let continue_iteration = !all_satisfied && current_iteration < max_iterations;
        
        (continue_iteration, satisfied)
    }
}

impl BehaviorModule for IterationDecisionModule {
    fn process_event(&self, event: &Event, context: &AgentContext) -> Result<(), BehaviorModuleError> {
        // Implementation will be provided in a specialized module
        Ok(())
    }
    
    fn can_handle_event(&self, event: &Event) -> bool {
        self.base.matches_event_pattern(&event.name())
    }
    
    fn get_required_tools(&self) -> Vec<String> {
        self.base.required_tools.clone()
    }
} 