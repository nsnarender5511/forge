# Forge CLI Documentation for Rust Beginners

This documentation provides a comprehensive explanation of the Forge CLI project, designed specifically for developers who are new to Rust. It breaks down the architecture, components, and Rust-specific concepts used throughout the project.

## Documentation Overview

This documentation consists of several detailed guides:

1. **[Rust Project Explanation](./rust_project_explanation.md)** - A comprehensive overview of the project and Rust concepts used
2. **[Workflow Architecture](./workflow_architecture.md)** - Detailed explanation of the multi-agent workflow system
3. **[Tools System](./tools_system.md)** - In-depth guide to the tools system that enables AI capabilities

## Getting Started

If you're new to Rust and this project, we recommend reading these documents in the following order:

1. Start with the **Rust Project Explanation** for a high-level overview and introduction to key Rust concepts
2. Explore the **Tools System** to understand how the AI interacts with the system
3. Finally, dive into the **Workflow Architecture** to learn about the multi-agent capabilities

## Key Rust Concepts Covered

These documentation files explain numerous Rust concepts in the context of the Forge CLI project:

- Ownership and borrowing
- Traits and trait objects
- Error handling with Result and anyhow
- Asynchronous programming with async/await and Tokio
- Smart pointers (Arc, Box)
- Procedural macros
- Testing strategies
- Module organization
- Workspace and crate architecture

## Project Structure

The Forge CLI is organized as a Rust workspace with multiple crates. Each crate has a specific responsibility in the application:

```
forge_cli/
├── Cargo.toml            # Workspace definition
├── crates/               # Contains all the project crates
│   ├── forge_main/       # Main application executable
│   ├── forge_api/        # API definitions and interfaces
│   ├── forge_app/        # Core application logic
│   ├── forge_domain/     # Domain models and types
│   ├── forge_infra/      # Infrastructure interfaces
│   ├── forge_open_router/# OpenAI API integration
│   ├── forge_stream/     # Stream handling utilities
│   ├── forge_display/    # Terminal display utilities
│   ├── forge_tracker/    # Telemetry and tracking
│   └── forge_tool_macros/# Procedural macros for tools
```

## Running the Application

To run the application, use the following commands:

```bash
# Build the application
cargo build

# Run the application
cargo run

# Run with a specific workflow configuration
cargo run -- -w /path/to/workflow.yaml

# Run in restricted shell mode for enhanced security
cargo run -- -r
```

## Building Your Knowledge

As you explore this documentation, we recommend:

1. **Try Small Changes**: Make small modifications to understand how different parts work
2. **Read the Tests**: The project has extensive tests that demonstrate how components are expected to work
3. **Experiment with Workflows**: Create custom workflows to see how the multi-agent system operates
4. **Explore Tool Implementations**: Look at how different tools are implemented to learn about Rust patterns

## Resources for Learning Rust

If you're new to Rust, these resources can help you deepen your understanding:

- [The Rust Book](https://doc.rust-lang.org/book/) - The official Rust programming language book
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - Learn Rust through examples
- [Tokio Documentation](https://tokio.rs/tokio/tutorial) - Guide to async programming with Tokio
- [Rustlings](https://github.com/rust-lang/rustlings) - Small exercises to get used to reading and writing Rust code

## Conclusion

Forge CLI demonstrates modern Rust development practices in a real-world application. By studying this codebase and documentation, you'll gain valuable insights into how Rust's unique features enable secure, concurrent, and maintainable software.

Enjoy exploring the project, and welcome to the world of Rust programming!