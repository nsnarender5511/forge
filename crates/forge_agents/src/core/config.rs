use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Configuration for a behavior module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorModuleConfig {
    /// Type of the module (coordinator, analyzer, executor, etc.)
    pub type_name: String,
    
    /// Additional parameters for the module
    #[serde(default)]
    pub params: HashMap<String, String>,
}

/// Configuration for a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateConfig {
    /// Name of the template
    pub name: String,
    
    /// Path to the template file
    pub path: String,
}

/// Configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent identifier
    pub id: String,
    
    /// Agent role
    pub role: String,
    
    /// Model to use for the agent
    pub model: String,
    
    /// Whether the agent supports tools
    #[serde(default)]
    pub tool_supported: bool,
    
    /// Maximum walker depth
    #[serde(default)]
    pub max_walker_depth: Option<u32>,
    
    /// Base system prompt template
    pub system_prompt: Option<String>,
    
    /// User prompt template
    pub user_prompt: Option<String>,
    
    /// Behavior modules
    #[serde(default)]
    pub behavior_modules: Vec<BehaviorModuleConfig>,
    
    /// Prompt templates
    #[serde(default)]
    pub prompts: Vec<PromptTemplateConfig>,
    
    /// Tools
    #[serde(default)]
    pub tools: Vec<String>,
    
    /// Event subscriptions
    #[serde(default)]
    pub subscribe: Vec<String>,
    
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Configuration for agents in an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// List of agent configurations
    pub agents: Vec<AgentConfig>,
    
    /// Model configurations
    #[serde(default)]
    pub models: HashMap<String, String>,
    
    /// Variable configurations
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

impl AgentsConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            models: HashMap::new(),
            variables: HashMap::new(),
        }
    }
    
    /// Add an agent configuration
    pub fn add_agent(&mut self, agent: AgentConfig) {
        self.agents.push(agent);
    }
    
    /// Add a model configuration
    pub fn add_model(&mut self, name: &str, identifier: &str) {
        self.models.insert(name.to_string(), identifier.to_string());
    }
    
    /// Add a variable
    pub fn add_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }
    
    /// Get an agent configuration by ID
    pub fn get_agent_by_id(&self, id: &str) -> Option<&AgentConfig> {
        self.agents.iter().find(|a| a.id == id)
    }
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self::new()
    }
} 