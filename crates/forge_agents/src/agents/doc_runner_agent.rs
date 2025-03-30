use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use log::{debug, error, info, warn};

use crate::events::{Event, EventHandler, EventSystem};
use crate::events::doc_sync_events::{
    self, DocSyncEvent, DocSyncPayload, DocumentationMap, SyncOperation,
    ExecutePayload, ExecutionCompletePayload, ExecutionParams, ExecutionResults,
    ExecutionLogEntry, ContentOperation, StructureOperation, UiOperation, CssOperation
};
use crate::state::doc_sync_state::{StateManager, DocSyncState, StateError};

/// Errors that can occur during Doc Runner Agent operations
#[derive(Debug, Error)]
pub enum DocRunnerError {
    #[error("State error: {0}")]
    StateError(#[from] StateError),
    #[error("Event system error: {0}")]
    EventSystemError(String),
    #[error("Invalid event payload: {0}")]
    InvalidPayload(String),
    #[error("File operation error: {0}")]
    FileOperationError(String),
    #[error("File validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

/// Represents the result of a file operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub operation_type: String,
    pub status: String, // "success", "failure", "skipped"
    pub target_path: String,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}

/// Doc Runner Agent executes file operations for documentation synchronization
pub struct DocRunnerAgent {
    state_manager: Arc<StateManager>,
    event_system: Arc<EventSystem>,
}

impl DocRunnerAgent {
    /// Create a new DocRunnerAgent with the specified state manager and event system
    pub fn new(state_manager: Arc<StateManager>, event_system: Arc<EventSystem>) -> Self {
        Self {
            state_manager,
            event_system,
        }
    }
    
    /// Initialize the agent by registering event handlers
    pub fn initialize(&self) -> Result<(), DocRunnerError> {
        self.register_event_handlers()?;
        Ok(())
    }
    
