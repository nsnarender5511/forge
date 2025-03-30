use std::sync::Arc;

use anyhow::Result;
use colored::Colorize;
use forge_api::{AgentMessage, ChatRequest, ChatResponse, ConversationId, Event, Model, API};
use forge_display::TitleFormat;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::error;

use crate::banner;
use crate::cli::Cli;
use crate::console::CONSOLE;
use crate::info::Info;
use crate::input::Console;
use crate::model::{Command, ForgeCommandManager, UserInput};
use crate::state::{Mode, UIState};

// Event type constants moved to UI layer
pub const EVENT_USER_TASK_INIT: &str = "user_task_init";
pub const EVENT_USER_TASK_UPDATE: &str = "user_task_update";
pub const EVENT_USER_HELP_QUERY: &str = "user_help_query";
pub const EVENT_TITLE: &str = "title";

lazy_static! {
    pub static ref TRACKER: forge_tracker::Tracker = forge_tracker::Tracker::default();
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct PartialEvent {
    pub name: String,
    pub value: Value,
}

impl PartialEvent {
    pub fn new<V: Into<Value>>(name: impl ToString, value: V) -> Self {
        Self { name: name.to_string(), value: value.into() }
    }
}

impl From<PartialEvent> for Event {
    fn from(value: PartialEvent) -> Self {
        Event::new(value.name, value.value)
    }
}

pub struct UI<F> {
    state: UIState,
    api: Arc<F>,
    console: Console,
    command: Arc<ForgeCommandManager>,
    cli: Cli,
    models: Option<Vec<Model>>,
    #[allow(dead_code)] // The guard is kept alive by being held in the struct
    _guard: forge_tracker::Guard,
}

impl<F: API> UI<F> {
    // Helper method to flush any buffered incomplete line
    fn flush_line_buffer(&mut self, agent_id: &str) -> Result<()> {
        if !self.state.line_buffer.is_empty() {
            let agent_prefix = format!("[{}] ", agent_id.blue().bold());
            CONSOLE.write(agent_prefix)?;
            CONSOLE.write(self.state.line_buffer.dimmed().to_string())?;
            CONSOLE.newline()?;
            self.state.line_buffer.clear();
        }
        Ok(())
    }

    // Set the current mode and update conversation variable
    async fn handle_mode_change(&mut self, mode: Mode) -> Result<()> {
        // Update the mode in state
        self.state.mode = mode;

        // Show message that mode changed
        let mode_str = self.state.mode.to_string();

        // Set the mode variable in the conversation if a conversation exists
        let conversation_id = self.init_conversation().await?;
        self.api
            .set_variable(
                &conversation_id,
                "mode".to_string(),
                Value::from(mode_str.as_str()),
            )
            .await?;

        // Print a mode-specific message
        let mode_message = match self.state.mode {
            Mode::Act => "mode - executes commands and makes file changes",
            Mode::Plan => "mode - plans actions without making changes",
            Mode::Help => "mode - answers questions (type /act or /plan to switch back)",
        };

        CONSOLE.write(
            TitleFormat::success(&mode_str)
                .sub_title(mode_message)
                .format(),
        )?;

        Ok(())
    }
    // Helper functions for creating events with the specific event names
    fn create_task_init_event<V: Into<Value>>(content: V) -> Event {
        Event::new(EVENT_USER_TASK_INIT, content)
    }

    fn create_task_update_event<V: Into<Value>>(content: V) -> Event {
        Event::new(EVENT_USER_TASK_UPDATE, content)
    }
    fn create_user_help_query_event<V: Into<Value>>(content: V) -> Event {
        Event::new(EVENT_USER_HELP_QUERY, content)
    }

    pub fn init(cli: Cli, api: Arc<F>) -> Result<Self> {
        // Parse CLI arguments first to get flags
        let env = api.environment();
        let command = Arc::new(ForgeCommandManager::default());
        Ok(Self {
            state: Default::default(),
            api,
            console: Console::new(env.clone(), command.clone()),
            cli,
            command,
            models: None,
            _guard: forge_tracker::init_tracing(env.log_path())?,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Check for dispatch flag first
        if let Some(dispatch_json) = self.cli.event.clone() {
            return self.handle_dispatch(dispatch_json).await;
        }

        // Handle direct prompt if provided
        let prompt = self.cli.prompt.clone();
        if let Some(prompt) = prompt {
            self.chat(prompt).await?;
            return Ok(());
        }

        // Display the banner in dimmed colors since we're in interactive mode
        self.init_conversation().await?;
        banner::display(self.command.command_names())?;

        // Get initial input from file or prompt
        let mut input = match &self.cli.command {
            Some(path) => self.console.upload(path).await?,
            None => self.console.prompt(None).await?,
        };

        loop {
            match input {
                Command::Dump => {
                    self.handle_dump().await?;
                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                    continue;
                }
                Command::New => {
                    self.state = Default::default();
                    self.init_conversation().await?;
                    banner::display(self.command.command_names())?;
                    input = self.console.prompt(None).await?;

                    continue;
                }
                Command::Info => {
                    let info =
                        Info::from(&self.api.environment()).extend(Info::from(&self.state.usage));

                    CONSOLE.writeln(info.to_string())?;

                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                    continue;
                }
                Command::Docs(_) => {
                    // This should never be reached since we're redirecting /docs to Custom command
                    // But we need it for exhaustive pattern matching
                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                    continue;
                },
                Command::Message(ref content) => {
                    let chat_result = match self.state.mode {
                        Mode::Help => {
                            self.dispatch_event(Self::create_user_help_query_event(content.clone()))
                                .await
                        }
                        _ => self.chat(content.clone()).await,
                    };
                    if let Err(err) = chat_result {
                        tokio::spawn(
                            TRACKER.dispatch(forge_tracker::EventKind::Error(format!("{:?}", err))),
                        );
                        error!(error = ?err, "Chat request failed");

                        CONSOLE.writeln(TitleFormat::failed(format!("{:?}", err)).format())?;
                    }
                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                }
                Command::Act => {
                    self.handle_mode_change(Mode::Act).await?;

                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                    continue;
                }
                Command::Plan => {
                    self.handle_mode_change(Mode::Plan).await?;

                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                    continue;
                }
                Command::Help => {
                    self.handle_mode_change(Mode::Help).await?;

                    let prompt_input = Some((&self.state).into());
                    input = self.console.prompt(prompt_input).await?;
                    continue;
                }
                Command::Exit => {
                    break;
                }
                Command::Models => {
                    let models = if let Some(models) = self.models.as_ref() {
                        models
                    } else {
                        let models = self.api.models().await?;
                        self.models = Some(models);
                        self.models.as_ref().unwrap()
                    };
                    let info: Info = models.as_slice().into();
                    CONSOLE.writeln(info.to_string())?;

                    input = self.console.prompt(None).await?;
                }
                Command::Custom(event) => {
                    if let Err(e) = self.dispatch_event(event.into()).await {
                        CONSOLE.writeln(
                            TitleFormat::failed("Failed to execute the command.")
                                .sub_title("Command Execution")
                                .error(e.to_string())
                                .format(),
                        )?;
                    }

                    input = self.console.prompt(None).await?;
                }
            }
        }

        Ok(())
    }

    // Handle dispatching events from the CLI
    async fn handle_dispatch(&mut self, json: String) -> Result<()> {
        // Initialize the conversation
        let conversation_id = self.init_conversation().await?;

        // Parse the JSON to determine the event name and value
        let event: PartialEvent = serde_json::from_str(&json)?;

        // Create the chat request with the event
        let chat = ChatRequest::new(event.into(), conversation_id);

        // Process the event
        let mut stream = self.api.chat(chat).await?;
        self.handle_chat_stream(&mut stream).await
    }

    async fn init_conversation(&mut self) -> Result<ConversationId> {
        match self.state.conversation_id {
            Some(ref id) => Ok(id.clone()),
            None => {
                let workflow = self.api.load(self.cli.workflow.as_deref()).await?;
                self.command.register_all(&workflow);
                let conversation_id = self.api.init(workflow).await?;
                self.state.conversation_id = Some(conversation_id.clone());

                Ok(conversation_id)
            }
        }
    }

    async fn chat(&mut self, content: String) -> Result<()> {
        let conversation_id = self.init_conversation().await?;

        // Create a ChatRequest with the appropriate event type
        let event = if self.state.is_first {
            self.state.is_first = false;
            Self::create_task_init_event(content.clone())
        } else {
            Self::create_task_update_event(content.clone())
        };

        // Create the chat request with the event
        let chat = ChatRequest::new(event, conversation_id);

        match self.api.chat(chat).await {
            Ok(mut stream) => self.handle_chat_stream(&mut stream).await,
            Err(err) => Err(err),
        }
    }

    async fn handle_chat_stream(
        &mut self,
        stream: &mut (impl StreamExt<Item = Result<AgentMessage<ChatResponse>>> + Unpin),
    ) -> Result<()> {
        // Check if this is a docs synchronization operation based on conversation state
        // This is determined by examining the conversation's events to see if any have the "docs" name,
        // which indicates this conversation is handling a document synchronization operation
        let mut is_docs_sync = false;
        let mut progress_tracking_error = false;
        
        if let Some(conversation_id) = &self.state.conversation_id {
            // First, retrieve the current conversation using its ID
            match self.api.conversation(conversation_id).await {
                Ok(Some(conversation)) => {
                    // Then check if any events in this conversation are "docs" events
                    // If at least one "docs" event is found, this is a document sync operation
                    is_docs_sync = conversation.events.iter().any(|e| e.name == "docs");
                    self.state.progress_tracking_error_details = None; // Clear any previous error
                },
                Ok(None) => {
                    // Conversation ID exists but conversation was not found
                    // This is unexpected but we'll handle it gracefully
                    tracing::warn!("Conversation with ID {} not found", conversation_id);
                    progress_tracking_error = true;
                    self.state.progress_tracking_error_details = Some("conversation not found".to_string());
                    
                    // Fallback mechanism - check current message context
                    if let Some(last_event) = self.state.last_event.as_ref() {
                        is_docs_sync = last_event.name == "docs";
                    }
                },
                Err(err) => {
                    let error_msg = format!("{}", err);
                    tracing::error!("Error retrieving conversation {}: {}", conversation_id, err);
                    progress_tracking_error = true;
                    self.state.progress_tracking_error_details = Some(error_msg);
                    
                    // Fallback mechanism - check current message context
                    if let Some(last_event) = self.state.last_event.as_ref() {
                        is_docs_sync = last_event.name == "docs";
                    }
                }
            }
            
            // Inform user if progress tracking might be affected
            if progress_tracking_error && is_docs_sync {
                let error_message = if let Some(error_details) = &self.state.progress_tracking_error_details {
                    format!("Note: Progress tracking may be limited: {}", error_details)
                } else {
                    "Note: Progress tracking may be limited due to conversation retrieval issues.".to_string()
                };
                
                CONSOLE.writeln(error_message.dimmed().to_string())?;
            }
        }
            
        // Progress tracking variables for docs sync operations
        // These help provide feedback to the user during potentially long-running doc sync processes
        let mut tool_call_count = 0;
        
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    return Ok(());
                }
                maybe_message = stream.next() => {
                    match maybe_message {
                        Some(Ok(message)) => {
                            // For docs sync operations, provide additional feedback
                            if is_docs_sync {
                                match &message.message {
                                    ChatResponse::ToolCallStart(_) => {
                                        tool_call_count += 1;
                                        if tool_call_count % 5 == 0 {
                                            // Show occasional progress updates
                                            CONSOLE.writeln(
                                                format!("Still analyzing documentation... (processed {} operations)", 
                                                tool_call_count).dimmed().to_string()
                                            )?;
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            
                            self.handle_chat_response(message)?
                        },
                        Some(Err(err)) => {
                            return Err(err);
                        }
                        None => return Ok(()),
                    }
                }
            }
        }
    }

    async fn handle_dump(&mut self) -> Result<()> {
        if let Some(conversation_id) = self.state.conversation_id.clone() {
            let conversation = self.api.conversation(&conversation_id).await?;
            if let Some(conversation) = conversation {
                let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                let path = self
                    .state
                    .current_title
                    .as_ref()
                    .map_or(format!("{timestamp}"), |title| {
                        format!("{timestamp}-{title}")
                    });

                let path = format!("{path}-dump.json");

                let content = serde_json::to_string_pretty(&conversation)?;
                tokio::fs::write(path.as_str(), content).await?;

                CONSOLE.writeln(
                    TitleFormat::success("dump")
                        .sub_title(format!("path: {path}"))
                        .format(),
                )?;
            } else {
                CONSOLE.writeln(
                    TitleFormat::failed("dump")
                        .error("conversation not found")
                        .sub_title(format!("conversation_id: {conversation_id}"))
                        .format(),
                )?;
            }
        }
        Ok(())
    }

    fn handle_chat_response(&mut self, message: AgentMessage<ChatResponse>) -> Result<()> {
        // Update the current agent ID in the state
        self.state.current_agent = Some(message.agent.as_str().to_string());
        
        match message.message {
            ChatResponse::Text(text) => {
                // Append new text to any existing buffered text
                self.state.line_buffer.push_str(&text);
                
                // Split the buffer by newlines to identify complete lines
                let lines: Vec<&str> = self.state.line_buffer.split('\n').collect();
                
                // The last line might be incomplete unless the buffer ends with a newline
                let is_complete_line = self.state.line_buffer.ends_with('\n');
                
                // Process all lines except potentially the last one (which may be incomplete)
                let lines_to_process = if is_complete_line { lines.len() } else { lines.len() - 1 };
                
                for i in 0..lines_to_process {
                    let line = lines[i];
                    if !line.is_empty() {
                        // Show the agent name prefix only at the start of each complete line
                        let agent_prefix = format!("[{}] ", message.agent.as_str().blue().bold());
                        CONSOLE.write(agent_prefix)?;
                        CONSOLE.write(line.dimmed().to_string())?;
                    }
                    // Always add a newline after a complete line
                    CONSOLE.newline()?;
                }
                
                // Clear the processed lines from the buffer
                if is_complete_line {
                    // If the last line is complete (ends with newline), clear the entire buffer
                    self.state.line_buffer.clear();
                } else if lines_to_process > 0 {
                    // Otherwise keep the incomplete last line in the buffer
                    self.state.line_buffer = lines[lines_to_process].to_string();
                }
            },
            ChatResponse::ToolCallStart(_) => {
                // Ensure any buffered incomplete line is flushed before tool calls
                self.flush_line_buffer(message.agent.as_str())?;
                
                CONSOLE.newline()?;
                CONSOLE.newline()?;
            }
            ChatResponse::ToolCallEnd(tool_result) => {
                // Ensure any buffered incomplete line is flushed before tool results
                self.flush_line_buffer(message.agent.as_str())?;
                
                if !self.cli.verbose {
                    return Ok(());
                }

                let tool_name = tool_result.name.as_str();
                
                // Show which agent is executing the tool
                let agent_prefix = format!("[{}] ", self.state.current_agent.as_ref().unwrap_or(&"unknown".to_string())).blue().bold();

                CONSOLE.writeln(format!("{}", tool_result.content.dimmed()))?;

                if tool_result.is_error {
                    CONSOLE.writeln(format!("{}{}", agent_prefix, TitleFormat::failed(tool_name).format()))?;
                } else {
                    CONSOLE.writeln(format!("{}{}", agent_prefix, TitleFormat::success(tool_name).format()))?;
                }
            }
            ChatResponse::Event(event) => {
                // Ensure any buffered incomplete line is flushed before events
                self.flush_line_buffer(message.agent.as_str())?;
                
                if event.name == EVENT_TITLE {
                    self.state.current_title = Some(event.value.to_string());
                }
            }
            ChatResponse::Usage(u) => {
                // Ensure any buffered incomplete line is flushed before usage info
                self.flush_line_buffer(message.agent.as_str())?;
                
                self.state.usage = u;
            }
        }
        Ok(())
    }

    async fn dispatch_event(&mut self, event: Event) -> Result<()> {
        let conversation_id = self.init_conversation().await?;
        let chat = ChatRequest::new(event.clone(), conversation_id);
        // Store the event for potential fallback usage
        self.state.last_event = Some(event.clone());
        match self.api.chat(chat).await {
            Ok(mut stream) => self.handle_chat_stream(&mut stream).await,
            Err(err) => Err(anyhow::anyhow!("Failed to dispatch event {}: {}", event.name, err)),
        }
    }
}
