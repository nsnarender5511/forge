use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::events::{
    EventError, EventSystem, DOCS_ANALYZE_UI, DOCS_UI_ANALYZED,
};
use crate::events::Event as CrateEvent; // Rename to prevent conflict
use crate::events::doc_sync_events::{
    AnalysisReport, AnalyzeUiPayload, UiAnalyzedPayload, Finding, UiOperation, 
    OperationType, Severity, DocumentationMap, Operation,
};
use crate::state::StateManager;
use crate::utils::{find_files, path_exists, read_file_to_string};

/// Errors that can occur during UI Design Expert Agent operations
#[derive(Debug, Error)]
pub enum UiDesignExpertError {
    #[error("State error: {0}")]
    StateError(String),

    #[error("Event system error: {0}")]
    EventSystemError(#[from] EventError),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("UI analysis error: {0}")]
    UiAnalysisError(String),

    #[error("Path processing error: {0}")]
    PathError(String),
}

/// Structure to hold the results of UI analysis
#[derive(Debug)]
struct UiAnalysisResult {
    report: AnalysisReport,
    ui_operations: Vec<UiOperation>,
    doc_map: DocumentationMap,
}

/// UI Design Expert Agent analyzes and enhances user interface components
pub struct UiDesignExpertAgent {
    state_manager: StateManager,
    event_system: Arc<EventSystem>,
}

impl UiDesignExpertAgent {
    /// Create a new UI Design Expert Agent
    pub fn new(state_manager: StateManager, event_system: Arc<EventSystem>) -> Self {
        Self {
            state_manager,
            event_system,
        }
    }

    /// Initialize the agent and register event handlers
    pub fn initialize(&self) -> Result<(), UiDesignExpertError> {
        info!("Initializing UI Design Expert Agent");
        
        // Register event handler for UI analysis
        let event_system = self.event_system.clone();
        let agent = self.clone();

        self.event_system.register_handler(DOCS_ANALYZE_UI, Box::new(move |event| {
            debug!("Received DOCS_ANALYZE_UI event");
            let result = agent.handle_analyze_ui_event(&event);
            if let Err(e) = &result {
                error!("Error handling DOCS_ANALYZE_UI event: {:?}", e);
            }
            match result {
                Ok(_) => Ok(()),
                Err(e) => Err(EventError::HandlerError(format!("{}", e))),
            }
        }))?;

        info!("UiDesignExpertAgent initialized and registered for events");
        Ok(())
    }

    /// Handle the DOCS_ANALYZE_UI event
    fn handle_analyze_ui_event(&self, event: &CrateEvent) -> Result<(), UiDesignExpertError> {
        // Parse the payload
        let payload: AnalyzeUiPayload = serde_json::from_value(event.payload().clone())
            .map_err(|e| UiDesignExpertError::InvalidPayload(format!("Failed to parse payload: {}", e)))?;

        // Validate target path
        let target_path = PathBuf::from(&payload.target_path);
        if !path_exists(&target_path) {
            return Err(UiDesignExpertError::InvalidPayload(format!(
                "Target path does not exist: {}",
                target_path.display()
            )));
        }

        // Use the doc_map from the payload or create a new one
        let doc_map = payload.doc_map.unwrap_or_else(|| DocumentationMap::default());

        // Perform UI analysis
        info!("Analyzing UI components at path: {}", target_path.display());
        let analysis_result = self.analyze_ui_components(&target_path, doc_map)?;

        // Construct and emit response
        let response_payload = UiAnalyzedPayload {
            doc_map: analysis_result.doc_map,
            ui_operations: analysis_result.ui_operations,
            analysis_report: analysis_result.report,
        };

        let response_event = CrateEvent::Custom {
            name: DOCS_UI_ANALYZED.to_string(),
            payload: serde_json::to_value(response_payload).map_err(|e| {
                UiDesignExpertError::EventSystemError(EventError::SerializationError(e.to_string()))
            })?,
        };

        info!("Emitting DOCS_UI_ANALYZED event");
        self.event_system.emit(response_event)?;

        Ok(())
    }
    
