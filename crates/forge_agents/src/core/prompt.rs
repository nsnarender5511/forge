use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::Path;
use serde_json::Value;
use thiserror::Error;

/// Errors related to prompt template management
#[derive(Debug, Error)]
pub enum PromptError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Template rendering error: {0}")]
    RenderingError(String),
    
    #[error("Invalid template format: {0}")]
    InvalidFormat(String),
}

/// Manages prompt templates for agents
#[derive(Clone)]
pub struct PromptTemplateManager {
    templates: Arc<Mutex<HashMap<String, String>>>,
    base_path: String,
}

impl PromptTemplateManager {
    /// Create a new prompt template manager
    pub fn new() -> Self {
        Self {
            templates: Arc::new(Mutex::new(HashMap::new())),
            base_path: "templates".to_string(),
        }
    }
    
    /// Set the base path for template files
    pub fn with_base_path(mut self, path: &str) -> Self {
        self.base_path = path.to_string();
        self
    }
    
    /// Load a template from a file
    pub fn load_template(&self, name: &str, path: &str) -> Result<(), PromptError> {
        let full_path = Path::new(&self.base_path).join(path);
        let template_content = fs::read_to_string(&full_path)
            .map_err(|e| PromptError::IoError(e))?;
            
        let mut templates = self.templates.lock().unwrap();
        templates.insert(name.to_string(), template_content);
        
        Ok(())
    }
    
    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Result<String, PromptError> {
        let templates = self.templates.lock().unwrap();
        templates
            .get(name)
            .cloned()
            .ok_or_else(|| PromptError::TemplateNotFound(name.to_string()))
    }
    
    /// Set a template directly
    pub fn set_template(&self, name: &str, content: &str) {
        let mut templates = self.templates.lock().unwrap();
        templates.insert(name.to_string(), content.to_string());
    }
    
    /// Render a template with the given context
    pub fn render(&self, template_name: &str, context: &Value) -> Result<String, PromptError> {
        let template = self.get_template(template_name)?;
        
        // In a real implementation, you would use a template engine like Handlebars here
        // This is a very simple placeholder implementation
        let mut result = template.clone();
        
        // For now, just do simple variable substitution for demonstration
        if let Value::Object(obj) = context {
            for (key, value) in obj {
                let placeholder = format!("{{{{ {} }}}}", key);
                if let Value::String(str_value) = value {
                    result = result.replace(&placeholder, str_value);
                } else if let Ok(str_value) = serde_json::to_string(value) {
                    result = result.replace(&placeholder, &str_value);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Combine multiple templates into one
    pub fn combine_templates(&self, base_name: &str, module_names: &[&str]) -> Result<String, PromptError> {
        let base = self.get_template(base_name)?;
        let mut result = base;
        
        for module_name in module_names {
            let module_template = self.get_template(module_name)?;
            result.push_str("\n\n");
            result.push_str(&module_template);
        }
        
        Ok(result)
    }
} 