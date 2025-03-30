use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// Event constants for document syncing
pub const DOCS: &str = "docs";
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

pub mod doc_sync_events;

#[derive(Debug, Error)]
pub enum EventError {
    #[error("Failed to register event handler: {0}")]
    HandlerRegistrationError(String),
    
    #[error("Failed to emit event: {0}")]
    EmitError(String),
    
    #[error("Event handler error: {0}")]
    HandlerError(String),
    
    #[error("Invalid event payload: {0}")]
    InvalidPayload(String),
    
    #[error("State error: {0}")]
    StateError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[derive(Debug, Clone)]
pub enum Event {
    DocSync(doc_sync_events::DocSyncEvent),
    Custom { name: String, payload: serde_json::Value },
}

impl Event {
    pub fn name(&self) -> String {
        match self {
            Event::DocSync(event) => format!("doc_sync.{}", event.event_type),
            Event::Custom { name, .. } => name.clone(),
        }
    }
    
    pub fn payload(&self) -> serde_json::Value {
        match self {
            Event::DocSync(event) => serde_json::to_value(event).unwrap_or_default(),
            Event::Custom { payload, .. } => payload.clone(),
        }
    }
}

pub type EventHandler = Box<dyn Fn(Event) -> Result<(), EventError> + Send + Sync>;

pub struct EventSystem {
    handlers: Arc<Mutex<HashMap<String, Vec<EventHandler>>>>,
}

impl EventSystem {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_handler(&self, event_name: &str, handler: EventHandler) -> Result<(), EventError> {
        let mut handlers = self.handlers.lock().map_err(|e| {
            EventError::HandlerRegistrationError(format!("Failed to lock handlers: {}", e))
        })?;
        
        handlers
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
            
        Ok(())
    }

    pub fn emit(&self, event: Event) -> Result<(), EventError> {
        let handlers = self.handlers.lock().map_err(|e| {
            EventError::EmitError(format!("Failed to lock handlers: {}", e))
        })?;
        
        let event_name = event.name();
        if let Some(event_handlers) = handlers.get(&event_name) {
            for handler in event_handlers {
                handler(event.clone()).map_err(|e| {
                    EventError::HandlerError(format!("Handler error for event {}: {}", event_name, e))
                })?;
            }
        }
        
        Ok(())
    }
}

impl Default for EventSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventSystem {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
        }
    }
} 