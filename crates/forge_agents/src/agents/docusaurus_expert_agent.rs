use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::events::{
    EventError, EventSystem, DOCS_ANALYZE_DOCUSAURUS, DOCS_DOCUSAURUS_ANALYZED
};
use crate::events::Event as CrateEvent;
use crate::events::doc_sync_events::{
    AnalysisReport, AnalyzeDocusaurusPayload, DocusaurusAnalyzedPayload, DocumentationMap,
    Finding, Operation, OperationType, Severity, StructureOperation
};
use crate::state::StateManager;
use crate::utils::{path_exists, read_file_to_string};

/// Errors that can occur during Docusaurus Expert Agent operations
#[derive(Debug, Error)]
pub enum DocusaurusExpertError {
    #[error("State error: {0}")]
    StateError(String),

    #[error("Event system error: {0}")]
    EventSystemError(#[from] EventError),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Docusaurus analysis error: {0}")]
    DocusaurusAnalysisError(String),

    #[error("Path processing error: {0}")]
    PathError(String),
}

/// Docusaurus Expert Agent analyzes Docusaurus configuration and structure
pub struct DocusaurusExpertAgent {
    state_manager: StateManager,
    event_system: Arc<EventSystem>,
}

/// Result of Docusaurus analysis
pub struct DocusaurusAnalysisResult {
    pub report: AnalysisReport,
    pub structure_operations: Vec<StructureOperation>,
    pub updated_documentation_map: DocumentationMap,
}

impl DocusaurusExpertAgent {
    /// Create a new Docusaurus Expert Agent
    pub fn new(state_manager: StateManager, event_system: Arc<EventSystem>) -> Self {
        Self {
            state_manager,
            event_system,
        }
    }

    /// Initialize the agent and register event handlers
    pub fn initialize(&self) -> Result<(), DocusaurusExpertError> {
        info!("Initializing Docusaurus Expert Agent");
        
        // Register event handler for Docusaurus analysis
        let event_system = self.event_system.clone();
        let agent = self.clone();

        self.event_system.register_handler(DOCS_ANALYZE_DOCUSAURUS, Box::new(move |event| {
            debug!("Received DOCS_ANALYZE_DOCUSAURUS event");
            let result = agent.handle_analyze_docusaurus_event(&event);
            if let Err(e) = &result {
                error!("Error handling DOCS_ANALYZE_DOCUSAURUS event: {:?}", e);
            }
            match result {
                Ok(_) => Ok(()),
                Err(e) => Err(EventError::HandlerError(format!("{}", e))),
            }
        }))?;

        info!("DocusaurusExpertAgent initialized and registered for events");
        Ok(())
    }

    /// Handle the DOCS_ANALYZE_DOCUSAURUS event
    fn handle_analyze_docusaurus_event(&self, event: &CrateEvent) -> Result<(), DocusaurusExpertError> {
        // Parse the payload
        let payload = match event.payload().clone() {
            serde_json::Value::Object(obj) => {
                if let Some(serde_json::Value::String(target_path)) = obj.get("target_path") {
                    let doc_map = obj.get("doc_map").and_then(|v| {
                        serde_json::from_value::<Option<DocumentationMap>>(v.clone()).ok()
                    }).flatten();
                    
                    AnalyzeDocusaurusPayload {
                        target_path: target_path.clone(),
                        doc_map,
                    }
                } else {
                    return Err(DocusaurusExpertError::InvalidPayload(
                        "Missing target_path in payload".to_string()
                    ));
                }
            }
            _ => {
                return Err(DocusaurusExpertError::InvalidPayload(
                    "Invalid payload format".to_string()
                ));
            }
        };

        // Validate the target path
        let target_path = PathBuf::from(&payload.target_path);
        if !path_exists(&target_path) {
            return Err(DocusaurusExpertError::InvalidPayload(format!(
                "Target path does not exist: {}",
                target_path.display()
            )));
        }

        // Get the doc_map from payload or create a new one
        let doc_map = payload.doc_map.unwrap_or_else(|| {
            // Create a new default DocumentationMap 
            DocumentationMap::default()
        });

        // Perform Docusaurus analysis
        info!("Analyzing Docusaurus at target path: {}", target_path.display());
        let analysis_result = self.analyze_docusaurus(&target_path, doc_map)?;

        // Construct and emit response
        let response_payload = DocusaurusAnalyzedPayload {
            doc_map: analysis_result.updated_documentation_map,
            structure_operations: analysis_result.structure_operations,
            analysis_report: analysis_result.report,
        };

        let response_event = CrateEvent::Custom {
            name: DOCS_DOCUSAURUS_ANALYZED.to_string(),
            payload: serde_json::to_value(response_payload).map_err(|e| {
                DocusaurusExpertError::EventSystemError(EventError::SerializationError(e.to_string()))
            })?,
        };

        info!("Emitting DOCS_DOCUSAURUS_ANALYZED event");
        self.event_system.emit(response_event)?;

        Ok(())
    }

