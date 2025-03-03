---
layout: page
title: Custom Tool Development
nav_order: 8
description: "Guide to developing custom tools for Forge CLI"
permalink: /custom-tools
---

# Custom Tool Development Guide

This guide explains how to create custom tools for Forge CLI, allowing you to extend the capabilities of AI agents with your own specialized functionality.

## Overview

Tools in Forge CLI are the primary way that AI agents interact with the system. Each tool:

1. Has a unique name
2. Accepts strongly-typed input parameters
3. Performs a specific operation
4. Returns a result as a string

By creating custom tools, you can enable AI agents to perform specialized tasks specific to your needs or domain.

## Tool Architecture

Before creating a custom tool, it's important to understand the tool architecture in Forge CLI:

### Core Traits

Tools implement these core traits:

```rust
/// Provides metadata about a tool
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

## Creating a Custom Tool

Follow these steps to create your own custom tool:

### Step 1: Define Input Type

First, define the input parameters for your tool:

```rust
use serde::Deserialize;
use schemars::JsonSchema;

#[derive(Deserialize, JsonSchema)]
pub struct CustomToolInput {
    /// Primary data to process (provide a clear description for AI)
    pub data: String,
    
    /// Optional configuration parameter (describe the purpose)
    pub config: Option<String>,
    
    /// Processing mode (describe what different modes do)
    pub mode: ProcessingMode,
}

#[derive(Deserialize, JsonSchema)]
pub enum ProcessingMode {
    #[serde(rename = "fast")]
    Fast,
    
    #[serde(rename = "accurate")]
    Accurate,
}
```

Key points:
- Use `serde::Deserialize` for JSON deserialization
- Use `schemars::JsonSchema` for schema generation
- Provide clear doc comments for each field
- Use strongly-typed enums for options

### Step 2: Implement the Tool Type

Create a struct for your tool and implement the required traits:

```rust
/// Process data using custom algorithms
/// 
/// This tool processes input data according to the specified mode and configuration.
/// It supports both fast and accurate processing modes for different use cases.
#[derive(ToolDescription)]
pub struct CustomTool;

impl NamedTool for CustomTool {
    fn tool_name() -> ToolName {
        ToolName::new("tool_forge_custom_processor")
    }
}

#[async_trait::async_trait]
impl ExecutableTool for CustomTool {
    type Input = CustomToolInput;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        // Implementation logic
        match input.mode {
            ProcessingMode::Fast => {
                // Fast processing implementation
                Ok(format!("Fast processed: {}", input.data))
            }
            ProcessingMode::Accurate => {
                // Accurate processing implementation
                Ok(format!("Accurately processed: {}", input.data))
            }
        }
    }
}
```

Key points:
- Use the `ToolDescription` derive macro for automatic trait implementation
- Provide a unique, descriptive tool name with the `tool_forge_` prefix
- Document your tool with a clear description
- Implement the `call` method with your tool's logic

### Step 3: Register Your Tool

Register your tool with the tool service:

```rust
// Simplified example of registering a tool
impl ForgeToolService {
    pub fn register_custom_tools(&mut self) {
        // Register your custom tool
        self.register_tool(CustomTool);
    }
}
```

## Tool Development Best Practices

### Input Validation

Always validate inputs before processing:

```rust
async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
    // Validate input
    if input.data.is_empty() {
        anyhow::bail!("Input data cannot be empty");
    }
    
    // Process valid input
    // ...
}
```

### Error Handling

Use `anyhow` for rich error reporting:

```rust
async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
    // Attempt operation
    let result = perform_operation(&input.data)
        .with_context(|| format!("Failed to process data: {}", input.data))?;
    
    Ok(result)
}
```

### Asynchronous Operations

Use async/await for I/O-bound operations:

```rust
async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
    // Perform async operations
    let data = tokio::fs::read_to_string(&input.data_path).await?;
    let result = process_data(&data).await?;
    
    Ok(result)
}
```

### Security Considerations

Implement appropriate security checks:

```rust
async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
    // Security validation
    let path = Path::new(&input.path);
    if !path.is_absolute() {
        anyhow::bail!("Path must be absolute: {}", path.display());
    }
    
    // Safe operation on validated path
    // ...
}
```

## Tool Documentation

Provide clear documentation for your tool:

1. **Tool Purpose**: Explain what your tool does and when it should be used
2. **Input Parameters**: Describe each parameter with examples
3. **Return Values**: Explain what the tool returns
4. **Error Cases**: Document potential error scenarios
5. **Usage Examples**: Provide example inputs and outputs

This documentation helps the AI understand how and when to use your tool.

## Testing Custom Tools

Write comprehensive tests for your tool:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_custom_tool_fast_mode() {
        let tool = CustomTool;
        let input = CustomToolInput {
            data: "test data".to_string(),
            config: None,
            mode: ProcessingMode::Fast,
        };
        
        let result = tool.call(input).await.unwrap();
        assert_eq!(result, "Fast processed: test data");
    }
    
    #[tokio::test]
    async fn test_custom_tool_accurate_mode() {
        let tool = CustomTool;
        let input = CustomToolInput {
            data: "test data".to_string(),
            config: Some("extra precision".to_string()),
            mode: ProcessingMode::Accurate,
        };
        
        let result = tool.call(input).await.unwrap();
        assert_eq!(result, "Accurately processed: test data");
    }
    
    #[tokio::test]
    async fn test_custom_tool_error_handling() {
        let tool = CustomTool;
        let input = CustomToolInput {
            data: "".to_string(),  // Empty data should trigger validation error
            config: None,
            mode: ProcessingMode::Fast,
        };
        
        let result = tool.call(input).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }
}
```

