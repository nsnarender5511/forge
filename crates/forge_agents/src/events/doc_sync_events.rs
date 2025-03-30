use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::events::EventSystem;
use std::path::PathBuf;

/// DocSyncEvent represents an event in the documentation synchronization system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSyncEvent {
    pub event_type: String,
    pub payload: DocSyncPayload,
    pub source_agent: String,
    pub target_agent: String,
    pub timestamp: u64,
    pub correlation_id: String,
}

impl DocSyncEvent {
    pub fn new(
        event_type: &str,
        payload: DocSyncPayload,
        source_agent: &str,
        target_agent: &str,
        correlation_id: &str,
    ) -> Self {
        Self {
            event_type: event_type.to_string(),
            payload,
            source_agent: source_agent.to_string(),
            target_agent: target_agent.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            correlation_id: correlation_id.to_string(),
        }
    }

    pub fn generate_correlation_id() -> String {
        Uuid::new_v4().to_string()
    }
}

/// DocSyncPayload contains the data specific to different event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocSyncPayload {
    Start(StartPayload),
    AnalyzeContent(AnalyzeContentPayload),
    ContentAnalyzed(ContentAnalyzedPayload),
    AnalyzeDocusaurus(AnalyzeDocusaurusPayload),
    DocusaurusAnalyzed(DocusaurusAnalyzedPayload),
    AnalyzeUi(AnalyzeUiPayload),
    UiAnalyzed(UiAnalyzedPayload),
    AnalyzeCss(AnalyzeCssPayload),
    CssAnalyzed(CssAnalyzedPayload),
    Execute(ExecutePayload),
    ExecutionComplete(ExecutionCompletePayload),
    Verify(VerifyPayload),
    VerificationComplete(VerificationCompletePayload),
    Complete(CompletePayload),
    PlanChanges(PlanChangesPayload),
    ChangesPlanned(ChangesPlannedPayload),
    ImplementChanges(ImplementChangesPayload),
    ImplementationComplete(ImplementationCompletePayload),
    IterationDecision(IterationDecisionPayload),
    IterationComplete(IterationCompletePayload),
}

/// StartPayload contains parameters for initializing the synchronization process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPayload {
    pub source_path: String,
    pub target_path: String,
    pub scope: String, // "full", "incremental", "specific"
    pub options: HashMap<String, String>,
}

/// AnalyzeContentPayload contains parameters for content analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeContentPayload {
    pub source_path: String,
    pub doc_map: Option<DocumentationMap>,
}

/// ContentAnalyzedPayload contains the results of content analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalyzedPayload {
    pub doc_map: DocumentationMap,
    pub content_operations: Vec<ContentOperation>,
    pub analysis_report: AnalysisReport,
}

/// AnalyzeDocusaurusPayload contains parameters for Docusaurus-specific analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeDocusaurusPayload {
    pub target_path: String,
    pub doc_map: Option<DocumentationMap>,
}

/// DocusaurusAnalyzedPayload contains the results of Docusaurus-specific analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocusaurusAnalyzedPayload {
    pub doc_map: DocumentationMap,
    pub structure_operations: Vec<StructureOperation>,
    pub analysis_report: AnalysisReport,
}

/// AnalyzeUiPayload contains parameters for UI analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeUiPayload {
    pub target_path: String,
    pub doc_map: Option<DocumentationMap>,
}

/// UiAnalyzedPayload contains the results of UI analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAnalyzedPayload {
    pub doc_map: DocumentationMap,
    pub ui_operations: Vec<UiOperation>,
    pub analysis_report: AnalysisReport,
}

/// AnalyzeCssPayload contains parameters for CSS analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeCssPayload {
    pub target_path: String,
    pub doc_map: Option<DocumentationMap>,
}

/// CssAnalyzedPayload contains the results of CSS analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssAnalyzedPayload {
    pub doc_map: DocumentationMap,
    pub css_operations: Vec<CssOperation>,
    pub analysis_report: AnalysisReport,
}

/// ExecutePayload contains parameters for executing synchronization operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutePayload {
    pub doc_map: DocumentationMap,
    pub operations: Vec<SyncOperation>,
    pub execution_params: ExecutionParams,
}

