---
layout: page
title: API Reference
nav_order: 9
description: "API reference for Forge CLI components"
permalink: /api-reference
---

# Forge CLI API Reference

This document provides a reference for the key APIs and interfaces in the Forge CLI system. Use this as a technical reference when developing with or extending Forge CLI.

## Core Traits and Interfaces

### API Interface

The primary interface for interacting with Forge CLI:

```rust
#[async_trait::async_trait]
pub trait API {
    /// Get suggestions from the provider
    async fn suggestions(&self) -> Result<Vec<File>>;

    /// Get available tools
    async fn tools(&self) -> Vec<ToolDefinition>;

    /// Get available models
    async fn models(&self) -> Result<Vec<Model>>;

    /// Send a chat request
    async fn chat(
        &self,
        chat: ChatRequest,
    ) -> anyhow::Result<MpscStream<Result<AgentMessage<ChatResponse>, anyhow::Error>>>;

    /// Initialize a conversation with a workflow
    async fn init(&self, workflow: Workflow) -> anyhow::Result<ConversationId>;

    /// Get the environment configuration
    fn environment(&self) -> Environment;

    /// Load a workflow from a path
    async fn load(&self, path: Option<&Path>) -> anyhow::Result<Workflow>;

    /// Get a conversation by ID
    async fn conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> anyhow::Result<Option<Conversation>>;
}
```

### Tool Traits

Traits that define tool behavior:

```rust
/// Provides description metadata for a tool
pub trait ToolDescription: Send + Sync {
    fn description(&self) -> String;
    fn schema(&self) -> schemars::schema::RootSchema;
}

/// Provides a name for a tool
pub trait NamedTool: ToolDescription {
    fn tool_name() -> ToolName;
}

/// Defines how a tool is executed
#[async_trait::async_trait]
pub trait ExecutableTool: ToolDescription + Send + Sync {
    type Input: DeserializeOwned + Send + Sync;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String>;
}
```

### App Trait

The application interface:

```rust
#[async_trait::async_trait]
pub trait App: Send + Sync {
    fn conversation_service(&self) -> &dyn ConversationService;
    fn provider_service(&self) -> &dyn ProviderService;
    fn tool_service(&self) -> &dyn ToolService;
    fn suggestion_service(&self) -> &dyn SuggestionService;
    fn template_service(&self) -> &dyn TemplateService;
    fn environment_service(&self) -> &dyn EnvironmentService;
}
```

## Domain Models

### Agent

Defines an AI agent's configuration:

```rust
pub struct Agent {
    pub id: AgentId,
    pub model: ModelId,
    pub description: Option<String>,
    pub system_prompt: Option<Template<SystemContext>>,
    pub user_prompt: Option<Template<EventContext>>,
    pub suggestions: bool,
    pub ephemeral: bool,
    pub enable: bool,
    pub tools: Vec<ToolName>,
    pub transforms: Vec<Transform>,
    pub subscribe: Vec<String>,
    pub max_turns: Option<u64>,
    pub max_walker_depth: Option<usize>,
}
```

### Conversation

Represents a conversation with agents:

```rust
pub struct Conversation {
    pub id: ConversationId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub workflow: Workflow,
    pub messages: BTreeMap<AgentId, Vec<Message>>,
    pub contexts: BTreeMap<AgentId, SystemContext>,
    pub turns: BTreeMap<AgentId, u64>,
}
```

### Event

Represents an event in the system:

```rust
pub struct Event {
    pub name: String,
    pub value: String,
    pub metadata: HashMap<String, Value>,
}
```

### ChatRequest

Represents a request to chat with an agent:

```rust
pub struct ChatRequest {
    pub content: String,
    pub conversation_id: ConversationId,
}
```

### ChatResponse

Represents a response from an agent:

```rust
pub enum ChatResponse {
    Text(String),
    ToolCallStart(ToolCall),
    ToolCallEnd(ToolResult),
    Custom(Event),
    Usage(Usage),
}
```

## Service Interfaces

### ConversationService

Manages conversations:

```rust
#[async_trait::async_trait]
pub trait ConversationService: Send + Sync {
    async fn create(&self, workflow: Workflow) -> anyhow::Result<ConversationId>;
    
    async fn get(&self, id: &ConversationId) -> anyhow::Result<Option<Conversation>>;
    
    async fn inc_turn(&self, id: &ConversationId, agent: &AgentId) -> anyhow::Result<()>;
    
    async fn set_context(
        &self,
        id: &ConversationId,
        agent: &AgentId,
        context: SystemContext,
    ) -> anyhow::Result<()>;
    
    async fn insert_event(
        &self,
        id: &ConversationId,
        event: Event,
    ) -> anyhow::Result<Vec<AgentId>>;
}
```

### ProviderService

Manages AI provider integration:

```rust
#[async_trait::async_trait]
pub trait ProviderService: Send + Sync {
    async fn models(&self) -> anyhow::Result<Vec<Model>>;
    
    async fn chat(
        &self,
        request: ProviderRequest,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ProviderResponse>>>;
}
```

### ToolService

Manages tool registration and execution:

