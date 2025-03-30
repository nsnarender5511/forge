use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fmt;

pub mod doc_sync_state;

impl fmt::Debug for StateManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateManager")
            .field("state_count", &self.state.lock().unwrap().len())
            .finish()
    }
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.state.lock().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: &str, value: serde_json::Value) {
        self.state.lock().unwrap().insert(key.to_string(), value);
    }

    pub fn remove(&self, key: &str) -> Option<serde_json::Value> {
        self.state.lock().unwrap().remove(key)
    }

    pub fn clear(&self) {
        self.state.lock().unwrap().clear();
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct StateManager {
    state: Arc<Mutex<HashMap<String, serde_json::Value>>>,
} 