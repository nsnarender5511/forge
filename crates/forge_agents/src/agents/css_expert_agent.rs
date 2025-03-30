use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::events::{
    Event, EventError, EventSystem, DOCS_ANALYZE_CSS, DOCS_CSS_ANALYZED,
};
use crate::events::doc_sync_events::{
    AnalysisReport, AnalyzeCssPayload, CssAnalyzedPayload, Finding, CssOperation, 
    OperationType, Severity, DocumentationMap, Operation,
};
use crate::state::StateManager;
use crate::utils::{find_files, path_exists, read_file_to_string};

/// Errors that can occur during CSS Expert Agent operations
#[derive(Debug, Error)]
pub enum CssExpertError {
    #[error("State error: {0}")]
    StateError(String),

    #[error("Event system error: {0}")]
    EventSystemError(#[from] EventError),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("CSS analysis error: {0}")]
    CssAnalysisError(String),

    #[error("Path processing error: {0}")]
    PathError(String),
}

/// Structure to hold the results of CSS analysis
#[derive(Debug)]
struct CssAnalysisResult {
    report: AnalysisReport,
    css_operations: Vec<CssOperation>,
    doc_map: DocumentationMap,
}

/// CSS Expert Agent analyzes and enhances styling of documentation
pub struct CssExpertAgent {
    state_manager: StateManager,
    event_system: Arc<EventSystem>,
}

impl CssExpertAgent {
    /// Create a new CSS Expert Agent
    pub fn new(state_manager: StateManager, event_system: Arc<EventSystem>) -> Self {
        Self {
            state_manager,
            event_system,
        }
    }

    /// Initialize the agent and register event handlers
    pub fn initialize(&self) -> Result<(), CssExpertError> {
        info!("Initializing CSS Expert Agent");
        
        // Register event handler for CSS analysis
        let event_system = self.event_system.clone();
        let agent = self.clone();

        self.event_system.register_handler(DOCS_ANALYZE_CSS, Box::new(move |event| {
            debug!("Received DOCS_ANALYZE_CSS event");
            let result = agent.handle_analyze_css_event(&event);
            if let Err(e) = &result {
                error!("Error handling DOCS_ANALYZE_CSS event: {:?}", e);
            }
            match result {
                Ok(_) => Ok(()),
                Err(e) => Err(EventError::HandlerError(format!("{}", e))),
            }
        }))?;

        info!("CssExpertAgent initialized and registered for events");
        Ok(())
    }

    /// Handle the DOCS_ANALYZE_CSS event
    fn handle_analyze_css_event(&self, event: &Event) -> Result<(), CssExpertError> {
        // Parse the payload
        let payload: AnalyzeCssPayload = serde_json::from_value(event.payload().clone())
            .map_err(|e| CssExpertError::InvalidPayload(format!("Failed to parse payload: {}", e)))?;

        // Validate target path
        let target_path = PathBuf::from(&payload.target_path);
        if !path_exists(&target_path) {
            return Err(CssExpertError::InvalidPayload(format!(
                "Target path does not exist: {}",
                target_path.display()
            )));
        }

        // Use the doc_map from the payload or create a new one
        let doc_map = payload.doc_map.unwrap_or_else(|| DocumentationMap::default());

        // Perform CSS analysis
        info!("Analyzing CSS at path: {}", target_path.display());
        let analysis_result = self.analyze_css(&target_path, doc_map)?;

        // Construct and emit response
        let response_payload = CssAnalyzedPayload {
            doc_map: analysis_result.doc_map,
            css_operations: analysis_result.css_operations,
            analysis_report: analysis_result.report,
        };

        let response_event = Event::Custom {
            name: DOCS_CSS_ANALYZED.to_string(),
            payload: serde_json::to_value(response_payload).map_err(|e| {
                CssExpertError::EventSystemError(EventError::SerializationError(e.to_string()))
            })?,
        };

        info!("Emitting DOCS_CSS_ANALYZED event");
        self.event_system.emit(response_event)?;

        Ok(())
    }
    
