# Forge CLI - Workflow and Multi-Agent Architecture

## Table of Contents
1. [Introduction](#introduction)
2. [Workflow Concepts](#workflow-concepts)
3. [Agent System Explained](#agent-system-explained)
4. [Event Architecture](#event-architecture)
5. [Tool Permission System](#tool-permission-system)
6. [Configuration via YAML](#configuration-via-yaml)
7. [Workflow Example Walkthrough](#workflow-example-walkthrough)
8. [Advanced Configuration Options](#advanced-configuration-options)

## Introduction

One of the most powerful features of the Forge CLI is its multi-agent architecture, allowing complex workflows to be composed from specialized AI agents working together. This document explores how this system is implemented in Rust and how users can configure it for their needs.

## Workflow Concepts

In Forge CLI, a workflow is a configuration of one or more AI agents that work together to accomplish tasks. Each agent:

1. Has a specific role or capability
2. Can access certain tools (like file operations or shell commands)
3. Subscribes to certain events
4. May publish events for other agents to consume

This event-driven architecture allows complex tasks to be broken down into smaller, more specialized pieces handled by appropriate agents.

## Agent System Explained

### Agent Definition

In the Rust codebase, agents are defined in the domain layer:

```rust
// Simplified version of the actual code
pub struct Agent {
    pub id: String,
    pub model: Model,
    pub tools: Vec<ToolName>,
    pub subscribe: Vec<String>,
    pub ephemeral: bool,
    pub system_prompt: Option<String>,
    pub user_prompt: Option<String>,
}
```

Key components:
- `id`: Unique identifier for the agent
- `model`: AI model specification (e.g., "anthropic/claude-3.5-sonnet")
- `tools`: List of tools this agent can use
- `subscribe`: Events the agent listens to
- `ephemeral`: If true, agent is destroyed after task completion
- `system_prompt`: Instructions for how the agent should behave
- `user_prompt`: Template for formatting user inputs

### Agent Implementation

The agent implementation is primarily in the `forge_app` crate, where agents are instantiated from configuration and registered with the event system.

Agents process events they subscribe to, potentially invoking AI models, using tools, and publishing new events. This is handled by the `ForgeExecutorService`.

## Event Architecture

The event system is the backbone of the multi-agent architecture:

```rust
// Simplified example
pub struct Event {
    pub name: String,
    pub value: String,
    pub metadata: HashMap<String, Value>,
}
```

### Built-in Events

- `user_task_init` - Published when a new task is initiated
- `user_task_update` - Published when follow-up instructions are provided by the user

### Custom Events

Agents can publish custom events to communicate with other agents, for example:

- `code_analysis_complete` - Published when code analysis is done
- `task_title_generated` - Published when a descriptive title is created
- `code_change_recommended` - Published when code changes are suggested

### Event Dispatch

Events are dispatched using the `tool_forge_event_dispatch` tool, allowing agents to trigger actions in other agents:

```rust
// Example of how an agent would dispatch an event
let dispatch_input = EventDispatchInput {
    name: "code_analysis_complete".to_string(),
    value: analysis_result.to_string(),
    metadata: HashMap::new(),
};

event_dispatch_tool.call(dispatch_input).await?;
```

## Tool Permission System

Agents are granted permission to use specific tools, defined in their configuration:

```yaml
agents:
  - id: code_analyzer
    tools:
      - tool_forge_fs_read
      - tool_forge_fs_search
      - tool_forge_event_dispatch
    # Other configuration...
```

This allows for precise control over what each agent can do, implementing a principle of least privilege approach to security.

### Tool Isolation

Tools are implemented as trait objects that implement the `ExecutableTool` trait:

```rust
#[async_trait::async_trait]
pub trait ExecutableTool: ToolDescription + Send + Sync {
    type Input: DeserializeOwned + Send + Sync;

    async fn call(&self, input: Self::Input) -> anyhow::Result<String>;
}
```

Each tool is registered with the tool service and only made available to agents with appropriate permissions.

## Configuration via YAML

Workflows are configured using YAML files, allowing for declarative definitions of complex agent systems:

### Basic Structure

```yaml
variables:
  # Define reusable variables
  models:
    advanced_model: &advanced_model anthropic/claude-3.7-sonnet
    efficiency_model: &efficiency_model anthropic/claude-3.5-haiku

agents:
  # List of agent definitions
  - id: agent_id
    model: *model_reference
    tools:
      # List of available tools
    subscribe:
      # Events to listen to
    # Other configuration...
```

### Loading Workflows

Workflows are loaded from YAML files using the `ForgeLoaderService`:

```rust
// Simplified example
pub async fn load(&self, path: Option<&Path>) -> anyhow::Result<Workflow> {
    match path {
        Some(path) => self.load_from_file(path).await,
        None => self.load_default().await,
    }
}
```

## Workflow Example Walkthrough

Let's analyze a complete workflow example to understand how it operates:

```yaml
variables:
  models:
    advanced_model: &advanced_model anthropic/claude-3.7-sonnet
    efficiency_model: &efficiency_model anthropic/claude-3.5-haiku

agents:
  - id: title_generation_worker
    model: *efficiency_model
    tools:
      - tool_forge_event_dispatch
    subscribe:
      - user_task_init
    system_prompt: "{{> system-prompt-title-generator.hbs }}"
    user_prompt: <technical_content>{{event.value}}</technical_content>

  - id: developer
    model: *advanced_model
    tools:
      - tool_forge_fs_read
      - tool_forge_fs_create
      - tool_forge_fs_remove
      - tool_forge_fs_patch
      - tool_forge_process_shell
      - tool_forge_net_fetch
      - tool_forge_fs_search
    subscribe:
      - user_task_init
      - user_task_update
    ephemeral: false
    system_prompt: "{{> system-prompt-engineer.hbs }}"
    user_prompt: |
      <task>{{event.value}}</task>
```

### Workflow Execution Flow:

1. User enters a task, triggering the `user_task_init` event
2. Both `title_generation_worker` and `developer` agents receive this event
3. `title_generation_worker`:
   - Processes the task to generate a meaningful title
   - Uses a more efficient model for this limited task
   - Dispatches an event with the generated title
4. `developer`:
   - Processes the full task with a more capable model
   - Has access to file system and shell tools for complex operations
   - Remains active (`ephemeral: false`) to handle follow-up interactions
5. User provides additional input, triggering `user_task_update`
6. Only `developer` receives this event (as configured) and continues the task

## Advanced Configuration Options

### Templating System

Forge uses Handlebars templates for agent prompts:

```yaml
system_prompt: "{{> system-prompt-engineer.hbs }}"
```

This loads a predefined template, allowing for consistent agent behavior across workflows.

### Event Metadata

Events can include structured metadata beyond the main value:

```rust
let metadata = HashMap::from([
    ("file_count".to_string(), json!(files.len())),
    ("language".to_string(), json!("rust")),
]);

event_dispatch_tool.call(EventDispatchInput {
    name: "analysis_result".to_string(),
    value: summary,
    metadata,
}).await?;
```

### Conditional Subscriptions

While not directly supported in the YAML, agents can implement conditional event handling in their system prompts:

```
When you receive a 'code_analysis_complete' event, check if the 'language' 
metadata is 'rust'. If it is, proceed with Rust-specific analysis.
```

## Rust Implementation Details

The workflow system leverages several advanced Rust features:

1. **Trait Objects for Polymorphism**: Tools are managed as `Box<dyn ExecutableTool>`, allowing different tool implementations to be used uniformly.

2. **Async/Await for Concurrency**: Agent processing uses async Rust to handle concurrent operations efficiently.

3. **Type-Safe Deserialization**: YAML configurations are deserialized into strongly-typed Rust structures using `serde`.

4. **Arc for Shared Ownership**: Shared resources like the event bus use `Arc` for thread-safe reference counting.

5. **Error Handling with Context**: The `anyhow` crate is used for rich error handling with context.

Example:

```rust
// Loading a workflow with context-aware error handling
async fn load_from_file(&self, path: &Path) -> anyhow::Result<Workflow> {
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read workflow file: {}", path.display()))?;
        
    let workflow: Workflow = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse workflow YAML: {}", path.display()))?;
        
    Ok(workflow)
}
```

## Conclusion

The multi-agent workflow system in Forge CLI represents a sophisticated approach to AI orchestration, allowing specialized agents to collaborate on complex tasks through an event-driven architecture. 

By understanding how this system is implemented and configured, users can create powerful custom workflows tailored to their specific needs, while developers can extend the system with new tools, event types, and agent capabilities.