## Example Custom Tool Types

Here are some ideas for custom tools you might develop:

### Database Integration Tool

```rust
#[derive(Deserialize, JsonSchema)]
pub struct DatabaseQueryInput {
    /// SQL query to execute
    pub query: String,
    
    /// Optional parameter bindings for the query
    pub parameters: Option<HashMap<String, String>>,
}

#[derive(ToolDescription)]
pub struct DatabaseQueryTool {
    connection_pool: Pool<ConnectionManager<PgConnection>>,
}

impl DatabaseQueryTool {
    pub fn new(connection_string: &str) -> Self {
        // Initialize connection pool
        let manager = ConnectionManager::new(connection_string);
        let pool = Pool::builder().build(manager).expect("Failed to create pool");
        
        Self { connection_pool: pool }
    }
}

#[async_trait::async_trait]
impl ExecutableTool for DatabaseQueryTool {
    type Input = DatabaseQueryInput;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        // Execute database query
        // ...
    }
}
```

### API Integration Tool

```rust
#[derive(Deserialize, JsonSchema)]
pub struct ApiRequestInput {
    /// API endpoint URL
    pub url: String,
    
    /// HTTP method
    pub method: HttpMethod,
    
    /// Request headers
    pub headers: Option<HashMap<String, String>>,
    
    /// Request body
    pub body: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub enum HttpMethod {
    #[serde(rename = "GET")]
    Get,
    
    #[serde(rename = "POST")]
    Post,
    
    #[serde(rename = "PUT")]
    Put,
    
    #[serde(rename = "DELETE")]
    Delete,
}

#[derive(ToolDescription)]
pub struct ApiRequestTool;

#[async_trait::async_trait]
impl ExecutableTool for ApiRequestTool {
    type Input = ApiRequestInput;
    
    async fn call(&self, input: Self::Input) -> anyhow::Result<String> {
        // Make API request
        // ...
    }
}
```

## Conclusion

Creating custom tools is a powerful way to extend Forge CLI with domain-specific capabilities. By following the patterns and best practices outlined in this guide, you can create robust, secure, and useful tools that enhance the abilities of AI agents in your workflow.

For more information about the tools system, see the [Tools System](./tools_system.html) documentation.

To understand how tools fit into the workflow architecture, see the [Workflow Architecture](./workflow_architecture.html) documentation.