    /// Analyze UI components in the target directory
    fn analyze_ui_components(
        &self,
        target_path: &Path,
        mut doc_map: DocumentationMap,
    ) -> Result<UiAnalysisResult, UiDesignExpertError> {
        // Validate target path
        if !target_path.exists() || !target_path.is_dir() {
            return Err(UiDesignExpertError::PathError(format!(
                "Target path is not a valid directory: {}",
                target_path.display()
            )));
        }

        let mut findings = Vec::new();
        let mut operations = Vec::new();
        
        // Find React/JSX component files
        let component_files = self.find_component_files(target_path)?;
        
        if component_files.is_empty() {
            findings.push(Finding {
                category: "ui_structure".to_string(),
                severity: Severity::Medium,
                message: "No React component files found in the target directory".to_string(),
                file_path: Some(target_path.to_string_lossy().to_string()),
                line_number: None,
                recommendation: Some("Add React component files for enhancing the documentation UI".to_string()),
            });
        } else {
            info!("Found {} component files to analyze", component_files.len());
            
            // Process each component file
            for file_path in &component_files {
                let content = read_file_to_string(file_path)
                    .map_err(|e| UiDesignExpertError::IoError(format!("Failed to read component file: {}", e)))?;
                
                // Analyze the component content
                let file_findings = self.analyze_component_content(file_path, &content);
                findings.extend(file_findings);
                
                // Check for potential improvements
                let operations_for_file = self.suggest_component_improvements(file_path, &content);
                operations.extend(operations_for_file);
            }
            
            // Check for common UI components
            self.check_common_ui_components(&component_files, &mut findings, &mut operations)?;
            
            // Check for accessibility
            self.check_accessibility(&component_files, &mut findings, &mut operations)?;
        }
        
        // Update doc_map metadata with UI information
        doc_map.metadata.insert("ui_analyzed".to_string(), "true".to_string());
        doc_map.metadata.insert("ui_structure".to_string(), format!("{{\"component_files_count\": {}, \"operations_count\": {}}}",
            component_files.len(), operations.len()));

        // Convert UiOperation vec to general Operation vec for AnalysisReport
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
            summary: format!("Analyzed UI components: found {} files, {} findings, {} operations to perform",
                component_files.len(), findings.len(), operations.len()),
        };

        // Convert Operation vec to UiOperation vec
        let ui_operations = operations;

        Ok(UiAnalysisResult {
            report,
            ui_operations,
            doc_map,
        })
    }
    
    /// Find all component files in the target directory
    fn find_component_files(&self, target_path: &Path) -> Result<Vec<PathBuf>, UiDesignExpertError> {
        // Check for React component files
        let jsx_files = find_files(target_path, "**/*.jsx")
            .map_err(|e| UiDesignExpertError::IoError(format!("Failed to find JSX files: {}", e)))?;
            
        let tsx_files = find_files(target_path, "**/*.tsx")
            .map_err(|e| UiDesignExpertError::IoError(format!("Failed to find TSX files: {}", e)))?;
            
        let js_files = find_files(target_path, "**/*.js")
            .map_err(|e| UiDesignExpertError::IoError(format!("Failed to find JS files: {}", e)))?;
            
        // Combine all component files
        let mut component_files = Vec::new();
        component_files.extend(jsx_files);
        component_files.extend(tsx_files);
        component_files.extend(js_files);
        
        Ok(component_files)
    }
    
    /// Scan a directory recursively for component files
    fn scan_directory_for_components(&self, dir_path: &Path) -> Result<Vec<PathBuf>, UiDesignExpertError> {
        let mut component_files = Vec::new();
        
        for entry in fs::read_dir(dir_path)
            .map_err(|e| UiDesignExpertError::IoError(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| UiDesignExpertError::IoError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip node_modules and other common directories to ignore
                if self.should_skip_path(&path) {
                    continue;
                }
                
                // Scan subdirectories recursively
                let sub_component_files = self.scan_directory_for_components(&path)?;
                component_files.extend(sub_component_files);
            } else if self.is_component_file(&path) {
                component_files.push(path);
            }
        }
        
        Ok(component_files)
    }
    
