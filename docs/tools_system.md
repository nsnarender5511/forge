# Forge CLI - Tools System Explained

## Table of Contents
1. [Introduction](#introduction)
2. [Tool Architecture](#tool-architecture)
3. [Tool Categories](#tool-categories)
4. [Tool Implementation](#tool-implementation)
5. [Tool Invocation Flow](#tool-invocation-flow)
6. [Security Considerations](#security-considerations)
7. [Custom Tool Development](#custom-tool-development)
8. [Rust Features Utilized](#rust-features-utilized)

## Introduction

The tools system is a central component of Forge CLI, enabling AI agents to interact with the file system, execute shell commands, make network requests, and perform other operations. This document explains the architecture and implementation details of the tools system, with a focus on how these features are built in Rust.

## Tool Architecture

### Core Concepts

In Forge CLI, tools are Rust types that implement specific traits to provide functionality to AI agents. The core design follows these principles:

1. **Trait-based abstraction**: Tools implement common traits for uniformity
2. **Strong typing**: Tool inputs and outputs are strongly typed
3. **Asynchronous execution**: Tools use Rust's async/await for efficient I/O
4. **Declarative description**: Tools provide metadata about their purpose and parameters
5. **Error handling**: Tools use Result types for clear error reporting

### Core Traits

The tool system is built around several key traits:

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

These traits provide a unified interface for tools, allowing them to be registered, described, and invoked in a consistent manner.

## Tool Categories

Forge CLI implements several categories of tools:

### File System Tools

Tools for interacting with the file system:

- `tool_forge_fs_read`: Read file contents
- `tool_forge_fs_create`: Create or overwrite files
- `tool_forge_fs_remove`: Remove files
- `tool_forge_fs_search`: Search for patterns in files
- `tool_forge_fs_patch`: Apply patches to existing files

### Process Tools

Tools for executing code and shell commands:

- `tool_forge_process_shell`: Execute shell commands
- `tool_forge_process_think`: Perform internal reasoning (no external effect)

### Network Tools

Tools for making network requests:

- `tool_forge_net_fetch`: Fetch data from a URL

### Event Tools

Tools for inter-agent communication:

- `tool_forge_event_dispatch`: Dispatch events to other agents

## Tool Implementation

Let's examine the implementation of a specific tool to understand the pattern:

### Example: FSRead Tool

```rust
#[derive(Deserialize, JsonSchema)]
pub struct FSReadInput {
    /// The path of the file to read, always provide absolute paths.
    pub path: String,
}

/// Request to read the contents of a file at the specified path. Use this when
/// you need to examine the contents of an existing file you don't know the
/// contents of, for example to analyze code, review text files, or extract
/// information from configuration files.
#[derive(ToolDescription)]
pub struct FSRead;

impl NamedTool for FSRead {
    fn tool_name() -> ToolName {
        ToolName::new("tool_forge_fs_read")
    }
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

Key components:

1. **Input Definition**: A struct with JSON Schema attributes for validation and documentation
2. **Tool Description**: A derive macro that implements the `ToolDescription` trait
3. **Tool Name**: Implementation of `NamedTool` for identifying the tool
4. **Execution Logic**: Implementation of `ExecutableTool` with the actual functionality

### Tool Registration

Tools are registered with a tool service that makes them available to agents:

```rust
// Simplified version of how tools are registered
pub fn register_tools(&mut self) {
    self.register_tool(FSRead);
    self.register_tool(FSCreate);
    self.register_tool(FSRemove);
    self.register_tool(ShellTool::new(self.env.clone()));
    self.register_tool(NetFetch::new());
    // ... other tools
}
```

## Tool Invocation Flow

When an AI agent wants to use a tool, it follows this process:

1. **Tool Selection**: The AI selects a tool by name based on the task
2. **Input Preparation**: The AI constructs a JSON object matching the tool's input schema
3. **Tool Call**: The framework deserializes the input and invokes the appropriate tool
4. **Result Handling**: The tool's output is returned to the AI as a string
5. **Error Handling**: If the tool fails, the error is returned to the AI for handling

This process is managed by the `ForgeExecutorService`, which handles the interaction between AI agents and tools:

```rust
// Simplified tool execution flow
async fn execute_tool_call(&self, tool_call: ToolCall) -> Result<ToolResult, Error> {
    let tool_name = ToolName::new(&tool_call.name);
    
    // Find the appropriate tool
    let maybe_tool = self.tools.get(&tool_name);
    let tool = match maybe_tool {
        Some(tool) => tool,
        None => return Err(Error::UnknownTool(tool_name.to_string())),
    };
    
    // Execute the tool with the provided input
    let result = tool.execute(tool_call.input).await?;
    
    Ok(ToolResult {
        name: tool_name,
        content: result,
        is_error: false,
    })
}
```

## Security Considerations

Security is a major concern for tools that interact with the system:

### Path Validation

File system tools validate paths to prevent unauthorized access:

```rust
fn assert_absolute_path(path: &Path) -> anyhow::Result<()> {
    if !path.is_absolute() {
        anyhow::bail!("Path must be absolute: {}", path.display());
    }
    Ok(())
}
```

### Restricted Shell

The shell tool can be run in a restricted mode that limits what commands can be executed:

```rust
pub fn new(env: Environment) -> Self {
    // Use rbash in restricted mode
    let shell = if env.restricted {
        "/bin/rbash".to_string()
    } else {
        env.shell.clone()
    };
    
    Self { shell }
}
```

### Error Isolation

Errors in tools are caught and reported without crashing the application:

```rust
async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
    // Attempt operation and provide context for errors
    tokio::fs::remove_file(&input.path)
        .await
        .with_context(|| format!("Failed to remove file: {}", input.path))
}
```

## Custom Tool Development

Forge CLI can be extended with custom tools. Here's how to create one:

### 1. Define Input Type

```rust
#[derive(Deserialize, JsonSchema)]
pub struct CustomToolInput {
    /// Description of the parameter
    pub parameter_one: String,
    /// Description of another parameter
    pub parameter_two: Option<i32>,
}
```

### 2. Implement the Tool

```rust
/// Description of what the tool does
#[derive(ToolDescription)]
pub struct CustomTool;

impl NamedTool for CustomTool {
    fn tool_name() -> ToolName {
        ToolName::new("tool_forge_custom_tool")
    }
}

#[async_trait::async_trait]
impl ExecutableTool for CustomTool {
    type Input = CustomToolInput;

    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        // Implementation logic
        Ok(format!("Processed: {}", input.parameter_one))
    }
}
```

### 3. Register the Tool

```rust
self.register_tool(CustomTool);
```

## Rust Features Utilized

The tools system leverages several advanced Rust features:

### 1. Procedural Macros

The `ToolDescription` derive macro auto-generates implementation details:

```rust
#[derive(ToolDescription)]
pub struct ShellTool { /* ... */ }
```

This expands to implement the `ToolDescription` trait with JSONSchema generation.

### 2. Associated Types

The `ExecutableTool` trait uses associated types to specify the input type:

```rust
#[async_trait::async_trait]
pub trait ExecutableTool: ToolDescription + Send + Sync {
    type Input: DeserializeOwned + Send + Sync;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String>;
}
```

This provides type safety while allowing flexibility in implementation.

### 3. Generic Trait Bounds

Tools use generic traits to constrain behavior:

```rust
pub trait ToolDescription: Send + Sync {
    // Methods...
}
```

The `Send + Sync` bounds ensure tools can be shared across threads safely.

### 4. Async/Await

Tools use async/await for non-blocking operations:

```rust
async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
    let result = tokio::fs::read_to_string(&input.path).await?;
    Ok(result)
}
```

This allows the application to handle many concurrent tool executions efficiently.

### 5. Error Handling with Context

Tools use `anyhow` for rich error reporting:

```rust
.with_context(|| format!("Failed to read file: {}", input.path))
```

This provides helpful context when errors occur.

## Testing Tools

The tools are thoroughly tested through unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fs_read_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let test_content = "Hello, World!";
        fs::write(&file_path, test_content).await.unwrap();
        
        let fs_read = FSRead;
        let result = fs_read
            .call(FSReadInput { path: file_path.to_string_lossy().to_string() })
            .await
            .unwrap();
            
        assert_eq!(result, test_content);
    }
    
    #[tokio::test]
    async fn test_fs_read_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_file = temp_dir.path().join("nonexistent.txt");
        
        let fs_read = FSRead;
        let result = fs_read
            .call(FSReadInput { path: nonexistent_file.to_string_lossy().to_string() })
            .await;
            
        assert!(result.is_err());
    }
}
```

These tests ensure tools behave as expected under various conditions.

## Conclusion

The tools system in Forge CLI demonstrates how Rust's type system, trait-based polymorphism, and async capabilities can be used to create a flexible, secure, and efficient framework for AI to interact with system resources. 

By understanding this system, developers can both use the existing tools effectively and extend the system with new capabilities, leveraging the safety and performance guarantees of Rust.