    /// Register event handlers for the agent
    fn register_event_handlers(&self) -> Result<(), DocRunnerError> {
        let agent_clone = Arc::new(self.clone());
        
        // Register handler for execute event
        self.event_system
            .register_handler(
                doc_sync_events::events::DOCS_EXECUTE,
                Box::new(move |event| {
                    let agent = agent_clone.clone();
                    if let Event::DocSync(doc_sync_event) = event {
                        if let Err(e) = agent.handle_execute_event(doc_sync_event) {
                            error!("Error handling execute event: {}", e);
                        }
                    }
                    Ok(())
                }),
            )
            .map_err(|e| DocRunnerError::EventSystemError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Handle the DOCS_EXECUTE event
    fn handle_execute_event(&self, event: DocSyncEvent) -> Result<(), DocRunnerError> {
        let correlation_id = event.correlation_id.clone();
        
        info!("Handling execute event for correlation ID: {}", correlation_id);
        
        // Extract payload
        let execute_payload = if let DocSyncPayload::Execute(payload) = event.payload {
            payload
        } else {
            return Err(DocRunnerError::InvalidPayload(
                "Expected Execute payload".to_string(),
            ));
        };
        
        // Update state to indicate we're starting execution
        self.state_manager.update_state(&correlation_id, |state| {
            state.metadata.insert("status".to_string(), "executing".to_string());
        })?;
        
        // Validate operations
        for operation in &execute_payload.operations {
            self.validate_operation(operation)?;
        }
        
        // Execute operations
        let mut execution_results = ExecutionResults {
            successful_operations: 0,
            failed_operations: 0,
            skipped_operations: 0,
            metadata: HashMap::new(),
        };
        
        let mut execution_log = Vec::new();
        
        // If dry run is enabled, skip actual file operations
        if execute_payload.execution_params.dry_run {
            info!("Dry run enabled, skipping actual file operations");
            
            for operation in &execute_payload.operations {
                let log_entry = ExecutionLogEntry {
                    operation_type: match operation {
                        SyncOperation::ContentOperation(op) => op.operation_type.clone(),
                        SyncOperation::StructureOperation(op) => op.operation_type.clone(),
                        SyncOperation::UiOperation(op) => op.operation_type.clone(),
                        SyncOperation::CssOperation(op) => op.operation_type.clone(),
                    },
                    status: "skipped".to_string(),
                    target_path: match operation {
                        SyncOperation::ContentOperation(op) => op.target_path.clone(),
                        SyncOperation::StructureOperation(op) => op.target_path.clone(),
                        SyncOperation::UiOperation(op) => op.target_path.clone(),
                        SyncOperation::CssOperation(op) => op.target_path.clone(),
                    },
                    error_message: None,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("reason".to_string(), "dry_run".to_string());
                        metadata
                    },
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                execution_log.push(log_entry);
                execution_results.skipped_operations += 1;
            }
        } else {
            // Execute each operation
            for operation in &execute_payload.operations {
                match self.execute_operation(operation) {
                    Ok(result) => {
                        let log_entry = ExecutionLogEntry {
                            operation_type: result.operation_type,
                            status: result.status.clone(),
                            target_path: result.target_path,
                            error_message: result.error_message,
                            metadata: result.metadata,
                            timestamp: result.timestamp,
                        };
                        
                        execution_log.push(log_entry);
                        
                        match result.status.as_str() {
                            "success" => execution_results.successful_operations += 1,
                            "failure" => execution_results.failed_operations += 1,
                            "skipped" => execution_results.skipped_operations += 1,
                            _ => {}
                        }
                    }
                    Err(e) => {
                        let error_message = format!("Error executing operation: {}", e);
                        error!("{}", error_message);
                        
                        let target_path = match operation {
                            SyncOperation::ContentOperation(op) => op.target_path.clone(),
                            SyncOperation::StructureOperation(op) => op.target_path.clone(),
                            SyncOperation::UiOperation(op) => op.target_path.clone(),
                            SyncOperation::CssOperation(op) => op.target_path.clone(),
                        };
                        
                        let log_entry = ExecutionLogEntry {
                            operation_type: match operation {
                                SyncOperation::ContentOperation(op) => op.operation_type.clone(),
                                SyncOperation::StructureOperation(op) => op.operation_type.clone(),
                                SyncOperation::UiOperation(op) => op.operation_type.clone(),
                                SyncOperation::CssOperation(op) => op.operation_type.clone(),
                            },
                            status: "failure".to_string(),
                            target_path,
                            error_message: Some(error_message),
                            metadata: HashMap::new(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        
                        execution_log.push(log_entry);
                        execution_results.failed_operations += 1;
                    }
                }
                
                // Report progress every 5 operations
                if (execution_results.successful_operations + execution_results.failed_operations + execution_results.skipped_operations) % 5 == 0 {
                    let total_operations = execute_payload.operations.len();
                    let completed_operations = execution_results.successful_operations + execution_results.failed_operations + execution_results.skipped_operations;
                    
                    info!(
                        "Progress: {}/{} operations completed ({} succeeded, {} failed, {} skipped)",
                        completed_operations,
                        total_operations,
                        execution_results.successful_operations,
                        execution_results.failed_operations,
                        execution_results.skipped_operations
                    );
                }
            }
        }
        
        // Update state with execution results
        self.state_manager.update_state(&correlation_id, |state| {
            state.metadata.insert("status".to_string(), "execution_complete".to_string());
            state.metadata.insert(
                "execution_summary".to_string(),
                format!(
                    "{} succeeded, {} failed, {} skipped",
                    execution_results.successful_operations,
                    execution_results.failed_operations,
                    execution_results.skipped_operations
                ),
            );
            
            // Could store execution log in state if needed
        })?;
        
        // Create and emit execution complete event
        let execution_complete_payload = ExecutionCompletePayload {
            doc_map: execute_payload.doc_map,
            execution_results,
            execution_log,
        };
        
        let execution_complete_event = DocSyncEvent::new(
            doc_sync_events::events::DOCS_EXECUTION_COMPLETE,
            DocSyncPayload::ExecutionComplete(execution_complete_payload),
            doc_sync_events::agents::DOC_RUNNER,
            doc_sync_events::agents::DOC_COORDINATOR,
            &correlation_id,
        );
        
        self.event_system
            .emit(Event::DocSync(execution_complete_event))
            .map_err(|e| DocRunnerError::EventSystemError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Validate a sync operation to ensure it's safe to execute
    fn validate_operation(&self, operation: &SyncOperation) -> Result<(), DocRunnerError> {
        match operation {
            SyncOperation::ContentOperation(op) => self.validate_content_operation(op),
            SyncOperation::StructureOperation(op) => self.validate_structure_operation(op),
            SyncOperation::UiOperation(op) => self.validate_ui_operation(op),
            SyncOperation::CssOperation(op) => self.validate_css_operation(op),
        }
    }
    
    /// Validate a content operation
    fn validate_content_operation(&self, operation: &ContentOperation) -> Result<(), DocRunnerError> {
        // Check that the target path is within the website directory
        let target_path = PathBuf::from(&operation.target_path);
        
        // Validate that the path doesn't contain ".." to prevent path traversal
        if operation.target_path.contains("..") {
            return Err(DocRunnerError::ValidationError(
                format!("Path contains '..' which is not allowed: {}", operation.target_path)
            ));
        }
        
        // For delete operations, check that the file exists
        if operation.operation_type == "delete" && !target_path.exists() {
            return Err(DocRunnerError::ValidationError(
                format!("Cannot delete non-existent file: {}", operation.target_path)
            ));
        }
        
        // For update operations, check that the file exists
        if operation.operation_type == "update" && !target_path.exists() {
            return Err(DocRunnerError::ValidationError(
                format!("Cannot update non-existent file: {}", operation.target_path)
            ));
        }
        
        // For create operations, check that the file doesn't exist
        if operation.operation_type == "create" && target_path.exists() {
            return Err(DocRunnerError::ValidationError(
                format!("Cannot create file that already exists: {}", operation.target_path)
            ));
        }
        
        // Check that content is provided for create and update operations
        if (operation.operation_type == "create" || operation.operation_type == "update") 
            && operation.content.is_none() {
            return Err(DocRunnerError::ValidationError(
                format!("Content is required for {} operation on {}", 
                    operation.operation_type, operation.target_path)
            ));
        }
        
        Ok(())
    }
    
    /// Validate a structure operation
    fn validate_structure_operation(&self, operation: &StructureOperation) -> Result<(), DocRunnerError> {
        // Check that the target path is within the website directory
        let target_path = PathBuf::from(&operation.target_path);
        
        // Validate that the path doesn't contain ".." to prevent path traversal
        if operation.target_path.contains("..") {
            return Err(DocRunnerError::ValidationError(
                format!("Path contains '..' which is not allowed: {}", operation.target_path)
            ));
        }
        
        // Additional validation specific to structure operations
        // For update_sidebar and update_navigation operations, check that content is provided
        if (operation.operation_type == "update_sidebar" || operation.operation_type == "update_navigation") 
            && operation.content.is_none() {
            return Err(DocRunnerError::ValidationError(
                format!("Content is required for {} operation on {}", 
                    operation.operation_type, operation.target_path)
            ));
        }
        
        Ok(())
    }
    
    /// Validate a UI operation
    fn validate_ui_operation(&self, operation: &UiOperation) -> Result<(), DocRunnerError> {
        // Similar validation logic as other operations
        // Check that the target path is within the website directory
        let target_path = PathBuf::from(&operation.target_path);
        
        // Validate that the path doesn't contain ".." to prevent path traversal
        if operation.target_path.contains("..") {
            return Err(DocRunnerError::ValidationError(
                format!("Path contains '..' which is not allowed: {}", operation.target_path)
            ));
        }
        
        // Additional validation specific to UI operations
        // For update_component operations, check that content is provided
        if operation.operation_type == "update_component" && operation.content.is_none() {
            return Err(DocRunnerError::ValidationError(
                format!("Content is required for {} operation on {}", 
                    operation.operation_type, operation.target_path)
            ));
        }
        
        Ok(())
    }
    
    /// Validate a CSS operation
    fn validate_css_operation(&self, operation: &CssOperation) -> Result<(), DocRunnerError> {
        // Similar validation logic as other operations
        // Check that the target path is within the website directory
        let target_path = PathBuf::from(&operation.target_path);
        
        // Validate that the path doesn't contain ".." to prevent path traversal
        if operation.target_path.contains("..") {
            return Err(DocRunnerError::ValidationError(
                format!("Path contains '..' which is not allowed: {}", operation.target_path)
            ));
        }
        
        // Additional validation specific to CSS operations
        // For update_style operations, check that content is provided
        if operation.operation_type == "update_style" && operation.content.is_none() {
            return Err(DocRunnerError::ValidationError(
                format!("Content is required for {} operation on {}", 
                    operation.operation_type, operation.target_path)
            ));
        }
        
        Ok(())
    }
    
    /// Execute a sync operation
    fn execute_operation(&self, operation: &SyncOperation) -> Result<OperationResult, DocRunnerError> {
        match operation {
            SyncOperation::ContentOperation(op) => self.execute_content_operation(op),
            SyncOperation::StructureOperation(op) => self.execute_structure_operation(op),
            SyncOperation::UiOperation(op) => self.execute_ui_operation(op),
            SyncOperation::CssOperation(op) => self.execute_css_operation(op),
        }
    }
    
    /// Execute a content operation
    fn execute_content_operation(&self, operation: &ContentOperation) -> Result<OperationResult, DocRunnerError> {
        let target_path = PathBuf::from(&operation.target_path);
        
        let result = match operation.operation_type.as_str() {
            "create" => {
                // Create directory structure if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Create the file with the provided content
                let content = operation.content.as_ref().ok_or_else(|| {
                    DocRunnerError::ExecutionError(
                        format!("Content is required for create operation on {}", operation.target_path)
                    )
                })?;
                
                let mut file = File::create(&target_path)?;
                file.write_all(content.as_bytes())?;
                
                info!("Created file: {}", operation.target_path);
                
                OperationResult {
                    operation_type: "create".to_string(),
                    status: "success".to_string(),
                    target_path: operation.target_path.clone(),
                    error_message: None,
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            },
            "update" => {
                // Update the file with the provided content
                let content = operation.content.as_ref().ok_or_else(|| {
                    DocRunnerError::ExecutionError(
                        format!("Content is required for update operation on {}", operation.target_path)
                    )
                })?;
                
                let mut file = File::create(&target_path)?;
                file.write_all(content.as_bytes())?;
                
                info!("Updated file: {}", operation.target_path);
                
                OperationResult {
                    operation_type: "update".to_string(),
                    status: "success".to_string(),
                    target_path: operation.target_path.clone(),
                    error_message: None,
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            },
            "delete" => {
                // Delete the file
                fs::remove_file(&target_path)?;
                
                info!("Deleted file: {}", operation.target_path);
                
                OperationResult {
                    operation_type: "delete".to_string(),
                    status: "success".to_string(),
                    target_path: operation.target_path.clone(),
                    error_message: None,
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            },
            _ => {
                return Err(DocRunnerError::ExecutionError(
                    format!("Unsupported operation type: {}", operation.operation_type)
                ));
            }
        };
        
        Ok(result)
    }
    
    /// Execute a structure operation
    fn execute_structure_operation(&self, operation: &StructureOperation) -> Result<OperationResult, DocRunnerError> {
        // Similar implementation as content operation, but for structure-specific operations
        // For brevity, this is a simplified implementation
        
        let target_path = PathBuf::from(&operation.target_path);
        
        let result = match operation.operation_type.as_str() {
            "update_sidebar" | "update_navigation" => {
                // Create directory structure if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Update the file with the provided content
                let content = operation.content.as_ref().ok_or_else(|| {
                    DocRunnerError::ExecutionError(
                        format!("Content is required for {} operation on {}", 
                            operation.operation_type, operation.target_path)
                    )
                })?;
                
                let mut file = File::create(&target_path)?;
                file.write_all(content.as_bytes())?;
                
                info!("Updated {}: {}", 
                    if operation.operation_type == "update_sidebar" { "sidebar" } else { "navigation" },
                    operation.target_path
                );
                
                OperationResult {
                    operation_type: operation.operation_type.clone(),
                    status: "success".to_string(),
                    target_path: operation.target_path.clone(),
                    error_message: None,
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            },
            _ => {
                return Err(DocRunnerError::ExecutionError(
                    format!("Unsupported operation type: {}", operation.operation_type)
                ));
            }
        };
        
        Ok(result)
    }
    
    /// Execute a UI operation
    fn execute_ui_operation(&self, operation: &UiOperation) -> Result<OperationResult, DocRunnerError> {
        // Similar implementation as other operations, but for UI-specific operations
        // For brevity, this is a simplified implementation
        
        let target_path = PathBuf::from(&operation.target_path);
        
        let result = match operation.operation_type.as_str() {
            "update_component" | "add_component" => {
                // Create directory structure if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Update or add the component with the provided content
                let content = operation.content.as_ref().ok_or_else(|| {
                    DocRunnerError::ExecutionError(
                        format!("Content is required for {} operation on {}", 
                            operation.operation_type, operation.target_path)
                    )
                })?;
                
                let mut file = File::create(&target_path)?;
                file.write_all(content.as_bytes())?;
                
                info!("{} component: {}", 
                    if operation.operation_type == "update_component" { "Updated" } else { "Added" },
                    operation.target_path
                );
                
                OperationResult {
                    operation_type: operation.operation_type.clone(),
                    status: "success".to_string(),
                    target_path: operation.target_path.clone(),
                    error_message: None,
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            },
            _ => {
                return Err(DocRunnerError::ExecutionError(
                    format!("Unsupported operation type: {}", operation.operation_type)
                ));
            }
        };
        
        Ok(result)
    }
    
    /// Execute a CSS operation
    fn execute_css_operation(&self, operation: &CssOperation) -> Result<OperationResult, DocRunnerError> {
        // Similar implementation as other operations, but for CSS-specific operations
        // For brevity, this is a simplified implementation
        
        let target_path = PathBuf::from(&operation.target_path);
        
        let result = match operation.operation_type.as_str() {
            "update_style" | "add_style" => {
                // Create directory structure if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Update or add the style with the provided content
                let content = operation.content.as_ref().ok_or_else(|| {
                    DocRunnerError::ExecutionError(
                        format!("Content is required for {} operation on {}", 
                            operation.operation_type, operation.target_path)
                    )
                })?;
                
                let mut file = File::create(&target_path)?;
                file.write_all(content.as_bytes())?;
                
                info!("{} style: {}", 
                    if operation.operation_type == "update_style" { "Updated" } else { "Added" },
                    operation.target_path
                );
                
                OperationResult {
                    operation_type: operation.operation_type.clone(),
                    status: "success".to_string(),
                    target_path: operation.target_path.clone(),
                    error_message: None,
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            },
            _ => {
                return Err(DocRunnerError::ExecutionError(
                    format!("Unsupported operation type: {}", operation.operation_type)
                ));
            }
        };
        
        Ok(result)
    }
}

// Clone implementation for use with Arc
impl Clone for DocRunnerAgent {
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            event_system: self.event_system.clone(),
        }
    }
} 