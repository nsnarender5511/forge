use std::sync::Arc;
use log::{info, error};
use forge_agents::core::factory::AgentFactory;
use forge_agents::events::EventSystem;
use forge_agents::state::StateManager;

fn main() {
    // Initialize logging
    env_logger::init();
    
    info!("Starting agent system");
    
    // Create shared components
    let state_manager = Arc::new(StateManager::new());
    let event_system = Arc::new(EventSystem::new());
    
    // Load agents from YAML configuration
    let config_path = "templates/doc-content-syncer-config.yaml";
    match AgentFactory::load_agents_from_yaml(config_path, state_manager.clone(), event_system.clone()) {
        Ok(agents) => {
            info!("Loaded {} agents from configuration", agents.len());
            
            // Initialize each agent
            for agent in &agents {
                if let Err(e) = agent.initialize() {
                    error!("Failed to initialize agent {}: {}", agent.id(), e);
                } else {
                    info!("Initialized agent: {}", agent.id());
                }
            }
            
            // In a real application, you would keep the agents alive and handle events
            info!("Agent system running with configuration-driven agents. Press Ctrl+C to exit.");
            std::thread::park(); // Wait indefinitely
        },
        Err(e) => {
            error!("Failed to load agents from config file {}: {}", config_path, e);
        }
    }
} 