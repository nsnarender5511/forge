use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use log::{debug, error, info};

use crate::events::EventSystem;
use crate::state::StateManager;
use super::agent::{Agent, AgentError};
use super::behavior::{
    BehaviorModule, BaseBehaviorModule, CoordinatorModule, AnalyzerModule, ExecutorModule
};
use super::config::{AgentConfig, BehaviorModuleConfig, AgentsConfig};
use super::prompt::PromptTemplateManager;
use super::role::AgentRole;

/// Factory for creating agents from configuration
pub struct AgentFactory;

impl AgentFactory {
    /// Create an agent from configuration
    pub fn create_agent(
        config: &AgentConfig,
        state_manager: Arc<StateManager>,
        event_system: Arc<EventSystem>,
    ) -> Result<Box<dyn Agent>, AgentError> {
        let role = AgentRole::from_str(&config.role);
        
        // Create a unified agent
        let mut agent = super::agent::UnifiedAgent::new(
            config.id.clone(),
            role,
            state_manager,
            event_system,
        );
        
        // Register event subscriptions
        for subscription in &config.subscribe {
            agent.add_subscription(subscription);
        }
        
        // Add metadata
        for (key, value) in &config.metadata {
            agent.set_metadata(key, value);
        }
        
        // Create prompt manager
        let prompt_manager = Self::create_prompt_manager(config)?;
        agent.set_prompt_manager(prompt_manager);
        
        // Create behavior modules
        for module_config in &config.behavior_modules {
            let module = Self::create_behavior_module(module_config)?;
            agent.add_behavior_module(module);
        }
        
        Ok(Box::new(agent))
    }
    
    /// Create a prompt template manager from configuration
    fn create_prompt_manager(config: &AgentConfig) -> Result<PromptTemplateManager, AgentError> {
        let mut prompt_manager = PromptTemplateManager::new();
        
        // Set base system prompt if available
        if let Some(system_prompt) = &config.system_prompt {
            prompt_manager.set_template("system", system_prompt);
        }
        
        // Set user prompt if available
        if let Some(user_prompt) = &config.user_prompt {
            prompt_manager.set_template("user", user_prompt);
        }
        
        // Add additional templates
        for template in &config.prompts {
            // In a real implementation, you would load templates from files
            // Here we're just setting the path as the content for simplicity
            prompt_manager.set_template(&template.name, &template.path);
        }
        
        Ok(prompt_manager)
    }
    
    /// Create a behavior module from configuration
    fn create_behavior_module(config: &BehaviorModuleConfig) -> Result<Box<dyn BehaviorModule>, AgentError> {
        match config.type_name.as_str() {
            "coordinator" => {
                let mut module = CoordinatorModule::new(&config.type_name);
                
                // Configure coordination mappings if present
                for (key, value) in &config.params {
                    if key.starts_with("map_") {
                        let event = key.strip_prefix("map_").unwrap_or("");
                        module.add_coordination_mapping(event, value);
                    }
                }
                
                Ok(Box::new(module))
            },
            "analyzer" => {
                let mut module = AnalyzerModule::new(&config.type_name);
                
                // Configure analysis depth if present
                if let Some(depth) = config.params.get("analysis_depth") {
                    module.set_analysis_depth(depth);
                }
                
                Ok(Box::new(module))
            },
            "executor" => {
                let module = ExecutorModule::new(&config.type_name);
                Ok(Box::new(module))
            },
            _ => Err(AgentError::ConfigurationError(
                format!("Unknown behavior module type: {}", config.type_name)
            )),
        }
    }
    
    /// Load agents from configuration file
    pub fn load_agents(
        config_path: &str,
        state_manager: Arc<StateManager>,
        event_system: Arc<EventSystem>,
    ) -> Result<Vec<Box<dyn Agent>>, AgentError> {
        // In a real implementation, you would load from a file
        // Here we're creating a simple example config
        
        let mut agents = Vec::new();
        
        // Load the configuration file
        // In a real implementation, you would use serde to deserialize from YAML
        info!("Loading agents from configuration: {}", config_path);
        
        // Example of creating a simple agent
        let config = AgentConfig {
            id: "example-agent".to_string(),
            role: "DocCoordinator".to_string(),
            model: "anthropic/claude-3.5-sonnet".to_string(),
            tool_supported: true,
            max_walker_depth: Some(1024),
            system_prompt: Some("You are a helpful assistant".to_string()),
            user_prompt: None,
            behavior_modules: vec![
                BehaviorModuleConfig {
                    type_name: "coordinator".to_string(),
                    params: HashMap::new(),
                }
            ],
            prompts: vec![],
            tools: vec![
                "tool_forge_fs_read".to_string(),
                "tool_forge_fs_list".to_string(),
            ],
            subscribe: vec!["docs".to_string()],
            metadata: HashMap::new(),
        };
        
        // Create the agent
        match Self::create_agent(&config, state_manager.clone(), event_system.clone()) {
            Ok(agent) => {
                agents.push(agent);
            },
            Err(e) => {
                error!("Failed to create agent {}: {}", config.id, e);
            }
        }
        
        Ok(agents)
    }

    /// Load agents from a YAML configuration file
    pub fn load_agents_from_yaml(
        config_path: &str,
        state_manager: Arc<StateManager>,
        event_system: Arc<EventSystem>,
    ) -> Result<Vec<Box<dyn Agent>>, AgentError> {
        info!("Loading agents from YAML configuration: {}", config_path);
        
        // Read the YAML file
        let yaml_content = fs::read_to_string(config_path)
            .map_err(|e| AgentError::ConfigurationError(
                format!("Failed to read configuration file {}: {}", config_path, e)
            ))?;
        
        // Parse the YAML content
        let config: AgentsConfig = serde_yaml::from_str(&yaml_content)
            .map_err(|e| AgentError::ConfigurationError(
                format!("Failed to parse YAML configuration: {}", e)
            ))?;
        
        let mut agents = Vec::new();
        
        // Create agents from configuration
        for agent_config in &config.agents {
            match Self::create_agent(agent_config, state_manager.clone(), event_system.clone()) {
                Ok(agent) => {
                    info!("Created agent: {}", agent_config.id);
                    agents.push(agent);
                },
                Err(e) => {
                    error!("Failed to create agent {}: {}", agent_config.id, e);
                    // Continue with other agents even if one fails
                }
            }
        }
        
        Ok(agents)
    }
} 