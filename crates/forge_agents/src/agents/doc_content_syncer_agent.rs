use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error, info};
use sha2::{Sha256, Digest};

use crate::events::{
    EventError, EventSystem, DOCS_ANALYZE_CONTENT, DOCS_CONTENT_ANALYZED,
};
use crate::events::Event as CrateEvent;
use crate::events::doc_sync_events::{
    AnalysisReport, ContentAnalysisPayload, ContentAnalyzedPayload, DocumentMap, Finding, Operation,
    OperationType, Severity, ContentOperation,
    DocumentationMap, DocumentInfo,
};
use crate::state::StateManager;
use crate::utils::{calculate_file_hash, find_files, get_relative_path, path_exists, read_file_to_string};

/// Structure to hold the results of content analysis
struct ContentAnalysisResult {
    report: AnalysisReport,
    document_structure: HashMap<String, serde_json::Value>,
    metadata_extraction: HashMap<String, serde_json::Value>,
    operations: Vec<Operation>,
    updated_documentation_map: DocumentMap,
}

/// Errors that can occur during Doc Content Syncer Agent operations
#[derive(Debug, Error)]
pub enum DocContentSyncerError {
    #[error("State error: {0}")]
    StateError(String),

    #[error("Event system error: {0}")]
    EventSystemError(#[from] EventError),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Content analysis error: {0}")]
    ContentAnalysisError(String),

    #[error("Path processing error: {0}")]
    PathError(String),
}

/// Doc Content Syncer Agent analyzes documentation content and prepares synchronization operations
pub struct DocContentSyncerAgent {
    state_manager: StateManager,
    event_system: Arc<EventSystem>,
}

impl DocContentSyncerAgent {
    /// Create a new Doc Content Syncer Agent
    pub fn new(state_manager: StateManager, event_system: Arc<EventSystem>) -> Self {
        Self {
            state_manager,
            event_system,
        }
    }

    /// Initialize the agent and register event handlers
    pub fn initialize(&self) -> Result<(), DocContentSyncerError> {
        info!("Initializing Doc Content Syncer Agent");
        
        // Register event handler for content analysis
        let event_system = self.event_system.clone();
        let agent = self.clone();

        self.event_system.register_handler(DOCS_ANALYZE_CONTENT, Box::new(move |event| {
            debug!("Received DOCS_ANALYZE_CONTENT event");
            let result = agent.handle_analyze_content_event(&event);
            if let Err(e) = &result {
                error!("Error handling DOCS_ANALYZE_CONTENT event: {:?}", e);
            }
            // Convert DocContentSyncerError to EventError
            result.map_err(|e| EventError::HandlerError(format!("{:?}", e)))
        }))?;

        info!("DocContentSyncerAgent initialized and registered for events");
        Ok(())
    }

    /// Handle the DOCS_ANALYZE_CONTENT event
    fn handle_analyze_content_event(&self, event: &CrateEvent) -> Result<(), DocContentSyncerError> {
        // Parse the payload
        let payload: ContentAnalysisPayload = serde_json::from_value(event.payload().clone())
            .map_err(|e| DocContentSyncerError::InvalidPayload(format!("Failed to parse payload: {}", e)))?;

        // Analyze content
        let source_path = PathBuf::from(&payload.source_path);
        if !path_exists(&source_path) {
            return Err(DocContentSyncerError::InvalidPayload(format!(
                "Source path does not exist: {}",
                source_path.display()
            )));
        }

        let target_path = PathBuf::from(&payload.target_path);
        if !path_exists(&target_path) {
            return Err(DocContentSyncerError::InvalidPayload(format!(
                "Target path does not exist: {}",
                target_path.display()
            )));
        }

        // Perform content analysis
        info!("Analyzing content at source path: {}", source_path.display());
        let content_analysis_result = self.analyze_content(&source_path, &target_path, &payload.documentation_map)?;

        // Construct and emit response
        let response_payload = ContentAnalyzedPayload {
            doc_map: DocumentationMap {
                source_files: HashMap::new(),
                target_files: HashMap::new(),
                relationships: Vec::new(),
                metadata: content_analysis_result.updated_documentation_map.metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect(),
            },
            content_operations: content_analysis_result.operations.into_iter().map(|op| {
                ContentOperation {
                    operation_type: match op.op_type {
                        OperationType::Create => "create".to_string(),
                        OperationType::Update => "update".to_string(),
                        OperationType::Delete => "delete".to_string(),
                        OperationType::Move => "move".to_string(),
                        OperationType::Copy => "copy".to_string(),
                    },
                    source_path: op.path.clone(),
                    target_path: op.path,
                    content: op.content,
                    metadata: HashMap::new(),
                }
            }).collect(),
            analysis_report: content_analysis_result.report,
        };

        let response_event = CrateEvent::Custom { 
            name: DOCS_CONTENT_ANALYZED.to_string(),
            payload: serde_json::to_value(response_payload).map_err(|e| {
                DocContentSyncerError::EventSystemError(EventError::SerializationError(e.to_string()))
            })?,
        };

        info!("Emitting DOCS_CONTENT_ANALYZED event");
        self.event_system.emit(response_event)?;

        Ok(())
    }
    