    /// Analyze Docusaurus configuration and structure
    fn analyze_docusaurus(&self, target_path: &Path, mut doc_map: DocumentationMap) 
        -> Result<DocusaurusAnalysisResult, DocusaurusExpertError> {
        
        info!("Starting Docusaurus analysis in {}", target_path.display());
        
        let mut findings = Vec::new();
        let mut operations = Vec::new();
        let mut structure_operations = Vec::new();
        
        // Validate Docusaurus configuration file exists
        let config_path = target_path.join("docusaurus.config.js");
        if !config_path.exists() {
            findings.push(Finding {
                category: "Configuration".to_string(),
                severity: Severity::Critical,
                message: "docusaurus.config.js file is missing".to_string(),
                file_path: Some(config_path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Create a docusaurus.config.js file at the root of your Docusaurus project".to_string()),
            });
        } else {
            // Analyze Docusaurus configuration
            let config_content = read_file_to_string(&config_path)
                .map_err(|e| DocusaurusExpertError::IoError(format!("Failed to read config file: {}", e)))?;
            
            findings.extend(self.analyze_docusaurus_config(&config_content, &config_path));
            
            // Check if there are any configuration improvements to make
            if findings.iter().any(|f| f.category == "Configuration") {
                structure_operations.push(StructureOperation {
                    operation_type: "update_config".to_string(),
                    target_path: config_path.to_string_lossy().to_string(),
                    content: None, // This would be populated with a suggested improved configuration
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Validate sidebars file exists
        let sidebars_path = target_path.join("sidebars.js");
        if !sidebars_path.exists() {
            findings.push(Finding {
                category: "Structure".to_string(),
                severity: Severity::High,
                message: "sidebars.js file is missing".to_string(),
                file_path: Some(sidebars_path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Create a sidebars.js file to define the sidebar structure".to_string()),
            });
        } else {
            // Analyze sidebars configuration
            let sidebars_content = read_file_to_string(&sidebars_path)
                .map_err(|e| DocusaurusExpertError::IoError(format!("Failed to read sidebars file: {}", e)))?;
            
            findings.extend(self.analyze_sidebar_config(&sidebars_content, &sidebars_path));
            
            // Check if there are any sidebar improvements to make
            if findings.iter().any(|f| f.category == "Sidebar") {
                structure_operations.push(StructureOperation {
                    operation_type: "update_sidebars".to_string(),
                    target_path: sidebars_path.to_string_lossy().to_string(),
                    content: None, // This would be populated with a suggested improved sidebar config
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Check for theme customization
        let theme_path = target_path.join("src/theme");
        if !theme_path.exists() {
            findings.push(Finding {
                category: "Theme".to_string(),
                severity: Severity::Low,
                message: "No theme customization found".to_string(),
                file_path: Some(theme_path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Consider adding theme customization for a more unique look".to_string()),
            });
        }
        
        // Update document map with structure information
        // Using metadata HashMap<String, String> from DocumentationMap
        doc_map.metadata.insert("docusaurus_version".to_string(), "2.x".to_string());
        doc_map.metadata.insert("has_custom_theme".to_string(), theme_path.exists().to_string());
        doc_map.metadata.insert("analyzed_timestamp".to_string(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string());
        
        // Create analysis report
        let analysis_report = AnalysisReport {
            total_items: findings.len(),
            findings: findings.clone(),
            operations: operations,
            summary: format!("Found {} issues in Docusaurus structure", findings.len()),
        };
        
        Ok(DocusaurusAnalysisResult {
            report: analysis_report,
            structure_operations,
            updated_documentation_map: doc_map,
        })
    }
    
    /// Analyze the Docusaurus configuration file
    fn analyze_docusaurus_config(&self, content: &str, path: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();
        
        // Check if the configuration contains required fields
        if !content.contains("title:") {
            findings.push(Finding {
                category: "Configuration".to_string(),
                severity: Severity::Medium,
                message: "Missing 'title' field in Docusaurus configuration".to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Add a 'title' field to your site configuration".to_string()),
            });
        }
        
        // Check for proper presets configuration
        if !content.contains("@docusaurus/preset-classic") {
            findings.push(Finding {
                category: "Configuration".to_string(),
                severity: Severity::Medium,
                message: "No @docusaurus/preset-classic found in configuration".to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Consider using @docusaurus/preset-classic for standard Docusaurus features".to_string()),
            });
        }
        
        // Check for SEO optimization
        if !content.contains("tagline:") {
            findings.push(Finding {
                category: "SEO".to_string(),
                severity: Severity::Low,
                message: "Missing 'tagline' field for SEO optimization".to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Add a descriptive 'tagline' to improve SEO".to_string()),
            });
        }
        
        // Check for proper plugins configuration
        if !content.contains("plugins:") {
            findings.push(Finding {
                category: "Configuration".to_string(),
                severity: Severity::Low,
                message: "No plugins configured in Docusaurus".to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Consider adding plugins for enhanced functionality".to_string()),
            });
        }
        
        findings
    }
    
    /// Analyze the sidebar configuration file
    fn analyze_sidebar_config(&self, content: &str, path: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();
        
        // Check if sidebar structure is defined
        if content.trim().is_empty() || content.contains("{}") {
            findings.push(Finding {
                category: "Sidebar".to_string(),
                severity: Severity::Medium,
                message: "Empty sidebar configuration".to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Define a sidebar structure to organize your documentation".to_string()),
            });
        }
        
        // Check for proper sidebar category structuring
        if !content.contains("items:") && !content.contains("type: 'category'") {
            findings.push(Finding {
                category: "Sidebar".to_string(),
                severity: Severity::Low,
                message: "No categorization in sidebar".to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Group related documentation using sidebar categories".to_string()),
            });
        }
        
        findings
    }
    
    /// Clone implementation for the agent
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            event_system: self.event_system.clone(),
        }
    }
}
