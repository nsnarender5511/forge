---
layout: page
title: Testing Guide
nav_order: 7
description: "Guide to testing Forge CLI components"
permalink: /testing-guide
---

# Testing Guide for Forge CLI

This guide covers the testing approach and best practices for Forge CLI, helping developers understand how to effectively test components of the system.

## Testing Philosophy

Forge CLI follows these testing principles:

1. **Comprehensive Coverage**: Tests should cover all critical paths and edge cases
2. **Isolated Tests**: Unit tests should be independent and not rely on external state
3. **Clear Assertions**: Tests should have clear expected outcomes
4. **Maintainable Tests**: Tests should be easy to understand and maintain
5. **Fast Execution**: Tests should execute quickly to encourage frequent running

## Test Types

### Unit Tests

Unit tests focus on testing individual components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tool_name_creation() {
        let tool_name = ToolName::new("test_tool");
        assert_eq!(tool_name.as_str(), "test_tool");
    }
}
```

### Integration Tests

Integration tests verify that multiple components work together correctly:

```rust
// In forge_inte/tests/api_spec.rs
#[tokio::test]
async fn test_api_workflow() {
    // Test components working together
    let api = ForgeAPI::init(false);
    let workflow = api.load(Some(Path::new("tests/test_workflow.yaml"))).await.unwrap();
    let conversation_id = api.init(workflow).await.unwrap();
    
    // Make assertions about the interaction
}
```

### Snapshot Tests

Snapshot tests compare the output of a function with a previously captured "snapshot":

```rust
#[test]
fn test_format_output() {
    let output = format_output(sample_output);
    insta::assert_snapshot!(output);
}
```

## Test Structure

Follow this pattern for test structure:

```rust
use pretty_assertions::assert_eq; // Always use pretty assertions

fn test_something() {
    // Arrange: Set up the test fixtures
    let fixture = get_test_fixture();
    
    // Act: Execute the code under test
    let actual = code_under_test(fixture);
    
    // Assert: Verify the outcome
    let expected = expected_outcome();
    assert_eq!(actual, expected);
}
```

## Test Utilities

### TempDir

Use `TempDir` for file system operations to ensure test isolation:

```rust
#[tokio::test]
async fn test_fs_read() {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    
    // Create a test file
    fs::write(&file_path, "Test content").await.unwrap();
    
    // Test file system operations
    let fs_read = FSRead;
    let result = fs_read
        .call(FSReadInput { path: file_path.to_string_lossy().to_string() })
        .await
        .unwrap();
    
    assert_eq!(result, "Test content");
    
    // No need to clean up - TempDir handles it automatically
}
```

### Test Mocks

Create mock implementations for testing interfaces:

```rust
// Define a test mock
#[cfg(test)]
pub struct TestToolService {
    // Mock state
    pub tools_called: RefCell<Vec<String>>,
}

#[cfg(test)]
impl TestToolService {
    pub fn new() -> Self {
        Self {
            tools_called: RefCell::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl ToolService for TestToolService {
    fn execute_tool(&self, name: &str, input: &str) -> Result<String, Error> {
        self.tools_called.borrow_mut().push(name.to_string());
        Ok("Mock response".to_string())
    }
}
```

## Testing Async Code

Use `tokio::test` for async tests:

```rust
#[tokio::test]
async fn test_async_function() {
    // Test async code
    let result = some_async_function().await;
    assert!(result.is_ok());
}
```

## Testing Different Components

### Testing Tools

When testing tools, focus on:

1. Valid inputs produce expected outputs
2. Invalid inputs produce descriptive errors
3. Security constraints are enforced
4. Error messages are helpful

Example:

```rust
#[tokio::test]
async fn test_fs_read_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_file = temp_dir.path().join("nonexistent.txt");
    
    let fs_read = FSRead;
    let result = fs_read
        .call(FSReadInput { path: nonexistent_file.to_string_lossy().to_string() })
        .await;
        
    // Verify error behavior
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to read file"));
}
```

### Testing Agents

When testing agent behavior:

1. Test event subscription and handling
2. Verify system prompt generation
3. Check tool permission enforcement
4. Test agent interactions via events

### Testing Workflows

For workflow testing:

1. Test workflow loading from YAML
2. Check agent instantiation
3. Verify event routing
4. Test end-to-end workflow execution

## Continuous Integration

Forge CLI uses GitHub Actions for continuous integration testing:

1. Unit tests run on every pull request
2. Integration tests verify end-to-end behavior
3. Snapshot tests catch unexpected changes
4. Linting ensures code quality

## Writing Effective Tests

### Do's

- Use descriptive test names that explain what's being tested
- Test both success and failure cases
- Use fixtures for test data preparation
- Test edge cases and boundary conditions
- Keep tests independent of each other

### Don'ts

- Don't rely on test execution order
- Avoid testing implementation details when possible
- Don't use sleep or fixed delays in tests
- Don't test third-party code
- Avoid duplicating implementation logic in tests

## Conclusion

Effective testing is essential for maintaining the quality and reliability of Forge CLI. By following these testing practices, you'll help ensure that the system remains robust, maintainable, and secure.

For more information about the components being tested, see:

- [Tools System](./tools_system.html) - Understanding the tools architecture
- [Workflow Architecture](./workflow_architecture.html) - Details on the workflow system
- [Development Guidelines](./guidelines.html) - General development best practices