```rust
#[async_trait::async_trait]
pub trait ToolService: Send + Sync {
    fn list(&self) -> Vec<ToolDefinition>;
    
    async fn execute(
        &self,
        name: &ToolName,
        input: Value,
    ) -> anyhow::Result<ToolResult>;
}
```

### TemplateService

Manages templates:

```rust
pub trait TemplateService: Send + Sync {
    fn render_system_prompt(
        &self,
        template: &Template<SystemContext>,
        context: &SystemContext,
    ) -> anyhow::Result<String>;
    
    fn render_user_prompt(
        &self,
        template: &Template<EventContext>,
        context: &EventContext,
    ) -> anyhow::Result<String>;
}
```

## Implementation Classes

### ForgeAPI

The main API implementation:

```rust
pub struct ForgeAPI<F> {
    app: Arc<F>,
    executor_service: ForgeExecutorService<F>,
    suggestion_service: ForgeSuggestionService<F>,
    loader: ForgeLoaderService<F>,
}

impl ForgeAPI<ForgeApp<ForgeInfra>> {
    pub fn init(restricted: bool) -> Self {
        let infra = Arc::new(ForgeInfra::new(restricted));
        let app = Arc::new(ForgeApp::new(infra));
        ForgeAPI::new(app)
    }
}
```

### ForgeApp

The main application implementation:

```rust
pub struct ForgeApp<F> {
    infra: Arc<F>,
    tool_service: Arc<ForgeToolService>,
    provider_service: ForgeProviderService,
    conversation_service: ForgeConversationService,
    prompt_service: ForgeTemplateService<F, ForgeToolService>,
}
```

### ForgeToolService

The tool service implementation:

```rust
pub struct ForgeToolService {
    tools: HashMap<ToolName, Arc<dyn Tool>>,
    tool_definitions: Vec<ToolDefinition>,
}
```

## Command Line Interface

### Cli Struct

Defines the command-line interface:

```rust
#[derive(Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Path to a file containing initial commands to execute
    #[arg(long, short = 'c')]
    pub command: Option<String>,

    /// Direct prompt to process without entering interactive mode
    #[arg(long, short = 'p')]
    pub prompt: Option<String>,

    /// Enable verbose output mode
    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    /// Enable restricted shell mode for enhanced security
    #[arg(long, default_value_t = false, short = 'r')]
    pub restricted: bool,

    /// Path to a file containing the workflow to execute
    #[arg(long, short = 'w')]
    pub workflow: Option<PathBuf>,
}
```

## Environment

### Environment Struct

Defines the application environment:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    pub os: String,
    pub cwd: PathBuf,
    pub home: Option<PathBuf>,
    pub shell: String,
    pub provider_key: String,
    pub provider_url: Url,
    pub base_path: PathBuf,
    pub qdrant_key: Option<String>,
    pub qdrant_cluster: Option<String>,
    pub pid: u32,
    pub openai_key: Option<String>,
}

impl Environment {
    pub fn log_path(&self) -> PathBuf {
        self.base_path.join("logs")
    }
}
```

## Tool Definitions

### FSRead

Reads file contents:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct FSReadInput {
    /// The path of the file to read, always provide absolute paths.
    pub path: String,
}

#[async_trait::async_trait]
impl ExecutableTool for FSRead {
    type Input = FSReadInput;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        let path = Path::new(&input.path);
        assert_absolute_path(path)?;

        tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file content from {}", input.path))
    }
}
```

### FSCreate

Creates or overwrites files:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct FSCreateInput {
    /// The content to write to the file. ALWAYS provide the COMPLETE
    /// intended content of the file, without any truncation or omissions.
    /// You MUST include ALL parts of the file, even if they haven't been modified.
    pub content: String,
    
    /// If set to true, existing files will be overwritten. If not set and the file
    /// exists, an error will be returned with the content of the existing file.
    #[serde(default)]
    pub overwrite: bool,
    
    /// The path of the file to write to (absolute path required)
    pub path: String,
}

#[async_trait::async_trait]
impl ExecutableTool for FSCreate {
    type Input = FSCreateInput;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        // Implementation details...
    }
}
```

### Shell

Executes shell commands:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct ShellInput {
    /// The shell command to execute.
    pub command: String,
    
    /// The working directory where the command should be executed.
    pub cwd: PathBuf,
}

#[async_trait::async_trait]
impl ExecutableTool for Shell {
    type Input = ShellInput;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        // Implementation details...
    }
}
```

## Useful Types

### ToolName

Represents a unique tool identifier:

```rust
#[derive(Debug, Display, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(String);

impl ToolName {
    pub fn new<S: AsRef<str>>(name: S) -> Self {
        Self(name.as_ref().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### ConversationId

Represents a unique conversation identifier:

```rust
#[derive(Debug, Display, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConversationId(String);

impl ConversationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}
```

## Conclusion

This API reference provides a technical overview of the key interfaces, types, and implementations in Forge CLI. For more detailed information about specific components, refer to the source code or the following documentation sections:

- [Rust Project Explanation](./rust_project_explanation.html)
- [Workflow Architecture](./workflow_architecture.html)
- [Tools System](./tools_system.html)

If you're developing custom tools or extending the system, this reference should serve as a helpful guide to the available APIs.

---

For information about implementing services, see the [Service Documentation](./service.html).