---
layout: page
title: Getting Started
nav_order: 1
description: "Get started with Forge CLI, an AI-enhanced terminal development environment"
permalink: /getting-started
---

# Getting Started with Forge CLI

This guide will help you get up and running with Forge CLI quickly.

## Installation

### Mac

Using Homebrew (macOS package manager):

```bash
# Add Code-Forge's package repository to Homebrew
brew tap antinomyhq/code-forge
# Install Code-Forge
brew install code-forge
```

### Linux

Choose either method to install:

```bash
# Using curl (common download tool)
curl -L https://raw.githubusercontent.com/antinomyhq/forge/main/install.sh | bash

# Or using wget (alternative download tool)
wget -qO- https://raw.githubusercontent.com/antinomyhq/forge/main/install.sh | bash
```

## Configuration

1. Create a `.env` file in your home directory with your API credentials:

   ```bash
   # Your API key for accessing AI models
   OPENROUTER_API_KEY=<Enter your Open Router Key>
   ```

   _You can get a Key at [Open Router](https://openrouter.ai/)_

## Basic Usage

### Launch Forge CLI

Simply run the `forge` command in your terminal:

```bash
forge
```

### Command-Line Options

Forge CLI supports several command-line options:

```bash
# Run with a one-time prompt (non-interactive)
forge -p "Create a hello world program in Python"

# Run commands from a file
forge -c commands.txt

# Enable restricted shell mode for enhanced security
forge -r

# Use a specific workflow configuration
forge -w /path/to/workflow.yaml

# Enable verbose output
forge --verbose
```

### Built-in Commands

During an interactive session, you can use these special commands:

- `\new` - Start a new task
- `\info` - View environment summary and logs
- `\models` - List available AI models
- `\dump` - Save the current conversation to a JSON file

### Using Forge CLI as a Development Assistant

Example tasks you can ask Forge CLI to help with:

1. **Code Generation**:
   ```
   Write a function that calculates the Fibonacci sequence in Rust
   ```

2. **Debugging**:
   ```
   Help me debug this code: [paste your code here]
   ```

3. **Code Explanation**:
   ```
   Explain this code to me: [paste code here]
   ```

4. **Refactoring**:
   ```
   Refactor this function to improve performance: [paste function]
   ```

5. **Testing**:
   ```
   Generate unit tests for this class: [paste class]
   ```

## Security Features

Forge CLI provides a restricted shell mode for enhanced security:

- Enable with the `-r` flag
- Prevents potentially harmful operations like:
  - Changing directories
  - Setting/modifying environment variables
  - Executing commands with absolute paths
  - Modifying shell options

## Next Steps

Now that you're up and running with Forge CLI, you might want to explore:

- [Concepts and Architecture](./index.html) - Learn about the core architecture
- [Workflow Architecture](./workflow_architecture.html) - Understand multi-agent workflows
- [Tools System](./tools_system.html) - Explore the tools available to agents