    /// Analyze the content of a documentation directory
    fn analyze_content(
        &self,
        source_path: &Path,
        target_path: &Path,
        documentation_map: &DocumentMap,
    ) -> Result<ContentAnalysisResult, DocContentSyncerError> {
        let mut findings = Vec::new();
        let mut operations = Vec::new();
        let mut document_structure = HashMap::new();
        let mut metadata_extraction = HashMap::new();
        let mut updated_map = documentation_map.clone();

        // Find markdown files in source directory
        let markdown_files = find_files(source_path, "**/*.md")
            .map_err(|e| DocContentSyncerError::IoError(format!("Failed to find markdown files: {}", e)))?;

        info!("Found {} markdown files in source directory", markdown_files.len());

        // Process each markdown file
        for file_path in &markdown_files {
            let relative_path = get_relative_path(source_path, file_path)
                .map_err(|e| DocContentSyncerError::PathError(e.to_string()))?;

            let file_content = read_file_to_string(file_path)
                .map_err(|e| DocContentSyncerError::IoError(format!("Failed to read file: {}", e)))?;

            let content_hash = calculate_file_hash(file_path)
                .map_err(|e| DocContentSyncerError::IoError(format!("Failed to calculate hash: {}", e)))?;

            // Extract frontmatter and metadata
            let (frontmatter, content) = self.extract_frontmatter(&file_content);
            let title = self.extract_title(frontmatter.as_ref(), &content);

            // Create or update document info in the map
            let path_str = relative_path.to_string_lossy().to_string();
            let existing_doc = updated_map.documents.get(&path_str);

            let is_new = existing_doc.is_none();
            let is_updated = match existing_doc {
                Some(doc) => doc.content_hash.as_ref() != Some(&content_hash),
                None => true,
            };

            let doc_info = updated_map.documents.entry(path_str.clone()).or_insert_with(|| {
                info!("Adding new document to map: {}", path_str);
                serde_json::from_value(json!({
                    "path": file_path.to_string_lossy().to_string(),
                    "relative_path": path_str,
                    "title": title.clone(),
                    "frontmatter": frontmatter.clone(),
                    "last_modified": chrono::Utc::now().to_rfc3339(),
                    "content_hash": content_hash,
                    "status": "new"
                }))
                .unwrap()
            });

            // Update existing document if needed
            if !is_new && is_updated {
                info!("Updating existing document in map: {}", path_str);
                doc_info.title = title.clone();
                doc_info.frontmatter = frontmatter.clone();
                doc_info.last_modified = Some(chrono::Utc::now().to_rfc3339());
                doc_info.content_hash = Some(content_hash);
                doc_info.status = Some("updated".to_string());
            }

            // Add document structure information
            document_structure.insert(
                path_str.clone(),
                json!({
                    "headings": self.extract_headings(&content),
                    "links": self.extract_links(&content),
                    "images": self.extract_images(&content),
                    "code_blocks": self.extract_code_blocks(&content)
                }),
            );

            // Check for common issues and add findings
            self.check_for_issues(&path_str, &content, &mut findings);

            // Create operations for new or updated files
            if is_new || is_updated {
                // Determine target path in Docusaurus
                let target_file_path = self.determine_target_path(target_path, &relative_path);
                
                operations.push(Operation {
                    op_type: if is_new { OperationType::Create } else { OperationType::Update },
                    path: target_file_path.to_string_lossy().to_string(),
                    content: Some(file_content),
                    metadata: Some(self.create_operation_metadata(&doc_info)),
                });
            }
        }

        // Check for deleted files
        self.check_for_deleted_files(
            source_path,
            &markdown_files,
            &mut updated_map,
            &mut findings,
            &mut operations,
        )?;

        // Create metadata extraction summary
        metadata_extraction.insert(
            "counts".to_string(),
            json!({
                "total_documents": markdown_files.len(),
                "new_documents": operations.iter().filter(|op| op.op_type == OperationType::Create).count(),
                "updated_documents": operations.iter().filter(|op| op.op_type == OperationType::Update).count(),
                "deleted_documents": operations.iter().filter(|op| op.op_type == OperationType::Delete).count(),
            }),
        );

        let report = AnalysisReport {
            total_items: markdown_files.len(),
            findings,
            operations: operations.clone(),
            summary: format!("Found {} documents, {} operations to perform", markdown_files.len(), operations.len()),
        };

        Ok(ContentAnalysisResult {
            report,
            document_structure,
            metadata_extraction,
            operations,
            updated_documentation_map: updated_map,
        })
    }
    
