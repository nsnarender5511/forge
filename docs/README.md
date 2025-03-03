# Forge CLI Documentation

Welcome to the comprehensive documentation for Forge CLI, an AI-enhanced terminal development environment built in Rust.

## Documentation Structure

This documentation is organized into the following sections:

### Core Documentation

1. **[Getting Started](./getting_started.md)** - Quick introduction to installing and using Forge CLI
2. **[Concepts and Architecture](./index.md)** - High-level overview of the Forge CLI architecture
3. **[Rust Project Explanation](./rust_project_explanation.md)** - Detailed breakdown of the Rust codebase
4. **[Workflow Architecture](./workflow_architecture.md)** - Understanding the multi-agent workflow system
5. **[Tools System](./tools_system.md)** - Guide to the tools system and its implementation

### Developer Guides

1. **[Development Guidelines](./guidelines.md)** - Best practices for developing with Forge CLI
2. **[Onboarding Guide](./onboarding.md)** - Getting started as a Forge CLI developer
3. **[Service Documentation](./service.md)** - Implementation details for services
4. **[Agent Architecture](./agent_architecture.md)** - In-depth explanation of the agent system
5. **[Testing Guide](./testing_guide.md)** - Guide to testing Forge CLI components
6. **[Custom Tool Development](./custom_tools.md)** - Creating custom tools for Forge CLI
7. **[API Reference](./api_reference.md)** - API documentation for Forge CLI components

### For Rust Beginners

If you're new to Rust and want to understand this codebase, check out our dedicated guide:

- **[Forge CLI for Rust Beginners](./forge_cli_rust_documentation.md)** - A guide specifically for those learning Rust

## Running the Documentation Locally

This documentation uses the [Just the Docs](https://just-the-docs.github.io/just-the-docs/) Jekyll theme.

### Prerequisites

- Ruby installed (version 2.7.0 or higher recommended)
- Bundler gem installed (`gem install bundler`)

### Setup and Run

1. Navigate to the docs directory:
   ```bash
   cd docs
   ```

2. Install dependencies:
   ```bash
   bundle install
   ```

3. Start the local server:
   ```bash
   bundle exec jekyll serve
   ```

4. Open your browser and go to: `http://localhost:4000`

## Adding or Modifying Documentation

1. All documentation is written in Markdown (.md) files
2. Each file should have front matter at the top like this:
   ```yaml
   ---
   layout: page
   title: Page Title
   nav_order: 2
   description: "Description of the page"
   permalink: /page-url
   ---
   ```

3. The `nav_order` value determines the position in the navigation menu

## Theme Configuration

The Just the Docs theme is configured in `_config.yml`. Refer to the [Just the Docs documentation](https://just-the-docs.github.io/just-the-docs/) for customization options.

## Contributing to Documentation

We welcome contributions to improve this documentation. To contribute:

1. Fork the repository
2. Create a new branch for your changes
3. Make your changes to the documentation
4. Submit a pull request with a clear description of your changes

Please ensure your contributions follow our [documentation standards](./documentation_standards.md).