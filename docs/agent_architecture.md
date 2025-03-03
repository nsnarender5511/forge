# Forge CLI Agent Architecture

This document provides a comprehensive explanation of how the agent architecture works in the Forge CLI system, including its components, interactions, and workflows.

## Table of Contents

- [Core Concepts](#core-concepts)
- [Agent Definition](#agent-definition)
- [Multi-Agent Workflow System](#multi-agent-workflow-system)
- [Event-Based Communication](#event-based-communication)
- [System Components](#system-components)
- [Workflow Configuration](#workflow-configuration)
- [Practical Example](#practical-example)
- [Agent Execution Flow](#agent-execution-flow)
- [Advanced Topics](#advanced-topics)

## Core Concepts

The Forge CLI uses a sophisticated multi-agent architecture where specialized AI agents work together to accomplish complex tasks. The key concepts in this architecture include:

- **Agents**: AI-powered entities with specific capabilities and responsibilities
- **Workflows**: Collections of agents organized to work together
- **Events**: Messages that trigger agent actions and facilitate communication
- **Tools**: Capabilities that agents use to interact with the system
- **Templates**: Structured documents that define agent behavior and prompts

This architecture allows the system to decompose complex tasks into smaller, manageable pieces that specialized agents can handle efficiently.

## Agent Definition

Agents are defined by the `Agent` structure in `forge_domain/src/agent.rs`. Each agent has the following properties:

```rust
pub struct Agent {
    pub id: AgentId,                                // Unique identifier
    pub model: ModelId,                             // AI model to use
    pub description: Option<String>,                // Optional description
    pub system_prompt: Option<Template<SystemContext>>, // Instructions for behavior
    pub user_prompt: Option<Template<EventContext>>,    // Format for user inputs
    pub suggestions: bool,                          // Whether to include suggestions
    pub ephemeral: bool,                            // Whether state persists
    pub enable: bool,                               // Whether agent is active
    pub tools: Vec<ToolName>,                       // Available tools
    pub transforms: Vec<Transform>,                 // Message transformations
    pub subscribe: Vec<String>,                     // Events to listen for
    pub max_turns: Option<u64>,                     // Turn limit
    pub max_walker_depth: Option<usize>,            // File traversal depth
}
```

### Agent Identity

Each agent has a unique `AgentId` that identifies it within the system:

```rust
#[derive(Debug, Display, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(String);
```

### Agent System Context

Agents operate within a `SystemContext` that provides environmental information:

```rust
#[derive(Debug, Default, Setters, Clone, Serialize, Deserialize)]
#[setters(strip_option)]
pub struct SystemContext {
    pub env: Option<Environment>,
    pub tool_information: Option<String>,
    pub tool_supported: Option<bool>,
    pub files: Vec<String>,
}
```

### Agent Transformations

Agents can apply transformations to their context:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transform {
    Assistant {
        input: String,
        output: String,
        agent_id: AgentId,
        token_limit: usize,
    },
    User { agent_id: AgentId, output: String },
    PassThrough { agent_id: AgentId, input: String },
}
```

## Multi-Agent Workflow System

Agents operate within a workflow system defined in `forge_domain/src/workflow.rs`:

```rust
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub agents: Vec<Agent>,
}
```

The workflow manages agent discovery and access:

```rust
impl Workflow {
    fn find_agent(&self, id: &AgentId) -> Option<&Agent> {
        self.agents
            .iter()
            .filter(|a| a.enable)
            .find(|a| a.id == *id)
    }

    pub fn get_agent(&self, id: &AgentId) -> crate::Result<&Agent> {
        self.find_agent(id)
            .ok_or_else(|| crate::Error::AgentUndefined(id.clone()))
    }
}
```

## Event-Based Communication

Agents communicate through an event system where:

1. Agents subscribe to specific event types
2. Events are published to the system
3. Subscribing agents process these events
4. Agents can publish new events in response

### Built-in Events

The system includes standard events:

- `user_task_init`: Triggered when a new task is initiated
- `user_task_update`: Triggered when follow-up instructions are provided

### Custom Events

Agents can create and dispatch custom events to facilitate complex workflows and agent interactions.

## System Components

### Conversation Service

The `ForgeConversationService` in `forge_app/src/conversation.rs` manages conversations:

```rust
pub struct ForgeConversationService {
    workflows: Arc<Mutex<HashMap<ConversationId, Conversation>>>,
}
```

Key functions include:

- `get`: Retrieve a conversation by ID
- `create`: Create a new conversation with a workflow
- `inc_turn`: Increment the turn count for an agent
- `set_context`: Set the context for an agent
- `insert_event`: Add an event to a conversation

### Template System

Templates use Handlebars syntax to provide:

- System prompts that define agent behavior
- User prompt formatting
- Tool information documentation

Built-in templates include:

- `system-prompt-engineer.hbs`: Template for engineering tasks
- `system-prompt-title-generator.hbs`: Template for generating descriptive titles
- `system-prompt-advocate.hbs`: Template for user advocacy and explanation
- `partial-tool-information.hbs`: Tool documentation for agents
- `partial-tool-examples.hbs`: Usage examples for tools

### Application Structure

The main application (`ForgeApp`) integrates these components:

```rust
pub struct ForgeApp<F> {
    infra: Arc<F>,
    tool_service: Arc<ForgeToolService>,
    provider_service: ForgeProviderService,
    conversation_service: ForgeConversationService,
    prompt_service: ForgeTemplateService<F, ForgeToolService>,
}
```

## Workflow Configuration

Workflows are configured using YAML files that specify:

- Models to use (with variable references for reuse)
- Agent configurations
- Tool assignments
- Event subscriptions
- Prompt templates

### Configuration Format

```yaml
variables:
  models:
    advanced_model: &advanced_model model/name
    efficiency_model: &efficiency_model model/name

agents:
  - id: agent_id
    model: *model_reference
    tools:
      - tool_name_1
      - tool_name_2
    subscribe:
      - event_name_1
      - event_name_2
    system_prompt: "template_or_direct_content"
    user_prompt: "template_or_direct_content"
```

## Practical Example

The standard Forge CLI configuration includes two agents:

```yaml
agents:
  - id: title_generation_worker
    model: *efficiency_model
    tools:
      - tool_forge_event_dispatch
    subscribe:
      - user_task_init
    system_prompt: "{{> system-prompt-title-generator.hbs }}"
    user_prompt: <technical_content>{{event.value}}</technical_content>

  - id: software-engineer
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

### Agent: title_generation_worker

A lightweight agent that:
- Uses an efficient model (Claude 3.5 Haiku)
- Has limited tool access (only event dispatch)
- Only subscribes to task initialization
- Creates descriptive titles for conversations
- Uses a specific template for title generation

### Agent: software-engineer

A comprehensive agent that:
- Uses an advanced model (Claude 3.7 Sonnet)
- Has access to a wide range of tools:
  - File operations (read, create, remove, patch)
  - Shell command execution
  - Network fetching
  - File searching
- Subscribes to both task initialization and updates
- Persists across interactions (non-ephemeral)
- Uses an engineer-specific system prompt template

## Agent Execution Flow

When a user interacts with the Forge CLI, the following sequence occurs:

1. **Initialization**:
   - The application loads the workflow configuration
   - Agents are instantiated based on the configuration
   - Event subscriptions are registered

2. **Task Submission**:
   - User submits a task via the CLI
   - A `user_task_init` event is created

3. **Event Processing**:
   - The event is published to the workflow
   - Subscribing agents (both title_generation_worker and software-engineer) receive the event

4. **Parallel Execution**:
   - The title_generation_worker creates a conversation title
   - The software-engineer processes the task and generates a response

5. **Tool Usage**:
   - The software-engineer uses its assigned tools to:
     - Read from the filesystem
     - Create or modify files
     - Execute shell commands
     - Search for patterns in files
     - Fetch information from the network

6. **Follow-up Interactions**:
   - User provides additional information
   - A `user_task_update` event is triggered
   - Only the software-engineer responds (since title_generation_worker doesn't subscribe to updates)

## Advanced Topics

### Agent Communication Patterns

Agents can communicate using different patterns:

1. **Direct Communication**:
   - Agent A dispatches an event specifically for Agent B
   - Agent B processes the event and can respond

2. **Broadcast Communication**:
   - Agent A dispatches a general event
   - All agents subscribing to that event type receive it

3. **Chained Processing**:
   - Agent A processes part of a task
   - Agent A dispatches an event with its results
   - Agent B processes the results further

### Agent Transformations

Transformations can modify how agents process or respond to events:

1. **Assistant Transformation**:
   - Compresses multiple assistant messages into a single message
   - Useful for maintaining context while managing token limits

2. **User Transformation**:
   - Enriches user prompts with additional information
   - Can add context or formatting to improve agent understanding

3. **PassThrough Transformation**:
   - Intercepts context without modifying it
   - Useful for logging, analysis, or triggering side effects

### Customizing Agent Behavior

Agent behavior can be customized through several mechanisms:

1. **System Prompts**:
   - Detailed instructions that define agent personality, capabilities, and constraints
   - Can include examples, guidelines, and procedural instructions

2. **User Prompts**:
   - Format specifications for user inputs
   - Can include XML tags, formatting, and variable references

3. **Tool Assignment**:
   - Controlling which tools an agent can use
   - Limiting capabilities based on security or performance considerations

4. **Event Subscriptions**:
   - Determining which events an agent responds to
   - Creating specialized agents that only handle specific situations

### Creating Custom Agents

To create a custom agent:

1. Define the agent in the workflow configuration:
   ```yaml
   - id: custom_agent
     model: *model_reference
     tools:
       - tool_1
       - tool_2
     subscribe:
       - event_1
       - event_2
     system_prompt: "Custom instructions"
     user_prompt: "Custom format"
   ```

2. Create custom templates if needed:
   - System prompt templates for specialized behavior
   - User prompt templates for specific formatting

3. Assign appropriate tools based on the agent's responsibilities

4. Configure event subscriptions to control when the agent activates

### Debugging Agent Behavior

When troubleshooting agent behavior:

1. Check event subscriptions to ensure agents are receiving the right events
2. Verify tool permissions to ensure agents can perform required operations
3. Examine system and user prompts for clear instructions
4. Review agent transformations for unexpected modifications
5. Check model assignments to ensure appropriate capabilities

---

This document provides an overview of the Forge CLI agent architecture. For more specific implementation details, refer to the relevant source files in the Forge CLI codebase.