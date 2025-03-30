use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use log::{debug, error, info, warn};

use crate::events::{Event, EventHandler, EventSystem};
use crate::events::doc_sync_events::{
    self, DocSyncEvent, DocSyncPayload, DocumentationMap, VerifyPayload,
    VerificationCompletePayload, ExecutionLogEntry, QualityMetrics,
    VerificationResults, VerificationLogEntry, VerificationParams, VerificationResult, 
    VerificationReport
};
use crate::state::doc_sync_state::{StateManager, DocSyncState, StateError};

/// Errors that can occur during Doc Verifier Agent operations
#[derive(Debug, Error)]
pub enum DocVerifierError {
    #[error("State error: {0}")]
    StateError(#[from] StateError),
    #[error("Event system error: {0}")]
    EventSystemError(String),
    #[error("Invalid event payload: {0}")]
    InvalidPayload(String),
    #[error("Verification error: {0}")]
    VerificationError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Doc Verifier Agent verifies the quality and completeness of documentation synchronization
pub struct DocVerifierAgent {
    state_manager: Arc<StateManager>,
    event_system: Arc<EventSystem>,
}

impl DocVerifierAgent {
    /// Create a new DocVerifierAgent with the specified state manager and event system
    pub fn new(state_manager: Arc<StateManager>, event_system: Arc<EventSystem>) -> Self {
        Self {
            state_manager,
            event_system,
        }
    }
    
    /// Initialize the agent by registering event handlers
    pub fn initialize(&self) -> Result<(), DocVerifierError> {
        self.register_event_handlers()?;
        Ok(())
    }
    
    /// Register event handlers for the agent
    fn register_event_handlers(&self) -> Result<(), DocVerifierError> {
        let agent_clone = Arc::new(self.clone());
        
        // Register handler for verify event
        self.event_system
            .register_handler(
                doc_sync_events::events::DOCS_VERIFY,
                Box::new(move |event| {
                    let agent = agent_clone.clone();
                    if let Event::DocSync(doc_sync_event) = event {
                        if let Err(e) = agent.handle_verify_event(doc_sync_event) {
                            error!("Error handling verify event: {}", e);
                        }
                    }
                    Ok(())
                }),
            )
            .map_err(|e| DocVerifierError::EventSystemError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Handle the DOCS_VERIFY event
    fn handle_verify_event(&self, event: DocSyncEvent) -> Result<(), DocVerifierError> {
        let correlation_id = event.correlation_id.clone();
        
        info!("Handling verify event for correlation ID: {}", correlation_id);
        
        // Extract payload
        let verify_payload = if let DocSyncPayload::Verify(payload) = event.payload {
            payload
        } else {
            return Err(DocVerifierError::InvalidPayload(
                "Expected Verify payload".to_string(),
            ));
        };
        
        // Update state to indicate we're starting verification
        self.state_manager.update_state(&correlation_id, |state| {
            state.metadata.insert("status".to_string(), "verifying".to_string());
        })?;
        
        // Extract state information
        let state = self.state_manager.read_state(&correlation_id)?;
        let source_path = state
            .metadata
            .get("source_path")
            .ok_or_else(|| {
                DocVerifierError::VerificationError("Source path not found in state".to_string())
            })?
            .clone();
        let target_path = state
            .metadata
            .get("target_path")
            .ok_or_else(|| {
                DocVerifierError::VerificationError("Target path not found in state".to_string())
            })?
            .clone();
        
        // Perform verification
        info!("Starting verification of documentation synchronization");
        info!("Source path: {}", source_path);
        info!("Target path: {}", target_path);
        
        let mut verification_results = VerificationResults {
            content_verification: VerificationResult {
                status: "pending".to_string(),
                issues_found: 0,
                critical_issues: 0,
                metadata: HashMap::new(),
            },
            structure_verification: VerificationResult {
                status: "pending".to_string(),
                issues_found: 0,
                critical_issues: 0,
                metadata: HashMap::new(),
            },
            ui_verification: VerificationResult {
                status: "pending".to_string(),
                issues_found: 0,
                critical_issues: 0,
                metadata: HashMap::new(),
            },
            css_verification: VerificationResult {
                status: "pending".to_string(),
                issues_found: 0,
                critical_issues: 0,
                metadata: HashMap::new(),
            },
            successful_checks: 0,
            failed_checks: 0,
            skipped_checks: 0,
            quality_metrics: QualityMetrics {
                content_quality: 0.0,
                structure_quality: 0.0,
                ui_quality: 0.0,
                css_quality: 0.0,
                technical_quality: 0.0,
                overall_quality: 0.0,
                metadata: HashMap::new(),
            },
            metadata: HashMap::new(),
        };
        
        let mut verification_log = Vec::new();
        
        // Execute verification checks
        let doc_map = verify_payload.doc_map.clone();
        let execution_log = verify_payload.execution_log.clone();
        
        // 1. Content verification checks
        let content_metrics = self.verify_content(
            &source_path,
            &target_path,
            &doc_map,
            &execution_log,
            &verify_payload.verification_params,
        )?;
        
        verification_results.successful_checks += content_metrics.0;
        verification_results.failed_checks += content_metrics.1;
        verification_results.skipped_checks += content_metrics.2;
        verification_results.quality_metrics.content_quality = content_metrics.3;
        
        verification_log.extend(content_metrics.4);
        
        // Report progress
        info!(
            "Content verification completed: {} successful, {} failed, {} skipped, quality score: {:.2}",
            content_metrics.0, content_metrics.1, content_metrics.2, content_metrics.3
        );
        
        // 2. Structure verification checks
        let structure_metrics = self.verify_structure(
            &source_path,
            &target_path,
            &doc_map,
            &execution_log,
            &verify_payload.verification_params,
        )?;
        
        verification_results.successful_checks += structure_metrics.0;
        verification_results.failed_checks += structure_metrics.1;
        verification_results.skipped_checks += structure_metrics.2;
        verification_results.quality_metrics.structure_quality = structure_metrics.3;
        
        verification_log.extend(structure_metrics.4);
        
        // Report progress
        info!(
            "Structure verification completed: {} successful, {} failed, {} skipped, quality score: {:.2}",
            structure_metrics.0, structure_metrics.1, structure_metrics.2, structure_metrics.3
        );
        
        // 3. UI verification checks
        let ui_metrics = self.verify_ui(
            &source_path,
            &target_path,
            &doc_map,
            &execution_log,
            &verify_payload.verification_params,
        )?;
        
        verification_results.successful_checks += ui_metrics.0;
        verification_results.failed_checks += ui_metrics.1;
        verification_results.skipped_checks += ui_metrics.2;
        verification_results.quality_metrics.ui_quality = ui_metrics.3;
        
        verification_log.extend(ui_metrics.4);
        
        // Report progress
        info!(
            "UI verification completed: {} successful, {} failed, {} skipped, quality score: {:.2}",
            ui_metrics.0, ui_metrics.1, ui_metrics.2, ui_metrics.3
        );
        
        // 4. CSS verification checks
        let css_metrics = self.verify_css(
            &source_path,
            &target_path,
            &doc_map,
            &execution_log,
            &verify_payload.verification_params,
        )?;
        
        verification_results.successful_checks += css_metrics.0;
        verification_results.failed_checks += css_metrics.1;
        verification_results.skipped_checks += css_metrics.2;
        verification_results.quality_metrics.css_quality = css_metrics.3;
        
        verification_log.extend(css_metrics.4);
        
        // Report progress
        info!(
            "CSS verification completed: {} successful, {} failed, {} skipped, quality score: {:.2}",
            css_metrics.0, css_metrics.1, css_metrics.2, css_metrics.3
        );
        
        // 5. Technical verification checks
        let technical_metrics = self.verify_technical(
            &source_path,
            &target_path,
            &doc_map,
            &execution_log,
            &verify_payload.verification_params,
        )?;
        
        verification_results.successful_checks += technical_metrics.0;
        verification_results.failed_checks += technical_metrics.1;
        verification_results.skipped_checks += technical_metrics.2;
        verification_results.quality_metrics.technical_quality = technical_metrics.3;
        
        verification_log.extend(technical_metrics.4);
        
        // Report progress
        info!(
            "Technical verification completed: {} successful, {} failed, {} skipped, quality score: {:.2}",
            technical_metrics.0, technical_metrics.1, technical_metrics.2, technical_metrics.3
        );
        
        // Calculate overall quality score
        verification_results.quality_metrics.overall_quality = (
            verification_results.quality_metrics.content_quality * 0.4 +
            verification_results.quality_metrics.structure_quality * 0.2 +
            verification_results.quality_metrics.ui_quality * 0.15 +
            verification_results.quality_metrics.css_quality * 0.1 +
            verification_results.quality_metrics.technical_quality * 0.15
        );
        
        // Update state with verification results
        self.state_manager.update_state(&correlation_id, |state| {
            state.metadata.insert("status".to_string(), "verification_complete".to_string());
            state.metadata.insert(
                "verification_summary".to_string(),
                format!(
                    "{} successful, {} failed, {} skipped, overall quality: {:.2}",
                    verification_results.successful_checks,
                    verification_results.failed_checks,
                    verification_results.skipped_checks,
                    verification_results.quality_metrics.overall_quality
                ),
            );
        })?;
        
        // Create a proper verification report
        let verification_report = VerificationReport {
            issues: vec![], // Add any issues found
            positives: vec![], // Add any positives found
            recommendations: vec![], // Add any recommendations
            metadata: HashMap::new(),
        };

        // Create status map
        let mut check_status = HashMap::new();
        check_status.insert("content".to_string(), verification_results.content_verification.status == "success");
        check_status.insert("structure".to_string(), verification_results.structure_verification.status == "success");
        check_status.insert("ui".to_string(), verification_results.ui_verification.status == "success");
        check_status.insert("css".to_string(), verification_results.css_verification.status == "success");

        // Create recommendation list
        let improvement_recommendations = vec![
            "Ensure all links are valid".to_string(),
            "Check image references".to_string(),
            "Validate code samples".to_string()
        ];

        // Fix VerificationCompletePayload with all required fields
        let verification_complete_payload = VerificationCompletePayload {
            doc_map,
            verification_results: verification_results.clone(),
            verification_report,
            verification_log,
            check_status,
            improvement_recommendations,
            quality_scores: verification_results.quality_metrics.clone(),
        };
        
        let verification_complete_event = DocSyncEvent::new(
            doc_sync_events::events::DOCS_VERIFICATION_COMPLETE,
            DocSyncPayload::VerificationComplete(verification_complete_payload),
            doc_sync_events::agents::DOC_VERIFIER,
            doc_sync_events::agents::DOC_COORDINATOR,
            &correlation_id,
        );
        
        self.event_system
            .emit(Event::DocSync(verification_complete_event))
            .map_err(|e| DocVerifierError::EventSystemError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Verify content (markdown, text) files
    fn verify_content(
        &self,
        source_path: &str,
        target_path: &str,
        doc_map: &DocumentationMap,
        execution_log: &[ExecutionLogEntry],
        params: &VerificationParams,
    ) -> Result<(u32, u32, u32, f64, Vec<VerificationLogEntry>), DocVerifierError> {
        info!("Verifying content files");
        
        let mut successful = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut quality_score = 0.0;
        let mut log_entries = Vec::new();
        
        // In a real implementation, we would perform the following checks:
        // 1. Verify that all content files in doc_map exist in the target directory
        // 2. Verify content formatting and structure (headers, links, etc.)
        // 3. Verify that content follows the documentation standards
        // 4. Check for broken links within the content
        // 5. Verify code examples work correctly
        
        // For brevity, this is a simplified implementation
        let source_dir = PathBuf::from(source_path);
        let target_dir = PathBuf::from(target_path);
        
        // Check that at least one content operation was executed successfully
        let content_operations = execution_log.iter()
            .filter(|entry| entry.operation_type.starts_with("create") || entry.operation_type.starts_with("update"))
            .filter(|entry| entry.status == "success")
            .count();
        
        if content_operations > 0 {
            successful += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "content".to_string(),
                status: "success".to_string(),
                message: format!("{} content operations executed successfully", content_operations),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        } else {
            failed += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "content".to_string(),
                status: "failure".to_string(),
                message: "No content operations were executed successfully".to_string(),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        }
        
        // Set quality score based on successful vs. failed checks
        if successful > 0 && failed == 0 {
            quality_score = 1.0;
        } else if successful > 0 {
            quality_score = successful as f64 / (successful + failed) as f64;
        }
        
        Ok((successful, failed, skipped, quality_score, log_entries))
    }
    
    /// Verify structure (navigation, sidebar, etc.)
    fn verify_structure(
        &self,
        source_path: &str,
        target_path: &str,
        doc_map: &DocumentationMap,
        execution_log: &[ExecutionLogEntry],
        params: &VerificationParams,
    ) -> Result<(u32, u32, u32, f64, Vec<VerificationLogEntry>), DocVerifierError> {
        info!("Verifying structure files");
        
        let mut successful = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut quality_score = 0.0;
        let mut log_entries = Vec::new();
        
        // In a real implementation, we would perform the following checks:
        // 1. Verify that the sidebar.js file exists and is properly formatted
        // 2. Verify that the navigation structure matches the documentation map
        // 3. Verify that all sections have appropriate navigation
        
        // For brevity, this is a simplified implementation
        let target_dir = PathBuf::from(target_path);
        
        // Check if any structure operations were executed
        let structure_operations = execution_log.iter()
            .filter(|entry| entry.operation_type.contains("sidebar") || entry.operation_type.contains("navigation"))
            .filter(|entry| entry.status == "success")
            .count();
        
        if structure_operations > 0 {
            successful += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "structure".to_string(),
                status: "success".to_string(),
                message: format!("{} structure operations executed successfully", structure_operations),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        } else {
            // Not a failure if no structure operations were needed
            skipped += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "structure".to_string(),
                status: "skipped".to_string(),
                message: "No structure operations were executed".to_string(),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        }
        
        // Set quality score based on successful vs. failed checks
        if successful > 0 && failed == 0 {
            quality_score = 1.0;
        } else if successful > 0 {
            quality_score = successful as f64 / (successful + failed) as f64;
        } else if skipped > 0 && failed == 0 {
            // If all checks were skipped but none failed, assume good quality
            quality_score = 0.8;
        }
        
        Ok((successful, failed, skipped, quality_score, log_entries))
    }
    
    /// Verify UI components
    fn verify_ui(
        &self,
        source_path: &str,
        target_path: &str,
        doc_map: &DocumentationMap,
        execution_log: &[ExecutionLogEntry],
        params: &VerificationParams,
    ) -> Result<(u32, u32, u32, f64, Vec<VerificationLogEntry>), DocVerifierError> {
        info!("Verifying UI components");
        
        let mut successful = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut quality_score = 0.0;
        let mut log_entries = Vec::new();
        
        // In a real implementation, we would perform the following checks:
        // 1. Verify that UI components are properly formatted
        // 2. Verify that UI components render correctly
        // 3. Verify that UI components are accessible
        
        // For brevity, this is a simplified implementation
        let target_dir = PathBuf::from(target_path);
        
        // Check if any UI operations were executed
        let ui_operations = execution_log.iter()
            .filter(|entry| entry.operation_type.contains("component"))
            .filter(|entry| entry.status == "success")
            .count();
        
        if ui_operations > 0 {
            successful += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "ui".to_string(),
                status: "success".to_string(),
                message: format!("{} UI operations executed successfully", ui_operations),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        } else {
            // Not a failure if no UI operations were needed
            skipped += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "ui".to_string(),
                status: "skipped".to_string(),
                message: "No UI operations were executed".to_string(),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        }
        
        // Set quality score based on successful vs. failed checks
        if successful > 0 && failed == 0 {
            quality_score = 1.0;
        } else if successful > 0 {
            quality_score = successful as f64 / (successful + failed) as f64;
        } else if skipped > 0 && failed == 0 {
            // If all checks were skipped but none failed, assume good quality
            quality_score = 0.8;
        }
        
        Ok((successful, failed, skipped, quality_score, log_entries))
    }
    
    /// Verify CSS styles
    fn verify_css(
        &self,
        source_path: &str,
        target_path: &str,
        doc_map: &DocumentationMap,
        execution_log: &[ExecutionLogEntry],
        params: &VerificationParams,
    ) -> Result<(u32, u32, u32, f64, Vec<VerificationLogEntry>), DocVerifierError> {
        info!("Verifying CSS styles");
        
        let mut successful = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut quality_score = 0.0;
        let mut log_entries = Vec::new();
        
        // In a real implementation, we would perform the following checks:
        // 1. Verify that CSS files are properly formatted
        // 2. Verify that CSS files contain the expected styles
        // 3. Verify that CSS files are optimized for performance
        
        // For brevity, this is a simplified implementation
        let target_dir = PathBuf::from(target_path);
        
        // Check if any CSS operations were executed
        let css_operations = execution_log.iter()
            .filter(|entry| entry.operation_type.contains("style"))
            .filter(|entry| entry.status == "success")
            .count();
        
        if css_operations > 0 {
            successful += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "css".to_string(),
                status: "success".to_string(),
                message: format!("{} CSS operations executed successfully", css_operations),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        } else {
            // Not a failure if no CSS operations were needed
            skipped += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "css".to_string(),
                status: "skipped".to_string(),
                message: "No CSS operations were executed".to_string(),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        }
        
        // Set quality score based on successful vs. failed checks
        if successful > 0 && failed == 0 {
            quality_score = 1.0;
        } else if successful > 0 {
            quality_score = successful as f64 / (successful + failed) as f64;
        } else if skipped > 0 && failed == 0 {
            // If all checks were skipped but none failed, assume good quality
            quality_score = 0.8;
        }
        
        Ok((successful, failed, skipped, quality_score, log_entries))
    }
    
    /// Verify technical aspects
    fn verify_technical(
        &self,
        source_path: &str,
        target_path: &str,
        doc_map: &DocumentationMap,
        execution_log: &[ExecutionLogEntry],
        params: &VerificationParams,
    ) -> Result<(u32, u32, u32, f64, Vec<VerificationLogEntry>), DocVerifierError> {
        info!("Verifying technical aspects");
        
        let mut successful = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut quality_score = 0.0;
        let mut log_entries = Vec::new();
        
        // In a real implementation, we would perform the following checks:
        // 1. Verify that all files are properly formatted
        // 2. Verify that all links are working
        // 3. Verify that all images are optimized
        // It would also include performance and SEO checks
        
        // For brevity, this is a simplified implementation
        
        // Check 1: Verify execution success rate
        let total_operations = execution_log.len();
        let successful_operations = execution_log.iter()
            .filter(|entry| entry.status == "success")
            .count();
        
        if total_operations > 0 {
            let success_rate = successful_operations as f64 / total_operations as f64;
            
            if success_rate >= 0.9 {
                successful += 1;
                
                let verification_log_entry = VerificationLogEntry {
                    check_type: "technical".to_string(),
                    status: "success".to_string(),
                    message: format!("Execution success rate: {:.1}%", success_rate * 100.0),
                    target_path: Some(target_path.to_string()),
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                log_entries.push(verification_log_entry);
            } else {
                failed += 1;
                
                let verification_log_entry = VerificationLogEntry {
                    check_type: "technical".to_string(),
                    status: "failure".to_string(),
                    message: format!("Execution success rate too low: {:.1}%", success_rate * 100.0),
                    target_path: Some(target_path.to_string()),
                    metadata: HashMap::new(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                log_entries.push(verification_log_entry);
            }
        } else {
            skipped += 1;
            
            let verification_log_entry = VerificationLogEntry {
                check_type: "technical".to_string(),
                status: "skipped".to_string(),
                message: "No operations were executed".to_string(),
                target_path: Some(target_path.to_string()),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            log_entries.push(verification_log_entry);
        }
        
        // Set quality score based on successful vs. failed checks
        if successful > 0 && failed == 0 {
            quality_score = 1.0;
        } else if successful > 0 {
            quality_score = successful as f64 / (successful + failed) as f64;
        } else if skipped > 0 && failed == 0 {
            // If all checks were skipped but none failed, assume good quality
            quality_score = 0.8;
        }
        
        Ok((successful, failed, skipped, quality_score, log_entries))
    }
}

// Clone implementation for use with Arc
impl Clone for DocVerifierAgent {
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            event_system: self.event_system.clone(),
        }
    }
} 