    /// Check if a file is a component file
    fn is_component_file(&self, file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            return ext == "jsx" || ext == "tsx" || ext == "js";
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
    
    /// Analyze component content for potential issues and improvements
    fn analyze_component_content(&self, file_path: &Path, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Check for React imports
        if !content.contains("import React") && !content.contains("from 'react'") && !content.contains("from \"react\"") {
            findings.push(Finding {
                category: "component_quality".to_string(),
                severity: Severity::Low,
                message: format!("Component may be missing React import: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Ensure React is properly imported in component files".to_string()),
            });
        }
        
        // Check for accessibility attributes
        if content.contains("<img") && !content.contains("alt=") {
            findings.push(Finding {
                category: "accessibility".to_string(),
                severity: Severity::Medium,
                message: format!("Image elements without alt attributes found: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Add alt attributes to all img elements for better accessibility".to_string()),
            });
        }
        
        // Check for aria attributes in interactive elements
        if (content.contains("<button") || content.contains("<a ")) && 
           !content.contains("aria-") && !content.contains("role=") {
            findings.push(Finding {
                category: "accessibility".to_string(),
                severity: Severity::Low,
                message: format!("Interactive elements may be missing aria attributes: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Consider adding appropriate aria attributes to interactive elements".to_string()),
            });
        }
        
        // Check for props validation
        if !content.contains("PropTypes") && !content.contains("type Props") && !content.contains("interface Props") {
            findings.push(Finding {
                category: "component_quality".to_string(),
                severity: Severity::Low,
                message: format!("Component may be missing props validation: {}", file_path.display()),
                file_path: Some(file_path_str.clone()),
                line_number: None,
                recommendation: Some("Add PropTypes or TypeScript types for component props".to_string()),
            });
        }
        
        // Check for inline styles (which might indicate styling issues)
        if content.contains("style={") || content.contains("style=\"") {
            findings.push(Finding {
                category: "component_quality".to_string(),
                severity: Severity::Low,
                message: format!("Component uses inline styles which may affect maintainability: {}", file_path.display()),
                file_path: Some(file_path_str),
                line_number: None,
                recommendation: Some("Consider using CSS modules or styled components instead of inline styles".to_string()),
            });
        }
        
        findings
    }
    
    /// Suggest component improvements
    fn suggest_component_improvements(&self, file_path: &Path, content: &str) -> Vec<UiOperation> {
        let mut operations = Vec::new();
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Suggest accessibility improvements if needed
        if content.contains("<img") && !content.contains("alt=") {
            operations.push(UiOperation {
                operation_type: "update".to_string(),
                target_path: file_path_str.clone(),
                content: Some("// Updated image elements with alt attributes for accessibility\n// Example:\n// Before: <img src=\"example.png\" />\n// After: <img src=\"example.png\" alt=\"Description of the image\" />\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Add alt attributes to images".to_string());
                    metadata
                },
            });
        }
        
        // Suggest props validation if missing
        if !content.contains("PropTypes") && !content.contains("type Props") && !content.contains("interface Props") {
            // Check if it's a TypeScript file
            if file_path_str.ends_with(".tsx") {
                operations.push(UiOperation {
                    operation_type: "update".to_string(),
                    target_path: file_path_str.clone(),
                    content: Some("// Add TypeScript interface for props\ninterface Props {\n  // Define your props here\n  title?: string;\n  description?: string;\n  children?: React.ReactNode;\n}\n\n// Use the interface in your component\nconst YourComponent: React.FC<Props> = ({ title, description, children }) => {\n  // Component implementation\n};\n".to_string()),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("type".to_string(), "suggestion".to_string());
                        metadata.insert("description".to_string(), "Add TypeScript props interface".to_string());
                        metadata
                    },
                });
            } else {
                operations.push(UiOperation {
                    operation_type: "update".to_string(),
                    target_path: file_path_str.clone(),
                    content: Some("// Add PropTypes for validation\nimport PropTypes from 'prop-types';\n\n// At the bottom of your file:\nYourComponent.propTypes = {\n  title: PropTypes.string,\n  description: PropTypes.string,\n  children: PropTypes.node,\n};\n".to_string()),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("type".to_string(), "suggestion".to_string());
                        metadata.insert("description".to_string(), "Add PropTypes validation".to_string());
                        metadata
                    },
                });
            }
        }
        
        // If using inline styles, suggest CSS modules or styled-components
        if content.contains("style={") || content.contains("style=\"") {
            operations.push(UiOperation {
                operation_type: "update".to_string(),
                target_path: file_path_str,
                content: Some("// Replace inline styles with CSS modules\n// Create a file named YourComponent.module.css with:\n/*\n.container {\n  display: flex;\n  flex-direction: column;\n  padding: 16px;\n}\n\n.title {\n  font-size: 1.5rem;\n  margin-bottom: 8px;\n}\n*/\n\n// Then import and use in your component:\n// import styles from './YourComponent.module.css';\n// <div className={styles.container}>\n//   <h2 className={styles.title}>Title</h2>\n// </div>\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Use CSS modules instead of inline styles".to_string());
                    metadata
                },
            });
        }
        
        operations
    }
    
    /// Check for common UI components
    fn check_common_ui_components(
        &self,
        component_files: &[PathBuf],
        findings: &mut Vec<Finding>,
        operations: &mut Vec<UiOperation>,
    ) -> Result<(), UiDesignExpertError> {
        // Essential components for documentation
        let essential_components = [
            "CodeBlock", "AdmonitionBlock", "Tabs", "TabItem", "Pagination"
        ];
        
        let mut found_components = HashMap::new();
        
        // Scan components
        for file_path in component_files {
            let filename = file_path.file_stem().unwrap_or_default().to_string_lossy();
            
            for component in &essential_components {
                if filename.contains(component) {
                    found_components.insert(*component, file_path.clone());
                    break;
                }
            }
            
            // Read content to check for component definitions
            if found_components.len() < essential_components.len() {
                let content = read_file_to_string(file_path)
                    .map_err(|e| UiDesignExpertError::IoError(format!("Failed to read component file: {}", e)))?;
                
                for component in &essential_components {
                    if !found_components.contains_key(component) && 
                       (content.contains(&format!("function {}", component)) || 
                        content.contains(&format!("class {} extends", component)) ||
                        content.contains(&format!("const {} =", component))) {
                        found_components.insert(*component, file_path.clone());
                    }
                }
            }
        }
        
        // Check for missing components
        for component in &essential_components {
            if !found_components.contains_key(component) {
                findings.push(Finding {
                    category: "ui_components".to_string(),
                    severity: Severity::Low,
                    message: format!("Missing common documentation component: {}", component),
                    file_path: None,
                    line_number: None,
                    recommendation: Some(format!("Consider adding a {} component to enhance documentation", component)),
                });
                
                // Suggest component implementation
                let component_template = match *component {
                    "CodeBlock" => self.get_code_block_template(),
                    "AdmonitionBlock" => self.get_admonition_template(),
                    "Tabs" => self.get_tabs_template(),
                    "TabItem" => self.get_tab_item_template(),
                    "Pagination" => self.get_pagination_template(),
                    _ => continue, // Skip for unknown components
                };
                
                operations.push(UiOperation {
                    operation_type: "create".to_string(),
                    target_path: format!("src/components/{}.jsx", component),
                    content: Some(component_template),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("type".to_string(), "suggestion".to_string());
                        metadata.insert("description".to_string(), format!("Create {} component", component));
                        metadata
                    },
                });
            }
        }
        
        Ok(())
    }
    
    /// Check for accessibility issues in components
    fn check_accessibility(
        &self,
        component_files: &[PathBuf],
        findings: &mut Vec<Finding>,
        operations: &mut Vec<UiOperation>,
    ) -> Result<(), UiDesignExpertError> {
        let mut has_accessibility_issues = false;
        
        for file_path in component_files {
            let content = read_file_to_string(file_path)
                .map_err(|e| UiDesignExpertError::IoError(format!("Failed to read component file: {}", e)))?;
            
            // Check for common accessibility issues
            if (content.contains("<img") && !content.contains("alt=")) ||
               (content.contains("<button") && !content.contains("aria-")) ||
               (content.contains("<div") && content.contains("onClick") && !content.contains("role=")) {
                has_accessibility_issues = true;
                findings.push(Finding {
                    category: "accessibility".to_string(),
                    severity: Severity::Medium,
                    message: format!("Accessibility issues found in: {}", file_path.display()),
                    file_path: Some(file_path.to_string_lossy().to_string()),
                    line_number: None,
                    recommendation: Some("Address accessibility issues to ensure documentation is usable by everyone".to_string()),
                });
            }
        }
        
        if has_accessibility_issues {
            // Suggest an accessibility guide
            operations.push(UiOperation {
                operation_type: "create".to_string(),
                target_path: "docs/developer/accessibility-guidelines.md".to_string(),
                content: Some("---\ntitle: Accessibility Guidelines\nsidebar_position: 5\n---\n\n# Accessibility Guidelines\n\nThis document outlines the accessibility standards for our documentation components.\n\n## General Guidelines\n\n- Ensure all images have appropriate `alt` text\n- Use semantic HTML elements (`<button>`, `<a>`, etc.) for their intended purpose\n- Add appropriate ARIA attributes when needed\n- Ensure proper color contrast (minimum 4.5:1 for normal text)\n- Make sure all functionality is accessible via keyboard\n- Test with screen readers\n\n## Component-specific Guidelines\n\n### Images\n\n```jsx\n// Good\n<img src=\"example.png\" alt=\"Description of the image\" />\n\n// Bad\n<img src=\"example.png\" />\n```\n\n### Interactive Elements\n\n```jsx\n// Good\n<button aria-label=\"Close dialog\" onClick={closeDialog}>\n  <span className=\"icon-close\" />\n</button>\n\n// Bad\n<div onClick={closeDialog}>\n  <span className=\"icon-close\" />\n</div>\n```\n\n### Forms\n\n```jsx\n// Good\n<label htmlFor=\"name-input\">Name</label>\n<input id=\"name-input\" type=\"text\" />\n\n// Bad\n<input type=\"text\" placeholder=\"Name\" />\n```\n".to_string()),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "suggestion".to_string());
                    metadata.insert("description".to_string(), "Create accessibility guidelines document".to_string());
                    metadata
                },
            });
        }
        
        Ok(())
    }
    
    /// Get template for CodeBlock component
    fn get_code_block_template(&self) -> String {
        r#"import React, { useState } from 'react';
import PropTypes from 'prop-types';
import './CodeBlock.css';

/**
 * CodeBlock component for displaying syntax-highlighted code
 */
const CodeBlock = ({ code, language, title, showLineNumbers = true }) => {
  const [isCopied, setIsCopied] = useState(false);
  
  const copyToClipboard = () => {
    navigator.clipboard.writeText(code).then(() => {
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    });
  };
  
  return (
    <div className="code-block">
      {title && (
        <div className="code-block-title">
          <span>{title}</span>
        </div>
      )}
      <div className="code-block-header">
        <span className="code-block-language">{language}</span>
        <button 
          onClick={copyToClipboard} 
          className="code-block-copy-button"
          aria-label={isCopied ? "Copied" : "Copy code to clipboard"}
        >
          {isCopied ? "Copied!" : "Copy"}
        </button>
      </div>
      <pre className={`language-${language}`}>
        <code>
          {showLineNumbers ? code.split('\n').map((line, i) => (
            <div key={i} className="code-line">
              <span className="code-line-number">{i + 1}</span>
              <span className="code-line-content">{line}</span>
            </div>
          )) : code}
        </code>
      </pre>
    </div>
  );
};

CodeBlock.propTypes = {
  code: PropTypes.string.isRequired,
  language: PropTypes.string,
  title: PropTypes.string,
  showLineNumbers: PropTypes.bool
};

export default CodeBlock;"#.to_string()
    }
    
    /// Get template for AdmonitionBlock component
    fn get_admonition_template(&self) -> String {
        r#"import React from 'react';
import PropTypes from 'prop-types';
import './AdmonitionBlock.css';

/**
 * AdmonitionBlock component for displaying warnings, notes, tips, etc.
 */
const AdmonitionBlock = ({ type = 'note', title, children }) => {
  const defaultTitles = {
    note: 'Note',
    info: 'Info',
    tip: 'Tip',
    warning: 'Warning',
    danger: 'Danger',
    caution: 'Caution'
  };
  
  const finalTitle = title || defaultTitles[type] || 'Note';
  
  return (
    <div className={`admonition admonition-${type}`} role="alert">
      <div className="admonition-heading">
        <h5>{finalTitle}</h5>
      </div>
      <div className="admonition-content">
        {children}
      </div>
    </div>
  );
};

AdmonitionBlock.propTypes = {
  type: PropTypes.oneOf(['note', 'info', 'tip', 'warning', 'danger', 'caution']),
  title: PropTypes.string,
  children: PropTypes.node.isRequired
};

export default AdmonitionBlock;"#.to_string()
    }
    
    /// Get template for Tabs component
    fn get_tabs_template(&self) -> String {
        r#"import React, { useState } from 'react';
import PropTypes from 'prop-types';
import './Tabs.css';

/**
 * Tabs component for organizing content in tabs
 */
const Tabs = ({ children, defaultValue }) => {
  // Find the default tab value or use the first tab
  const getDefaultValue = () => {
    if (defaultValue) return defaultValue;
    const firstTab = React.Children.toArray(children)[0];
    return firstTab?.props?.value || '';
  };
  
  const [activeTab, setActiveTab] = useState(getDefaultValue());
  
  // Extract tab labels and values from children
  const tabs = React.Children.map(children, (child) => {
    if (!React.isValidElement(child) || child.type.name !== 'TabItem') {
      console.warn('Tabs only accepts TabItem components as children');
      return null;
    }
    
    return {
      label: child.props.label,
      value: child.props.value
    };
  }).filter(Boolean);
  
  return (
    <div className="tabs-container">
      <div role="tablist" aria-orientation="horizontal" className="tabs">
        {tabs.map((tab) => (
          <button
            key={tab.value}
            role="tab"
            aria-selected={activeTab === tab.value}
            onClick={() => setActiveTab(tab.value)}
            className={`tab ${activeTab === tab.value ? 'active' : ''}`}
            id={`tab-${tab.value}`}
            aria-controls={`panel-${tab.value}`}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="tab-content">
        {React.Children.map(children, (child) => {
          if (!React.isValidElement(child) || child.type.name !== 'TabItem') return null;
          
          return React.cloneElement(child, {
            isActive: child.props.value === activeTab
          });
        })}
      </div>
    </div>
  );
};

Tabs.propTypes = {
  children: PropTypes.node.isRequired,
  defaultValue: PropTypes.string
};

export default Tabs;"#.to_string()
    }
    
    /// Get template for TabItem component
    fn get_tab_item_template(&self) -> String {
        r#"import React from 'react';
import PropTypes from 'prop-types';
import './TabItem.css';

/**
 * TabItem component to be used with Tabs component
 */
const TabItem = ({ children, label, value, isActive = false }) => {
  return (
    <div 
      role="tabpanel"
      id={`panel-${value}`}
      aria-labelledby={`tab-${value}`}
      className={`tab-panel ${isActive ? 'active' : 'hidden'}`}
    >
      {children}
    </div>
  );
};

TabItem.propTypes = {
  children: PropTypes.node.isRequired,
  label: PropTypes.string.isRequired,
  value: PropTypes.string.isRequired,
  isActive: PropTypes.bool
};

export default TabItem;"#.to_string()
    }
    
    /// Get template for Pagination component
    fn get_pagination_template(&self) -> String {
        r#"import React from 'react';
import PropTypes from 'prop-types';
import './Pagination.css';

/**
 * Pagination component for navigating between documentation pages
 */
const Pagination = ({ previousPage, nextPage }) => {
  return (
    <nav className="pagination-nav" aria-label="Documentation pages">
      <div className="pagination-nav-item">
        {previousPage && (
          <a 
            className="pagination-nav-link previous" 
            href={previousPage.url}
            aria-label={`Previous: ${previousPage.title}`}
          >
            <div className="pagination-nav-label">
              <span>«</span> Previous
            </div>
            <div className="pagination-nav-title">{previousPage.title}</div>
          </a>
        )}
      </div>
      <div className="pagination-nav-item">
        {nextPage && (
          <a 
            className="pagination-nav-link next" 
            href={nextPage.url}
            aria-label={`Next: ${nextPage.title}`}
          >
            <div className="pagination-nav-label">
              Next <span>»</span>
            </div>
            <div className="pagination-nav-title">{nextPage.title}</div>
          </a>
        )}
      </div>
    </nav>
  );
};

Pagination.propTypes = {
  previousPage: PropTypes.shape({
    title: PropTypes.string.isRequired,
    url: PropTypes.string.isRequired
  }),
  nextPage: PropTypes.shape({
    title: PropTypes.string.isRequired,
    url: PropTypes.string.isRequired
  })
};

export default Pagination;"#.to_string()
    }
}

impl Clone for UiDesignExpertAgent {
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            event_system: Arc::clone(&self.event_system),
        }
    }
} 