/// ExecutionCompletePayload contains the results of executing synchronization operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCompletePayload {
    pub doc_map: DocumentationMap,
    pub execution_results: ExecutionResults,
    pub execution_log: Vec<ExecutionLogEntry>,
}

/// VerifyPayload contains parameters for verifying synchronization results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyPayload {
    pub doc_map: DocumentationMap,
    pub execution_log: Vec<ExecutionLogEntry>,
    pub verification_params: VerificationParams,
}

/// VerificationCompletePayload contains the results of verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCompletePayload {
    pub doc_map: DocumentationMap,
    pub verification_results: VerificationResults,
    pub quality_scores: QualityMetrics,
    pub verification_report: VerificationReport,
    pub verification_log: Vec<VerificationLogEntry>,
    pub check_status: HashMap<String, bool>,
    pub improvement_recommendations: Vec<String>,
}

/// CompletePayload contains the final results of the synchronization process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletePayload {
    pub doc_map: DocumentationMap,
    pub quality_scores: QualityMetrics,
    pub summary_report: SyncSummaryReport,
}

/// PlanChangesPayload contains parameters for planning changes based on verification results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanChangesPayload {
    pub doc_map: DocumentationMap,
    pub verification_results: VerificationResults,
    pub verification_report: VerificationReport,
    pub iteration_number: u32,
    pub requirements: HashMap<String, bool>,
}

/// ChangesPlannedPayload contains the results of change planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesPlannedPayload {
    pub doc_map: DocumentationMap,
    pub planned_operations: Vec<SyncOperation>,
    pub planning_report: PlanningReport,
    pub iteration_number: u32,
}

/// ImplementChangesPayload contains parameters for implementing planned changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementChangesPayload {
    pub doc_map: DocumentationMap,
    pub planned_operations: Vec<SyncOperation>,
    pub execution_params: ExecutionParams,
    pub iteration_number: u32,
}

/// ImplementationCompletePayload contains the results of change implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationCompletePayload {
    pub doc_map: DocumentationMap,
    pub implementation_results: ImplementationResults,
    pub implementation_log: Vec<ImplementationLogEntry>,
    pub iteration_number: u32,
}

/// IterationDecisionPayload contains data for deciding whether to continue iterating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationDecisionPayload {
    pub doc_map: DocumentationMap,
    pub verification_results: VerificationResults,
    pub iteration_number: u32,
    pub requirements_satisfied: HashMap<String, bool>,
    pub continue_iteration: bool,
}

/// IterationCompletePayload contains the results of a complete iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationCompletePayload {
    pub doc_map: DocumentationMap,
    pub final_verification_results: VerificationResults,
    pub iteration_count: u32,
    pub quality_scores: QualityMetrics,
    pub summary_report: SyncSummaryReport,
}

/// PlanningReport contains information about the planning phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningReport {
    pub issues_addressed: Vec<String>,
    pub planned_operations_count: usize,
    pub improvement_focus_areas: Vec<String>,
    pub estimated_impact: HashMap<String, f64>,
    pub metadata: HashMap<String, String>,
}

/// ImplementationResults contains information about the implementation phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationResults {
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub modified_files: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// ImplementationLogEntry contains information about a specific implementation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationLogEntry {
    pub operation_type: String,
    pub status: String, // "success", "failure", "skipped", etc.
    pub target_path: String,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}

// Supporting data structures

/// DocumentationMap represents the central data structure tracking all documentation files.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentationMap {
    pub source_files: HashMap<String, DocSourceFile>,
    pub target_files: HashMap<String, DocTargetFile>,
    pub relationships: Vec<DocRelationship>,
    pub metadata: HashMap<String, String>,
}

/// DocSourceFile represents a source documentation file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSourceFile {
    pub path: String,
    pub content_hash: String,
    pub metadata: HashMap<String, String>,
    pub last_updated: u64,
}

/// DocTargetFile represents a target file in the Docusaurus website.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTargetFile {
    pub path: String,
    pub content_hash: String,
    pub file_type: String, // "markdown", "config", "component", etc.
    pub metadata: HashMap<String, String>,
    pub last_updated: u64,
}

