# Multi-Agent Documentation Synchronization System

A comprehensive system for synchronizing documentation between source files and a Docusaurus website using a coordinated ecosystem of specialized AI agents.

## Overview

The Multi-Agent Documentation Synchronization System automates the process of analyzing, transforming, and synchronizing documentation from source files to a Docusaurus website. It leverages a team of specialized agents, each with specific expertise, coordinated through an event-based communication system.

The system is designed to:

1. Analyze source documentation and website files
2. Generate a comprehensive documentation map
3. Determine necessary synchronization operations
4. Execute operations with proper validation
5. Verify synchronization quality and accuracy
6. Provide detailed reporting and quality metrics

## Architecture

### Agent Ecosystem

The system consists of the following specialized agents:

1. **Doc Coordinator Agent**
   - Orchestrates the entire synchronization process
   - Manages the master documentation map
   - Delegates tasks to specialized agents
   - Coordinates all agent activities through events
   - Provides the final synchronization report

2. **Doc Content Syncer Agent**
   - Analyzes source documentation content
   - Extracts metadata and structure information
   - Identifies content changes and transformation needs
   - Recommends content synchronization operations

3. **Docusaurus Expert Agent**
   - Analyzes Docusaurus-specific configurations
   - Manages navigation and sidebar structures
   - Optimizes component usage and configuration
   - Ensures proper Docusaurus implementation

4. **UI Design Expert Agent**
   - Analyzes UI elements and layouts
   - Optimizes component usage for readability
   - Ensures accessibility compliance
   - Enhances overall user experience

5. **CSS Expert Agent**
   - Analyzes and optimizes CSS styling
   - Ensures consistent theming and visual design
   - Improves typography and visual hierarchy
   - Creates visually cohesive documentation

6. **Doc Runner Agent**
   - Executes file operations and transformations
   - Manages error handling and recovery
   - Ensures data integrity during operations
   - Provides detailed execution logs

7. **Doc Verifier Agent**
   - Validates synchronization accuracy and completeness
   - Performs quality assessment across multiple dimensions
   - Identifies issues and improvement opportunities
   - Provides comprehensive verification reports

### Event-Based Communication

Agents communicate through a structured event system with defined payloads:

1. `docs` - Initial trigger from user to Doc Coordinator
2. `docs-analyze-content` - Coordinator request to Content Syncer
3. `docs-content-analyzed` - Content Syncer response to Coordinator
4. `docs-analyze-docusaurus` - Coordinator request to Docusaurus Expert
5. `docs-docusaurus-analyzed` - Docusaurus Expert response to Coordinator
6. `docs-analyze-ui` - Coordinator request to UI Design Expert
7. `docs-ui-analyzed` - UI Design Expert response to Coordinator
8. `docs-analyze-css` - Coordinator request to CSS Expert
9. `docs-css-analyzed` - CSS Expert response to Coordinator
10. `docs-execute` - Coordinator request to Doc Runner
11. `docs-execution-complete` - Doc Runner response to Coordinator
12. `docs-verify` - Coordinator request to Doc Verifier
13. `docs-verification-complete` - Doc Verifier response to Coordinator
14. `docs-complete` - Final completion notification to user

### Master Documentation Map

The system maintains a central documentation map that tracks:

- Source documentation files and metadata
- Target website files and relationships
- Synchronization status of each file
- Proposed and executed operations
- Quality metrics and assessments

This map serves as the shared state that allows coordination between specialized agents.

## Setup

### Prerequisites

- Node.js 16+
- A Docusaurus website setup
- Source documentation in a structured format

### Installation

1. Clone this repository or add it to your project:

```bash
git clone https://github.com/yourusername/doc-sync-system.git
# or
npm install doc-sync-system --save
```

2. Make sure all agent templates are placed in the `/templates` directory:

```
templates/
  ├── system-prompt-doc-coordinator.hbs
  ├── system-prompt-doc-content-syncer.hbs
  ├── system-prompt-docusaurus-expert.hbs
  ├── system-prompt-ui-design-expert.hbs
  ├── system-prompt-css-expert.hbs
  ├── system-prompt-doc-runner.hbs
  └── system-prompt-doc-verifier.hbs
```

3. Configure the agents in your project's agent configuration:

```yaml
# Include the doc-sync-agents.yaml in your project
!include config/agents/doc-sync-agents.yaml
```

## Usage

### Basic Usage

Trigger the documentation synchronization process using the `/docs` command:

```
/docs
```

This will start the complete synchronization process with default parameters.

### Advanced Usage

Provide specific parameters to customize the synchronization process:

```
/docs --source=/path/to/docs --target=/path/to/website --scope=full
```

Available parameters:

- `--source`: Path to source documentation folder
- `--target`: Path to Docusaurus website folder
- `--scope`: Synchronization scope (`full`, `incremental`, `specific`)
- `--filter`: Filter specific files or folders to synchronize
- `--dry-run`: Preview operations without executing them
- `--report-only`: Generate report without modifications

### Monitoring Progress

The system provides real-time progress updates as it moves through the synchronization phases:

1. Initialization and Discovery
2. Analysis Coordination
3. Execution Coordination
4. Verification Coordination
5. Completion and Reporting

Each agent reports its progress every 5 tool calls to provide visibility into the process.

## Customization

### Modifying Agent Templates

Each agent template can be customized to fit specific documentation needs:

1. Edit the appropriate `.hbs` template file
2. Customize the workflow sections to match your documentation structure
3. Add specific rules or considerations for your project

### Extending the System

To add new specialized agents:

1. Create a new agent template
2. Add the agent to `doc-sync-agents.yaml`
3. Define appropriate events for communication
4. Update the Doc Coordinator to delegate tasks to the new agent

## Troubleshooting

Common issues and solutions:

### Synchronization Failures

- Check the execution logs from the Doc Runner Agent
- Verify file permissions and access for both source and target paths
- Ensure Docusaurus configuration files are correctly formatted

### Quality Issues

- Review the verification report from the Doc Verifier Agent
- Check for specific quality scores below acceptable thresholds
- Implement recommended improvements from the verification report

### Agent Communication Errors

- Verify event payloads match expected formats
- Check for missing required fields in event data
- Ensure all agents are properly registered and subscribed to events

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- This system builds upon the Docusaurus documentation framework
- Inspired by multi-agent AI systems and event-driven architectures 