    /// Scan a directory recursively for documentation files
    fn scan_directory(&self, dir_path: &Path) -> Result<Vec<PathBuf>, DocContentSyncerError> {
        let mut result = Vec::new();
        
        if !dir_path.exists() || !dir_path.is_dir() {
            return Err(DocContentSyncerError::PathError(
                format!("Invalid directory path: {}", dir_path.display())
            ));
        }
        
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively scan subdirectories
                let mut sub_files = self.scan_directory(&path)?;
                result.append(&mut sub_files);
            } else if self.is_documentation_file(&path) {
                // Add documentation file to result
                result.push(path);
            }
        }
        
        Ok(result)
    }
    
    /// Check if a file is a documentation file based on its extension
    fn is_documentation_file(&self, file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            return ext == "md" || ext == "mdx" || ext == "markdown";
        }
        false
    }
    
    /// Get the relative path of a file from the base directory
    fn get_relative_path(&self, file_path: &Path, base_path: &Path) -> Result<PathBuf, DocContentSyncerError> {
        match file_path.strip_prefix(base_path) {
            Ok(relative) => Ok(relative.to_path_buf()),
            Err(_) => Err(DocContentSyncerError::PathError(
                format!("Failed to get relative path for: {}", file_path.display())
            )),
        }
    }
    
    /// Calculate SHA-256 hash for content
    fn calculate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Extract metadata from file content
    fn extract_metadata(&self, content: &str, file_path: &Path) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        
        // Extract title from the first heading
        if let Some(title_line) = content.lines().find(|line| line.starts_with("# ")) {
            metadata.insert("title".to_string(), title_line[2..].trim().to_string());
        } else {
            // Use filename as title if no heading found
            if let Some(stem) = file_path.file_stem() {
                metadata.insert("title".to_string(), stem.to_string_lossy().to_string());
            }
        }
        
        // Extract description (the first paragraph after the title)
        let lines: Vec<&str> = content.lines().collect();
        let mut in_frontmatter = false;
        let mut description = String::new();
        
        // Check for frontmatter
        if content.starts_with("---") {
            in_frontmatter = true;
            
            // Parse frontmatter
            for (i, line) in lines.iter().enumerate().skip(1) {
                if *line == "---" {
                    in_frontmatter = false;
                    
                    // Process frontmatter lines
                    for j in 1..i {
                        let fm_line = lines[j];
                        if let Some(index) = fm_line.find(':') {
                            let key = fm_line[..index].trim().to_string();
                            let value = fm_line[index + 1..].trim().to_string();
                            metadata.insert(key, value);
                        }
                    }
                    
                    break;
                }
            }
        }
        
        // Find first paragraph for description if not in frontmatter
        if !in_frontmatter && !metadata.contains_key("description") {
            let mut in_paragraph = false;
            
            for line in lines {
                // Skip headings and empty lines
                if line.starts_with('#') || line.trim().is_empty() {
                    if in_paragraph {
                        break; // End of paragraph
                    }
                    continue;
                }
                
                in_paragraph = true;
                description.push_str(line.trim());
                description.push(' ');
                
                // Limit description length
                if description.len() > 150 {
                    description = description[..150].to_string();
                    description.push_str("...");
                    break;
                }
            }
            
            if !description.is_empty() {
                metadata.insert("description".to_string(), description.trim().to_string());
            }
        }
        
        // Extract tags from content
        if let Some(tags_line) = content.lines().find(|line| 
            line.to_lowercase().contains("tags:") || 
            line.to_lowercase().contains("keywords:")
        ) {
            if let Some(colon_pos) = tags_line.find(':') {
                let tags_str = tags_line[colon_pos + 1..].trim();
                metadata.insert("tags".to_string(), tags_str.to_string());
            }
        }
        
        metadata
    }
    
    /// Determine the target path for a documentation file
    fn determine_target_path(&self, target_base: &Path, relative_path: &Path) -> PathBuf {
        // Convert from *.md to corresponding Docusaurus paths
        // For example, docs/guide/installation.md → website/docs/guide/installation.mdx
        
        // Extract components
        let file_name = relative_path.file_stem().unwrap_or_default();
        let parent_path = relative_path.parent().unwrap_or_else(|| Path::new(""));
        
        // Create target path in Docusaurus docs directory
        target_base
            .join("docs")
            .join(parent_path)
            .join(format!("{}.mdx", file_name.to_string_lossy()))
    }
    
    /// Analyze references in content
    fn analyze_references(
        &self, 
        content: &str, 
        file_path: &Path, 
        doc_map: &mut DocumentMap,
        findings: &mut Vec<Finding>
    ) {
        // Find markdown links [text](link)
        let link_regex = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        
        for capture in link_regex.captures_iter(content) {
            if let (Some(text_match), Some(link_match)) = (capture.get(1), capture.get(2)) {
                let text = text_match.as_str();
                let link_url = link_match.as_str();
                
                // Skip external links
                if link_url.starts_with("http://") || link_url.starts_with("https://") {
                    continue;
                }
                
                // Handle relative links to other documentation files
                if link_url.ends_with(".md") {
                    let source_path = file_path.to_string_lossy().to_string();
                    let link_target = match Path::new(link_url).file_name() {
                        Some(name) => name.to_string_lossy().to_string(),
                        None => continue,
                    };
                    
                    // Add finding if link target doesn't exist in the documents map
                    if !doc_map.documents.contains_key(&link_target) {
                        findings.push(Finding {
                            category: "Broken Links".to_string(),
                            severity: Severity::Medium,
                            message: format!("Potentially broken link to '{}' in '{}'", link_url, source_path),
                            file_path: Some(source_path),
                            line_number: None,
                            recommendation: Some(format!("Check if the target file '{}' exists", link_target)),
                        });
                    }
                }
            }
        }
    }

    fn extract_frontmatter(
        &self,
        content: &str,
    ) -> (Option<HashMap<String, serde_json::Value>>, String) {
        // Simple frontmatter extraction (basic implementation)
        if content.starts_with("---") {
            if let Some(end_idx) = content.find("---\n") {
                let frontmatter_text = &content[3..end_idx].trim();
                let remaining_content = &content[end_idx + 4..];
                
                // Parse YAML frontmatter (this is a simplified version)
                let mut frontmatter = HashMap::new();
                for line in frontmatter_text.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        frontmatter.insert(key.trim().to_string(), json!(value.trim()));
                    }
                }
                
                return (Some(frontmatter), remaining_content.to_string());
            }
        }
        
        (None, content.to_string())
    }

    fn extract_title(
        &self,
        frontmatter: Option<&HashMap<String, serde_json::Value>>,
        content: &str,
    ) -> Option<String> {
        // Try to get title from frontmatter
        if let Some(fm) = frontmatter {
            if let Some(title) = fm.get("title") {
                if let Some(title_str) = title.as_str() {
                    return Some(title_str.to_string());
                }
            }
        }
        
        // Try to extract H1 heading
        for line in content.lines() {
            if line.starts_with("# ") {
                return Some(line[2..].trim().to_string());
            }
        }
        
        None
    }

    fn extract_headings(&self, content: &str) -> Vec<serde_json::Value> {
        let mut headings = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("#") {
                let level = line.chars().take_while(|&c| c == '#').count();
                let text = line[level..].trim().to_string();
                
                headings.push(json!({
                    "level": level,
                    "text": text
                }));
            }
        }
        
        headings
    }

    fn extract_links(&self, content: &str) -> Vec<serde_json::Value> {
        let mut links = Vec::new();
        let mut current_pos = 0;
        
        while let Some(start_idx) = content[current_pos..].find('[') {
            let start = current_pos + start_idx;
            
            if let Some(text_end) = content[start..].find(']') {
                let text_end = start + text_end;
                
                if text_end + 1 < content.len() && content.chars().nth(text_end + 1) == Some('(') {
                    if let Some(url_end) = content[text_end..].find(')') {
                        let url_end = text_end + url_end;
                        
                        let text = &content[start + 1..text_end];
                        let url = &content[text_end + 2..url_end];
                        
                        links.push(json!({
                            "text": text,
                            "url": url,
                            "is_external": url.starts_with("http")
                        }));
                        
                        current_pos = url_end + 1;
                        continue;
                    }
                }
            }
            
            current_pos = start + 1;
        }
        
        links
    }

    fn extract_images(&self, content: &str) -> Vec<serde_json::Value> {
        let mut images = Vec::new();
        let mut current_pos = 0;
        
        while let Some(start_idx) = content[current_pos..].find('!') {
            let start = current_pos + start_idx;
            
            if start + 1 < content.len() && content.chars().nth(start + 1) == Some('[') {
                if let Some(alt_end) = content[start..].find(']') {
                    let alt_end = start + alt_end;
                    
                    if alt_end + 1 < content.len() && content.chars().nth(alt_end + 1) == Some('(') {
                        if let Some(url_end) = content[alt_end..].find(')') {
                            let url_end = alt_end + url_end;
                            
                            let alt = &content[start + 2..alt_end];
                            let url = &content[alt_end + 2..url_end];
                            
                            images.push(json!({
                                "alt": alt,
                                "url": url
                            }));
                            
                            current_pos = url_end + 1;
                            continue;
                        }
                    }
                }
            }
            
            current_pos = start + 1;
        }
        
        images
    }

    fn extract_code_blocks(&self, content: &str) -> Vec<serde_json::Value> {
        let mut code_blocks = Vec::new();
        let mut current_pos = 0;
        
        while let Some(start_idx) = content[current_pos..].find("```") {
            let start = current_pos + start_idx;
            
            if let Some(end_idx) = content[start + 3..].find("```") {
                let end = start + 3 + end_idx;
                
                let first_line_end = content[start..].find('\n').unwrap_or(0) + start;
                let language = content[start + 3..first_line_end].trim();
                let code = &content[first_line_end + 1..end].trim();
                
                code_blocks.push(json!({
                    "language": language,
                    "code": code
                }));
                
                current_pos = end + 3;
            } else {
                break;
            }
        }
        
        code_blocks
    }

    fn check_for_issues(
        &self,
        file_path: &str,
        content: &str,
        findings: &mut Vec<Finding>,
    ) {
        // Check for missing title
        if !content.contains("# ") {
            findings.push(Finding {
                category: "content_structure".to_string(),
                severity: Severity::Medium,
                message: "Document is missing a top-level heading (H1)".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: None,
                recommendation: Some("Add a top-level heading (# Title) to improve document structure".to_string()),
            });
        }
        
        // Check for very short content
        if content.len() < 100 {
            findings.push(Finding {
                category: "content_quality".to_string(),
                severity: Severity::Low,
                message: "Document has very little content".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: None,
                recommendation: Some("Consider expanding the content to provide more information".to_string()),
            });
        }
        
        // Check for broken internal links
        for link in self.extract_links(content) {
            if let Some(url) = link.get("url").and_then(|u| u.as_str()) {
                if !url.starts_with("http") && url.contains(".md") {
                    findings.push(Finding {
                        category: "broken_links".to_string(),
                        severity: Severity::Medium,
                        message: format!("Potential internal link to .md file found: {}", url),
                        file_path: Some(file_path.to_string()),
                        line_number: None,
                        recommendation: Some(
                            "In Docusaurus, internal links should not include .md extensions".to_string(),
                        ),
                    });
                }
            }
        }
        
        // Check for very long paragraphs
        for paragraph in content.split("\n\n") {
            if paragraph.len() > 1000 {
                findings.push(Finding {
                    category: "readability".to_string(),
                    severity: Severity::Low,
                    message: "Document contains very long paragraphs".to_string(),
                    file_path: Some(file_path.to_string()),
                    line_number: None,
                    recommendation: Some("Consider breaking long paragraphs into smaller chunks for better readability".to_string()),
                });
                break;
            }
        }
    }

    fn create_operation_metadata(
        &self,
        doc_info: &DocumentInfo,
    ) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();
        
        // Extract fields from DocumentInfo into HashMap
        if let Some(ref title) = doc_info.title {
            metadata.insert("title".to_string(), json!(title));
        }
        
        if let Some(ref content_hash) = doc_info.content_hash {
            metadata.insert("content_hash".to_string(), json!(content_hash));
        }
        
        if let Some(ref status) = doc_info.status {
            metadata.insert("status".to_string(), json!(status));
        }
        
        // Add metadata fields
        metadata.insert("last_modified".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        metadata.insert("synced_by".to_string(), json!("doc-content-syncer"));
        
        metadata
    }

    fn check_for_deleted_files(
        &self,
        source_path: &Path,
        existing_files: &[PathBuf],
        updated_map: &mut DocumentMap,
        findings: &mut Vec<Finding>,
        operations: &mut Vec<Operation>,
    ) -> Result<(), DocContentSyncerError> {
        // Convert existing files to relative paths for comparison
        let existing_relative: Vec<PathBuf> = existing_files
            .iter()
            .filter_map(|path| {
                get_relative_path(source_path, path)
                    .map_err(|e| {
                        error!("Error getting relative path: {}", e);
                        e
                    })
                    .ok()
            })
            .collect();

        // Check each document in the map
        let mut deleted_paths = Vec::new();
        
        for (path, doc) in &mut updated_map.documents {
            let doc_path = PathBuf::from(path);
            
            if !existing_relative.contains(&doc_path) {
                info!("Detected deleted file: {}", path);
                deleted_paths.push(path.clone());
                
                // Mark as deleted in the map
                doc.status = Some("deleted".to_string());
                
                // Add finding
                findings.push(Finding {
                    category: "deleted_content".to_string(),
                    severity: Severity::Info,
                    message: format!("Document has been deleted from source: {}", path),
                    file_path: Some(path.clone()),
                    line_number: None,
                    recommendation: None,
                });
                
                // Add delete operation
                operations.push(Operation {
                    op_type: OperationType::Delete,
                    path: self.determine_target_path(
                        &PathBuf::from(&updated_map.target_path),
                        &PathBuf::from(path),
                    ).to_string_lossy().to_string(),
                    content: None,
                    metadata: Some({
                        let mut metadata = HashMap::new();
                        metadata.insert("deleted".to_string(), json!(true));
                        metadata.insert("original_path".to_string(), json!(path));
                        metadata
                    }),
                });
            }
        }
        
        Ok(())
    }
}

impl Clone for DocContentSyncerAgent {
    fn clone(&self) -> Self {
        Self {
            state_manager: self.state_manager.clone(),
            event_system: Arc::clone(&self.event_system),
        }
    }
}

/// Add error conversion for io::Error
impl From<std::io::Error> for DocContentSyncerError {
    fn from(error: std::io::Error) -> Self {
        DocContentSyncerError::IoError(error.to_string())
    }
} 