/// DocRelationship represents a relationship between source and target files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRelationship {
    pub source_path: String,
    pub target_path: String,
    pub relationship_type: String, // "primary", "derived", "related", etc.
    pub metadata: HashMap<String, String>,
}

/// SyncOperation represents an operation to be performed during synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation_type")]
pub enum SyncOperation {
    ContentOperation(ContentOperation),
    StructureOperation(StructureOperation),
    UiOperation(UiOperation),
    CssOperation(CssOperation),
}

/// ContentOperation represents an operation related to content synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentOperation {
    pub operation_type: String, // "create", "update", "delete", etc.
    pub source_path: String,
    pub target_path: String,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// StructureOperation represents an operation related to Docusaurus structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureOperation {
    pub operation_type: String, // "update_sidebar", "update_navigation", etc.
    pub target_path: String,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// UiOperation represents an operation related to UI components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiOperation {
    pub operation_type: String, // "update_component", "add_component", etc.
    pub target_path: String,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// CssOperation represents an operation related to CSS styling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssOperation {
    pub operation_type: String, // "update_style", "add_style", etc.
    pub target_path: String,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// AnalysisReport contains the results of analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub total_items: usize,
    pub findings: Vec<Finding>,
    pub operations: Vec<Operation>,
    pub summary: String,
}

/// Finding represents a finding from analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub category: String,
    pub severity: Severity,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub recommendation: Option<String>,
}

/// Severity represents the severity of a finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Operation represents an operation from analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub op_type: OperationType,
    pub path: String,
    pub content: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// OperationType represents the type of an operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Move,
    Copy,
}

/// ExecutionParams contains parameters for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    pub dry_run: bool,
    pub max_parallel_operations: usize,
    pub backup: bool,
    pub metadata: HashMap<String, String>,
}

/// ExecutionResults contains the results of execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResults {
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub metadata: HashMap<String, String>,
}

/// ExecutionLogEntry represents an entry in the execution log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogEntry {
    pub operation_type: String,
    pub status: String, // "success", "failure", "skipped", etc.
    pub target_path: String,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}

/// VerificationParams contains parameters for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationParams {
    pub verify_content: bool,
    pub verify_structure: bool,
    pub verify_ui: bool,
    pub verify_css: bool,
    pub metadata: HashMap<String, String>,
}

/// VerificationResults contains the results of verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResults {
    pub content_verification: VerificationResult,
    pub structure_verification: VerificationResult,
    pub ui_verification: VerificationResult,
    pub css_verification: VerificationResult,
    pub successful_checks: u32,
    pub failed_checks: u32,
    pub skipped_checks: u32,
    pub quality_metrics: QualityMetrics,
    pub metadata: HashMap<String, String>,
}

/// VerificationResult represents the result of a verification check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub status: String, // "success", "failure", "partial", etc.
    pub issues_found: usize,
    pub critical_issues: usize,
    pub metadata: HashMap<String, String>,
}

/// VerificationReport contains the detailed results of verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub issues: Vec<VerificationIssue>,
    pub positives: Vec<VerificationPositive>,
    pub recommendations: Vec<VerificationRecommendation>,
    pub metadata: HashMap<String, String>,
}

/// VerificationIssue represents an issue found during verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    pub issue_type: String, // "content_mismatch", "broken_link", etc.
    pub severity: String,   // "critical", "major", "minor", etc.
    pub description: String,
    pub related_paths: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// VerificationPositive represents a positive finding during verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPositive {
    pub positive_type: String, // "improved_readability", "better_structure", etc.
    pub description: String,
    pub related_paths: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// VerificationRecommendation represents a recommendation from verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecommendation {
    pub recommendation_type: String, // "improve_content", "fix_links", etc.
    pub priority: String,           // "high", "medium", "low", etc.
    pub description: String,
    pub related_paths: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// QualityMetrics contains the quality metrics for the documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub content_quality: f64,    // Was content_score
    pub structure_quality: f64,  // Was structure_score
    pub ui_quality: f64,         // Was ui_score
    pub css_quality: f64,        // New field
    pub technical_quality: f64,  // Was technical_score
    pub overall_quality: f64,    // Was overall_score
    pub metadata: HashMap<String, String>,
}

