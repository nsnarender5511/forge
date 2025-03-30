use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use thiserror::Error;

use crate::events::doc_sync_events::{DocumentationMap, QualityMetrics};

/// Errors that can occur during state operations
#[derive(Debug, Error)]
pub enum StateError {
    #[error("File IO error: {0}")]
    FileIo(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    #[error("State lock error: {0}")]
    LockError(String),
    #[error("State not found: {0}")]
    NotFound(String),
    #[error("Lock timeout: {0}")]
    LockTimeout(String),
}

/// DocSyncState represents the complete state of the synchronization process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSyncState {
    pub master_doc_map: DocumentationMap,
    pub pending_tasks: Vec<SyncTask>,
    pub completed_tasks: Vec<SyncTask>,
    pub error_log: Vec<SyncError>,
    pub quality_scores: Option<QualityMetrics>,
    pub metadata: HashMap<String, String>,
    pub last_updated: u64,
    // Iteration tracking for feedback loop
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub iteration_history: Vec<IterationState>,
    pub requirements_satisfied: HashMap<String, bool>,
}

/// IterationState represents the state at a specific iteration of the feedback loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationState {
    pub iteration_number: u32,
    pub timestamp: u64,
    pub quality_scores: Option<QualityMetrics>,
    pub verification_results: HashMap<String, bool>,
    pub tasks_completed: usize,
    pub tasks_pending: usize,
    pub issues_found: usize,
    pub issues_resolved: usize,
}

impl Default for DocSyncState {
    fn default() -> Self {
        Self {
            master_doc_map: DocumentationMap::default(),
            pending_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            error_log: Vec::new(),
            quality_scores: None,
            metadata: HashMap::new(),
            last_updated: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            // Initialize feedback loop fields
            current_iteration: 1,
            max_iterations: 5, // Default max iterations to prevent infinite loops
            iteration_history: Vec::new(),
            requirements_satisfied: HashMap::new(),
        }
    }
}

/// SyncTask represents a task to be performed during synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub task_id: String,
    pub task_type: String,
    pub status: String, // "pending", "in_progress", "completed", "failed"
    pub assigned_to: String,
    pub parameters: HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// SyncError represents an error that occurred during synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    pub error_type: String,
    pub description: String,
    pub related_task_id: Option<String>,
    pub related_paths: Vec<String>,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

/// StateManager provides an interface for reading and writing the document synchronization state
pub struct StateManager {
    state_dir: PathBuf,
    locks: Arc<Mutex<HashMap<String, SystemTime>>>,
    lock_timeout: Duration,
}

impl StateManager {
    /// Create a new StateManager with the specified state directory
    pub fn new(state_dir: &str) -> Result<Self, StateError> {
        let path = PathBuf::from(state_dir);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        
        Ok(Self {
            state_dir: path,
            locks: Arc::new(Mutex::new(HashMap::new())),
            lock_timeout: Duration::from_secs(60), // Default 60-second lock timeout
        })
    }
    
    /// Set the lock timeout
    pub fn set_lock_timeout(&mut self, seconds: u64) {
        self.lock_timeout = Duration::from_secs(seconds);
    }
    
    /// Read the state for a specific sync process
    pub fn read_state(&self, correlation_id: &str) -> Result<DocSyncState, StateError> {
        let file_path = self.get_state_file_path(correlation_id);
        
        if !file_path.exists() {
            return Err(StateError::NotFound(correlation_id.to_string()));
        }
        
        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let state: DocSyncState = serde_json::from_str(&contents)?;
        Ok(state)
    }
    
    /// Write the state for a specific sync process
    pub fn write_state(&self, correlation_id: &str, state: &DocSyncState) -> Result<(), StateError> {
        let file_path = self.get_state_file_path(correlation_id);
        let dir_path = file_path.parent().unwrap();
        
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
        }
        
        let state_json = serde_json::to_string_pretty(&state)?;
        let mut file = File::create(file_path)?;
        file.write_all(state_json.as_bytes())?;
        
        Ok(())
    }
    
    /// Update a specific part of the state using a callback function
    pub fn update_state<F>(&self, correlation_id: &str, update_fn: F) -> Result<(), StateError>
    where
        F: FnOnce(&mut DocSyncState),
    {
        self.lock_state(correlation_id)?;
        
        let mut state = match self.read_state(correlation_id) {
            Ok(state) => state,
            Err(StateError::NotFound(_)) => DocSyncState::default(),
            Err(e) => {
                self.unlock_state(correlation_id)?;
                return Err(e);
            }
        };
        
        update_fn(&mut state);
        
        state.last_updated = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let result = self.write_state(correlation_id, &state);
        self.unlock_state(correlation_id)?;
        
        result
    }
    
    /// Lock the state for a specific sync process
    pub fn lock_state(&self, correlation_id: &str) -> Result<(), StateError> {
        let mut locks = self.locks.lock().map_err(|e| {
            StateError::LockError(format!("Failed to acquire lock mutex: {}", e))
        })?;
        
        // Check if the lock exists and is still valid
        if let Some(lock_time) = locks.get(correlation_id) {
            let now = SystemTime::now();
            if now.duration_since(*lock_time).unwrap() < self.lock_timeout {
                return Err(StateError::LockError(format!(
                    "State is already locked for correlation ID: {}", 
                    correlation_id
                )));
            }
            // Lock has expired, we can take it
        }
        
        // Acquire the lock
        locks.insert(correlation_id.to_string(), SystemTime::now());
        
        Ok(())
    }
    
    /// Unlock the state for a specific sync process
    pub fn unlock_state(&self, correlation_id: &str) -> Result<(), StateError> {
        let mut locks = self.locks.lock().map_err(|e| {
            StateError::LockError(format!("Failed to acquire lock mutex: {}", e))
        })?;
        
        locks.remove(correlation_id);
        
        Ok(())
    }
    
    /// List all available sync states
    pub fn list_states(&self) -> Result<Vec<String>, StateError> {
        let mut states = Vec::new();
        
        for entry in fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".json") {
                        states.push(file_name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
        
        Ok(states)
    }
    
    /// Delete the state for a specific sync process
    pub fn delete_state(&self, correlation_id: &str) -> Result<(), StateError> {
        let file_path = self.get_state_file_path(correlation_id);
        
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }
        
        Ok(())
    }
    
    /// Get the file path for a specific sync process state
    fn get_state_file_path(&self, correlation_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}.json", correlation_id))
    }
    
    /// Cleanup expired locks
    pub fn cleanup_expired_locks(&self) -> Result<usize, StateError> {
        let mut locks = self.locks.lock().map_err(|e| {
            StateError::LockError(format!("Failed to acquire lock mutex: {}", e))
        })?;
        
        let now = SystemTime::now();
        let expired_keys: Vec<String> = locks
            .iter()
            .filter(|(_, lock_time)| {
                now.duration_since(**lock_time).unwrap() >= self.lock_timeout
            })
            .map(|(key, _)| key.clone())
            .collect();
        
        let count = expired_keys.len();
        
        for key in expired_keys {
            locks.remove(&key);
        }
        
        Ok(count)
    }
} 