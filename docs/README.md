# XZe Documentation

Welcome to the XZe Pipeline Documentation Tool documentation. This documentation follows the [Diátaxis framework](https://diataxis.fr/), organizing content into four distinct categories based on your needs.

## Documentation Structure

```text
docs/
├── README.md                    # This file
├── tutorials/                   # Learning-oriented guides
├── how_to/                      # Problem-solving guides
├── explanation/                 # Understanding-oriented discussion
└── reference/                   # Information-oriented specifications
```

## Quick Navigation

### I want to learn XZe

Start with **[Tutorials](tutorials/)** - Step-by-step lessons that teach you how to use XZe through hands-on practice.

Coming soon:

- Getting Started with XZe
- Your First Documentation Generation
- Setting Up Auto-Mode

### I want to solve a problem

Check **[How-To Guides](how_to/)** - Task-oriented instructions for accomplishing specific goals.

Coming soon:

- How to Install XZe
- How to Configure Multiple Repositories
- How to Deploy XZe in Production
- How to Troubleshoot Common Issues

### I want to understand the system

Read **[Explanation](explanation/)** - Conceptual discussion that clarifies and illuminates topics.

Available now:

- [Project Status Summary](explanation/project_status_summary.md) - Current state and progress
- [Implementation Roadmap](explanation/implementation_roadmap.md) - Detailed phased development plan
- [Phase Overview](explanation/phase_overview.md) - Visual timeline and milestones

Coming soon:

- Architecture Overview
- Design Decisions
- Diátaxis Framework Application
- AI Integration Strategy

### I want to look something up

See **[Reference](reference/)** - Technical specifications and API documentation.

Coming soon:

- CLI Command Reference
- Configuration Schema
- API Reference
- Template Reference

## What is XZe?

XZe is a tool written in Rust that uses open source models from Ollama to analyze a service's source code and documentation in the service git repository and creates documentation for the project. The tool uses the Diátaxis Documentation Framework as the documentation layout.

### Key Features

- **Multi-language Support**: Analyze Rust, Go, Python, JavaScript, Java, and more
- **AI-Powered**: Leverages Ollama and LLMs for intelligent documentation generation
- **Diátaxis Framework**: Organizes documentation into tutorials, how-tos, explanation, and reference
- **Git Integration**: Automatically creates pull requests with generated documentation
- **Multiple Modes**: CLI, server, and VSCode extension
- **Auto-Mode**: Monitors repositories and updates documentation automatically

## Project Status

**Current Status**: Foundation complete, core implementation in progress (40-50% complete)

See [Project Status Summary](explanation/project_status_summary.md) for detailed current state.

## Contributing

We welcome contributions! Please see:

- [AGENTS.md](../AGENTS.md) - Guidelines for working on this project
- [Contributing Guide](../CONTRIBUTING.md) - How to contribute (coming soon)

## Documentation Philosophy

This documentation follows the Diátaxis framework, which recognizes that documentation serves different purposes:

### Tutorials (Learning-oriented)

- Take the user by the hand through a series of steps
- Help beginners get started
- Provide a learning experience
- Focus on teaching through doing

### How-To Guides (Problem-oriented)

- Guide the reader through solving a real-world problem
- Assume some knowledge and experience
- Focus on achieving a specific outcome
- Are recipes for accomplishing tasks

### Explanation (Understanding-oriented)

- Clarify and illuminate a particular topic
- Provide background and context
- Discuss alternatives and design decisions
- Deepen understanding of the system

### Reference (Information-oriented)

- State facts and describe the machinery
- Be accurate and complete
- Provide technical specifications
- Are consulted for specific information

## Getting Help

- **GitHub Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Documentation Issues**: Report unclear or missing documentation

## Related Resources

### Architecture and Requirements

- [XZe Architecture](xze-architecture.md) - Detailed architectural design
- [XZe Requirements](xze-prompt.md) - Original project requirements

### External Resources

- [Diátaxis Framework](https://diataxis.fr/) - Documentation framework we follow
- [Ollama](https://ollama.ai/) - AI model platform we use
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) - Coding standards

## About This Documentation

**Last Updated**: 2024

**Status**: In active development

This documentation is itself a living example of the Diátaxis framework that XZe implements. As we build XZe, we're simultaneously building the documentation that demonstrates its capabilities.

### Documentation Conventions

- **Filenames**: All lowercase with underscores (except README.md)
- **Format**: Markdown (.md extension)
- **Code Blocks**: Always specify language for syntax highlighting
- **Links**: Use relative paths within documentation
- **No Emojis**: Keep documentation professional and accessible

### Feedback

Found an issue with the documentation? Please:

1. Check if it's already reported in GitHub Issues
2. Create a new issue with the `documentation` label
3. Be specific about what's unclear or missing
4. Suggest improvements if you have ideas

Thank you for using XZe!