/// SyncSummaryReport contains the summary of the synchronization process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummaryReport {
    pub files_analyzed: usize,
    pub files_created: usize,
    pub files_updated: usize,
    pub files_deleted: usize,
    pub operations_executed: usize,
    pub operations_failed: usize,
    pub critical_issues: usize,
    pub major_issues: usize,
    pub minor_issues: usize,
    pub overall_status: String, // "success", "partial", "failure", etc.
    pub metadata: HashMap<String, String>,
}

/// VerificationLogEntry represents an entry in the verification log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationLogEntry {
    pub check_type: String,
    pub status: String, // "success", "failure", "skipped", etc.
    pub message: String,
    pub target_path: Option<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}

/// Register event handlers for documentation synchronization events
pub fn register_doc_sync_event_handlers(_event_system: &mut EventSystem) {
    // Register event handlers here when implemented
}

// Constants for event types
pub mod events {
    pub const DOCS_START: &str = "docs-start";
    pub const DOCS_ANALYZE_CONTENT: &str = "docs-analyze-content";
    pub const DOCS_CONTENT_ANALYZED: &str = "docs-content-analyzed";
    pub const DOCS_ANALYZE_DOCUSAURUS: &str = "docs-analyze-docusaurus";
    pub const DOCS_DOCUSAURUS_ANALYZED: &str = "docs-docusaurus-analyzed";
    pub const DOCS_ANALYZE_UI: &str = "docs-analyze-ui";
    pub const DOCS_UI_ANALYZED: &str = "docs-ui-analyzed";
    pub const DOCS_ANALYZE_CSS: &str = "docs-analyze-css";
    pub const DOCS_CSS_ANALYZED: &str = "docs-css-analyzed";
    pub const DOCS_EXECUTE: &str = "docs-execute";
    pub const DOCS_EXECUTION_COMPLETE: &str = "docs-execution-complete";
    pub const DOCS_VERIFY: &str = "docs-verify";
    pub const DOCS_VERIFICATION_COMPLETE: &str = "docs-verification-complete";
    pub const DOCS_COMPLETE: &str = "docs-complete";
}

// Constants for agent identifiers
pub mod agents {
    pub const DOC_COORDINATOR: &str = "doc-coordinator";
    pub const DOC_CONTENT_SYNCER: &str = "doc-content-syncer";
    pub const DOCUSAURUS_EXPERT: &str = "docusaurus-expert";
    pub const UI_DESIGN_EXPERT: &str = "ui-design-expert";
    pub const CSS_EXPERT: &str = "css-expert";
    pub const DOC_RUNNER: &str = "doc-runner";
    pub const DOC_VERIFIER: &str = "doc-verifier";
}

// Common types used across events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMap {
    pub source_path: String,
    pub target_path: String,
    pub documents: HashMap<String, DocumentInfo>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub path: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub frontmatter: Option<HashMap<String, serde_json::Value>>,
    pub last_modified: Option<String>,
    pub content_hash: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysisPayload {
    pub documentation_map: DocumentMap,
    pub source_path: String,
    pub target_path: String,
    pub analysis_parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocusaurusAnalysisPayload {
    pub documentation_map: DocumentMap,
    pub docusaurus_path: String,
    pub analysis_parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAnalysisPayload {
    pub documentation_map: DocumentMap,
    pub ui_parameters: HashMap<String, serde_json::Value>,
    pub component_references: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssAnalysisPayload {
    pub documentation_map: DocumentMap,
    pub css_parameters: HashMap<String, serde_json::Value>,
    pub style_references: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPayload {
    pub operations: Vec<Operation>,
    pub documentation_map: DocumentMap,
    pub execution_parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPayload {
    pub documentation_map: DocumentMap,
    pub execution_logs: HashMap<String, serde_json::Value>,
    pub verification_parameters: HashMap<String, serde_json::Value>,
} 