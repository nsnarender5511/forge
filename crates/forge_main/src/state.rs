use std::fmt::Display;

use forge_api::{ConversationId, Event, Usage};

use crate::input::PromptInput;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Act,
    Plan,
    Help,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Act => write!(f, "act"),
            Mode::Plan => write!(f, "plan"),
            Mode::Help => write!(f, "help"),
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::Act
    }
}

/// State information for the UI
#[derive(Debug)]
pub struct UIState {
    pub mode: Mode,
    pub is_first: bool,
    pub current_title: Option<String>,
    pub current_agent: Option<String>,
    pub conversation_id: Option<ConversationId>,
    pub usage: Usage,
    pub last_event: Option<Event>,
    pub progress_tracking_error_details: Option<String>,
    pub line_buffer: String,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            is_first: true,
            current_title: None,
            current_agent: None,
            conversation_id: None,
            usage: Default::default(),
            last_event: None,
            progress_tracking_error_details: None,
            line_buffer: String::new(),
        }
    }
}

impl From<&UIState> for PromptInput {
    fn from(state: &UIState) -> Self {
        PromptInput::Update {
            title: state.current_title.clone(),
            usage: Some(state.usage.clone()),
            mode: state.mode.clone(),
        }
    }
}