    /// Analyze CSS files in the target directory
    fn analyze_css(
        &self,
        target_path: &Path,
        mut doc_map: DocumentationMap,
    ) -> Result<CssAnalysisResult, CssExpertError> {
        // Validate target path
        if !target_path.exists() || !target_path.is_dir() {
            return Err(CssExpertError::PathError(format!(
                "Target path is not a valid directory: {}",
                target_path.display()
            )));
        }

        let mut findings = Vec::new();
        let mut operations = Vec::new();
        
        // Find CSS files
        let css_files = self.find_css_files(target_path)?;
        
        if css_files.is_empty() {
            findings.push(Finding {
                category: "css_structure".to_string(),
                severity: Severity::Medium,
                message: "No CSS files found in the target directory".to_string(),
                file_path: Some(target_path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Add CSS files for styling the documentation".to_string()),
            });
        } else {
            info!("Found {} CSS files to analyze", css_files.len());
            
            // Process each CSS file
            for file_path in &css_files {
                let content = read_file_to_string(file_path)
                    .map_err(|e| CssExpertError::IoError(format!("Failed to read CSS file: {}", e)))?;
                
                // Analyze the CSS content
                let file_findings = self.analyze_css_content(file_path, &content);
                findings.extend(file_findings);
                
                // Check for potential improvements
                let operations_for_file = self.suggest_css_improvements(file_path, &content);
                operations.extend(operations_for_file);
            }
            
            // Check for theme consistency
            self.check_theme_consistency(&css_files, &mut findings, &mut operations)?;
            
            // Check for responsiveness
            self.check_responsiveness(&css_files, &mut findings, &mut operations)?;
        }
        
        // Update doc_map metadata with CSS information
        doc_map.metadata.insert("css_analyzed".to_string(), "true".to_string());
        doc_map.metadata.insert("css_structure".to_string(), format!("{{\"css_files_count\": {}, \"operations_count\": {}}}",
            css_files.len(), operations.len()));

        // Convert CssOperation vec to general Operation vec for AnalysisReport
        let generic_operations: Vec<Operation> = operations.iter().map(|op| {
            Operation {
                op_type: match op.operation_type.as_str() {
                    "create" => OperationType::Create,
                    "update" => OperationType::Update,
                    "delete" => OperationType::Delete,
                    _ => OperationType::Update
                },
                path: op.target_path.clone(),
                content: op.content.clone(),
                metadata: Some(op.metadata.iter().map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect()),
            }
        }).collect();

        let report = AnalysisReport {
            total_items: findings.len() + operations.len(),
            findings: findings.clone(),
            operations: generic_operations,
            summary: format!("Analyzed CSS: found {} files, {} findings, {} operations to perform",
                css_files.len(), findings.len(), operations.len()),
        };

        // Convert Operation vec to CssOperation vec
        let css_operations: Vec<CssOperation> = operations.iter().map(|op| {
            CssOperation {
                operation_type: op.operation_type.clone(),
                target_path: op.target_path.clone(),
                content: op.content.clone(),
                metadata: op.metadata.clone(),
            }
        }).collect();

        Ok(CssAnalysisResult {
            report,
            css_operations,
            doc_map,
        })
    }
    
    /// Find all CSS files in the target directory
    fn find_css_files(&self, target_path: &Path) -> Result<Vec<PathBuf>, CssExpertError> {
        // Check for CSS files directly
        let css_files = find_files(target_path, "**/*.css")
            .map_err(|e| CssExpertError::IoError(format!("Failed to find CSS files: {}", e)))?;
            
        // Also look for SCSS/SASS files if they might be used
        let scss_files = find_files(target_path, "**/*.scss")
            .map_err(|e| CssExpertError::IoError(format!("Failed to find SCSS files: {}", e)))?;
            
        let less_files = find_files(target_path, "**/*.less")
            .map_err(|e| CssExpertError::IoError(format!("Failed to find LESS files: {}", e)))?;
            
        // Combine all style files
        let mut style_files = Vec::new();
        style_files.extend(css_files);
        style_files.extend(scss_files);
        style_files.extend(less_files);
        
        Ok(style_files)
    }
    
    /// Scan a directory recursively for CSS files
    fn scan_directory_for_css(&self, dir_path: &Path) -> Result<Vec<PathBuf>, CssExpertError> {
        let mut css_files = Vec::new();
        
        for entry in fs::read_dir(dir_path)
            .map_err(|e| CssExpertError::IoError(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| CssExpertError::IoError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip node_modules and other common directories to ignore
                if self.should_skip_path(&path) {
                    continue;
                }
                
                // Scan subdirectories recursively
                let sub_css_files = self.scan_directory_for_css(&path)?;
                css_files.extend(sub_css_files);
            } else if self.is_css_file(&path) {
                css_files.push(path);
            }
        }
        
        Ok(css_files)
    }
    
    /// Check if a file is a CSS file
    fn is_css_file(&self, file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            return ext == "css" || ext == "scss" || ext == "sass" || ext == "less";
        }
        false
    }
    
    /// Check if a path should be skipped during scanning
    fn should_skip_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        path_str.contains("node_modules") || 
        path_str.contains(".git") || 
        path_str.contains("build") ||
        path_str.contains("dist")
    }
    
    /// Analyze CSS content for potential issues and improvements
    fn analyze_css_content(&self, file_path: &Path, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Check if CSS is minified (which would make it hard to analyze)
        if content.lines().count() <= 5 && content.len() > 1000 {
            findings.push(Finding {
                category: "css_quality".to_string(),
                severity: Severity::Low,
                message: format!("CSS file appears to be minified: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Consider using unminified CSS files during development for better maintainability".to_string()),
            });
            return findings;  // Skip further analysis for minified files
        }
        
        // Check for vendor prefixes
        if content.contains("-webkit-") || content.contains("-moz-") || content.contains("-ms-") {
            findings.push(Finding {
                category: "css_compatibility".to_string(),
                severity: Severity::Low,
                message: format!("CSS file contains vendor prefixes: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Consider using autoprefixer or a similar tool to automatically handle vendor prefixes".to_string()),
            });
        }
        
        // Check for !important usage
        let important_count = content.matches("!important").count();
        if important_count > 3 {
            findings.push(Finding {
                category: "css_quality".to_string(),
                severity: Severity::Medium,
                message: format!("Excessive use of !important ({} occurrences): {}", important_count, file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Avoid using !important as it breaks the natural cascading of CSS and makes maintenance harder".to_string()),
            });
        }
        
        // Check for CSS variables usage
        if !content.contains("var(--") {
            findings.push(Finding {
                category: "css_maintainability".to_string(),
                severity: Severity::Low,
                message: format!("No CSS custom properties (variables) found: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Consider using CSS variables for colors, spacing, and other repeated values to improve maintainability".to_string()),
            });
        }
        
        // Check for responsive design
        if !content.contains("@media") {
            findings.push(Finding {
                category: "css_responsiveness".to_string(),
                severity: Severity::Medium,
                message: format!("No media queries found for responsive design: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Add media queries to ensure your documentation is responsive on different screen sizes".to_string()),
            });
        }
        
        // Check for dark mode support
        if !content.contains("prefers-color-scheme") {
            findings.push(Finding {
                category: "css_accessibility".to_string(),
                severity: Severity::Low,
                message: format!("No dark mode support detected: {}", file_path.display()),
                file_path: Some(file_path_str),
                line_number: None,
                recommendation: Some("Consider adding support for dark mode using @media (prefers-color-scheme: dark)".to_string()),
            });
        }
        
        findings
    }
    
    /// Suggest CSS improvements
    fn suggest_css_improvements(&self, file_path: &Path, content: &str) -> Vec<CssOperation> {
        let mut operations = Vec::new();
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Check if we should add CSS variable suggestions
        if !content.contains("var(--") {
            operations.push(CssOperation {
                operation_type: "update".to_string(),
                target_path: file_path_str.clone(),
                content: Some("/* CSS Variables Declaration */\n:root {\n  --primary-color: #3578e5;\n  --secondary-color: #303846;\n  --text-color: #1c1e21;\n  --background-color: #ffffff;\n  --link-color: #1a73e8;\n  --font-size-base: 16px;\n  --font-size-small: 14px;\n  --spacing-unit: 8px;\n  \n  /* Dark mode variables */\n  @media (prefers-color-scheme: dark) {\n    --text-color: #f5f6f7;\n    --background-color: #1c1e21;\n    --link-color: #4dabf7;\n  }\n}\n\n/* Use these variables in your existing CSS */\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Add CSS variables declaration".to_string());
                    metadata
                },
            });
        }
        
        // Check if we should add responsive design suggestions
        if !content.contains("@media") {
            operations.push(CssOperation {
                operation_type: "update".to_string(),
                target_path: file_path_str.clone(),
                content: Some("/* Responsive Design */\n@media (max-width: 996px) {\n  /* Tablet styles */\n  .container {\n    padding: 0 var(--spacing-unit);\n  }\n}\n\n@media (max-width: 768px) {\n  /* Mobile styles */\n  .container {\n    padding: 0 calc(var(--spacing-unit) / 2);\n  }\n  \n  .sidebar {\n    display: none;\n  }\n}\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Add responsive design media queries".to_string());
                    metadata
                },
            });
        }
        
        // Check if we should add dark mode support
        if !content.contains("prefers-color-scheme") {
            operations.push(CssOperation {
                operation_type: "update".to_string(),
                target_path: file_path_str,
                content: Some("/* Dark Mode Support */\n@media (prefers-color-scheme: dark) {\n  body {\n    background-color: var(--background-color);\n    color: var(--text-color);\n  }\n  \n  a {\n    color: var(--link-color);\n  }\n  \n  pre, code {\n    background-color: #2d333b;\n  }\n}\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Add dark mode support".to_string());
                    metadata
                },
            });
        }
        
        operations
    }
    
    /// Check for theme consistency across CSS files
    fn check_theme_consistency(
        &self,
        css_files: &[PathBuf],
        findings: &mut Vec<Finding>,
        operations: &mut Vec<CssOperation>,
    ) -> Result<(), CssExpertError> {
        let mut color_values = HashMap::new();
        let mut font_values = HashMap::new();
        
        // Extract color and font values from all CSS files
        for file_path in css_files {
            let content = read_file_to_string(file_path)
                .map_err(|e| CssExpertError::IoError(format!("Failed to read CSS file: {}", e)))?;
            
            // Simple regex for color values (not comprehensive)
            let color_regex = regex::Regex::new(r"#[0-9a-fA-F]{3,6}|rgba?\([^)]+\)").unwrap();
            for color_capture in color_regex.captures_iter(&content) {
                let color = color_capture.get(0).unwrap().as_str();
                *color_values.entry(color.to_string()).or_insert(0) += 1;
            }
            
            // Simple regex for font family values
            let font_regex = regex::Regex::new(r"font-family:\s*([^;]+);").unwrap();
            for font_capture in font_regex.captures_iter(&content) {
                if let Some(font_match) = font_capture.get(1) {
                    let font = font_match.as_str().trim();
                    *font_values.entry(font.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        // Check for too many color variations
        if color_values.len() > 10 {
            findings.push(Finding {
                category: "css_consistency".to_string(),
                severity: Severity::Medium,
                message: format!("Too many different color values: {} unique colors", color_values.len()),
                file_path: None,
                line_number: None,
                recommendation: Some("Consider using a consistent color palette with CSS variables".to_string()),
            });
            
            // Create a theme file suggestion
            operations.push(CssOperation {
                operation_type: "create".to_string(),
                target_path: "theme.css".to_string(),
                content: Some("/* Theme variables for consistency */\n:root {\n  /* Primary Colors */\n  --primary-100: #ebf5ff;\n  --primary-200: #c3dafe;\n  --primary-300: #a3bffa;\n  --primary-400: #7f9cf5;\n  --primary-500: #667eea;\n  --primary-600: #5a67d8;\n  --primary-700: #4c51bf;\n  --primary-800: #434190;\n  --primary-900: #3c366b;\n  \n  /* Neutral Colors */\n  --neutral-100: #f7fafc;\n  --neutral-200: #edf2f7;\n  --neutral-300: #e2e8f0;\n  --neutral-400: #cbd5e0;\n  --neutral-500: #a0aec0;\n  --neutral-600: #718096;\n  --neutral-700: #4a5568;\n  --neutral-800: #2d3748;\n  --neutral-900: #1a202c;\n  \n  /* Typography */\n  --font-family-base: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;\n  --font-family-heading: 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;\n  --font-family-mono: SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;\n}\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Create a theme file with consistent color variables".to_string());
                    metadata
                },
            });
        }
        
        // Check for too many font variations
        if font_values.len() > 3 {
            findings.push(Finding {
                category: "css_consistency".to_string(),
                severity: Severity::Medium,
                message: format!("Too many different font families: {} unique font declarations", font_values.len()),
                file_path: None,
                line_number: None,
                recommendation: Some("Consider using a consistent typography system with 2-3 font families".to_string()),
            });
        }
        
        Ok(())
    }
    
    /// Check for responsive design across CSS files
    fn check_responsiveness(
        &self,
        css_files: &[PathBuf],
        findings: &mut Vec<Finding>,
        operations: &mut Vec<CssOperation>,
    ) -> Result<(), CssExpertError> {
        let mut has_responsive_design = false;
        
        for file_path in css_files {
            let content = read_file_to_string(file_path)
                .map_err(|e| CssExpertError::IoError(format!("Failed to read CSS file: {}", e)))?;
            
            if content.contains("@media") {
                has_responsive_design = true;
                break;
            }
        }
        
        if !has_responsive_design {
            findings.push(Finding {
                category: "css_responsiveness".to_string(),
                severity: Severity::High,
                message: "No responsive design media queries found in any CSS file".to_string(),
                file_path: None,
                line_number: None,
                recommendation: Some("Add responsive media queries to ensure documentation works well on all devices".to_string()),
            });
            
            // Create a responsive utilities file suggestion
            operations.push(CssOperation {
                operation_type: "create".to_string(),
                target_path: "responsive.css".to_string(),
                content: Some("/* Responsive Design Utilities */\n\n/* Base styles for all devices */\n.container {\n  width: 100%;\n  max-width: 1200px;\n  margin: 0 auto;\n  padding: 0 16px;\n}\n\n/* Tablet devices */\n@media (max-width: 996px) {\n  .container {\n    max-width: 768px;\n  }\n  \n  .docs-main {\n    padding: 1rem;\n  }\n  \n  .docs-sidebar {\n    width: 240px;\n  }\n}\n\n/* Mobile devices */\n@media (max-width: 768px) {\n  .container {\n    max-width: 100%;\n  }\n  \n  .docs-wrapper {\n    flex-direction: column;\n  }\n  \n  .docs-sidebar {\n    width: 100%;\n    margin-bottom: 1rem;\n  }\n  \n  .navbar__items {\n    flex-direction: column;\n  }\n}\n\n/* Small mobile devices */\n@media (max-width: 480px) {\n  h1 {\n    font-size: 1.75rem;\n  }\n  \n  h2 {\n    font-size: 1.5rem;\n  }\n  \n  .docs-main {\n    padding: 0.5rem;\n  }\n}\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Create responsive design utilities".to_string());
                    metadata
                },
            });
        }
        
        Ok(())
    }
    
    /// Get the relative path of a file from the base directory
    fn get_relative_path(&self, file_path: &Path, base_path: &Path) -> Result<PathBuf, CssExpertError> {
        match file_path.strip_prefix(base_path) {
            Ok(rel_path) => Ok(rel_path.to_path_buf()),
            Err(_) => Err(CssExpertError::PathError(format!(
                "Failed to get relative path from base directory: {}", file_path.display()
            ))),
        }
    }
    
    /// Calculate hash for content
    fn calculate_hash(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        
        format!("{:x}", hash)
    }
    
    /// Count CSS selectors in content
    fn count_css_selectors(&self, content: &str) -> usize {
        let mut count = 0;
        let mut in_comment = false;
        let mut _in_rule = false;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() { continue; }
            
            // Handle comments
            if line.contains("/*") { in_comment = true; }
            if line.contains("*/") { in_comment = false; continue; }
            if in_comment { continue; }
            
            // Count selector declarations
            if line.contains("{") { 
                _in_rule = true; 
                count += 1;
            }
            if line.contains("}") { _in_rule = false; }
        }
        
        count
    }
    
    /// Count CSS properties in content
    fn count_css_properties(&self, content: &str) -> usize {
        let mut count = 0;
        let mut in_comment = false;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() { continue; }
            
            // Handle comments
            if line.contains("/*") { in_comment = true; }
            if line.contains("*/") { in_comment = false; continue; }
            if in_comment { continue; }
            
            // Count property declarations
            if line.contains(":") && !line.contains("{") { 
                count += 1;
            }
        }
        
        count
    }
    
    /// Count media queries in content
    fn count_media_queries(&self, content: &str) -> usize {
        content.matches("@media").count()
    }
}

impl Clone for CssExpertAgent {
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            event_system: Arc::clone(&self.event_system),
        }
    }
}
