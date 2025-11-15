# XZe Documentation

Welcome to the XZe documentation. This documentation follows the [Diátaxis framework](https://diataxis.fr/), organizing content into four distinct categories based on your needs.

## What is XZe?

XZe is an AI-powered documentation generation tool that automatically creates and maintains Diátaxis-structured documentation for your code repositories.

### Key Features

- **API-First Design**: REST API (`xze-serve`) as the primary interface
- **Event-Driven**: Automatically triggered by webhooks (GitHub/GitLab) or Kafka events
- **AI-Powered**: Uses Ollama models to intelligently analyze code and generate documentation
- **Diátaxis Framework**: Organizes documentation into tutorials, how-tos, explanations, and reference
- **Git Integration**: Automatically creates pull requests with generated documentation
- **Multi-Repository**: Manage documentation for multiple codebases simultaneously

## Documentation Structure

This documentation is organized following the Diátaxis framework:

```text
docs/
├── README.md              # This file - navigation and overview
├── tutorials/             # Learning-oriented: step-by-step lessons
├── how_to/                # Goal-oriented: problem-solving recipes
├── explanation/           # Understanding-oriented: conceptual discussion
└── reference/             # Information-oriented: technical specifications
```

## Quick Navigation

### 📚 [Tutorials](tutorials/) - Learn XZe

**Learning-oriented guides** that take you by the hand through XZe step-by-step.

Start here if you're new to XZe.

- Getting started with XZe (coming soon)
- Your first documentation generation (coming soon)
- Working with events and webhooks (coming soon)
- Deploying XZe to production (coming soon)

### 🔧 [How-To Guides](how_to/) - Solve Problems

**Task-oriented recipes** for accomplishing specific goals with XZe.

Use these when you know what you want to achieve.

- Configure webhooks for GitHub/GitLab (coming soon)
- Set up Kafka event streaming (coming soon)
- Customize AI models and generation (coming soon)
- Deploy to Kubernetes (coming soon)
- Monitor XZe in production (coming soon)

### 💡 [Explanation](explanation/) - Understand the System

**Conceptual discussion** that clarifies and illuminates XZe's design and decisions.

Read these to deepen your understanding of how and why XZe works.

- [Implementation Log](explanation/implementations.md) - Chronological record of all implementations
- Architecture decision records (coming soon)
- Design rationale for major features (coming soon)
- AI-powered documentation generation concepts (coming soon)
- Event-driven architecture explained (coming soon)

### 📖 [Reference](reference/) - Look Things Up

**Technical specifications** and precise information about XZe's components.

Consult these for detailed technical information.

- [Architecture Specification](reference/architecture.md) - **Complete system architecture (START HERE)**
- API reference documentation (coming soon)
- Configuration reference (coming soon)
- CLI command reference (coming soon)
- Data models and schemas (coming soon)

## Getting Started

### For First-Time Users

1. **Read the architecture**: Start with [architecture.md](reference/architecture.md) to understand XZe's design
2. **Follow a tutorial**: Complete a hands-on tutorial (coming soon)
3. **Refer as needed**: Use how-to guides and reference docs when you need them

### For Developers and Contributors

1. **Read AGENTS.md**: See [AGENTS.md](../AGENTS.md) for development guidelines
2. **Review architecture**: Study [architecture.md](reference/architecture.md) thoroughly
3. **Check implementations**: Review [implementations.md](explanation/implementations.md) for current progress

## Understanding the Diátaxis Framework

XZe's documentation follows [Diátaxis](https://diataxis.fr/), which organizes documentation into four distinct types:

| Type              | Purpose         | Question Answered         | Focus                    |
| ----------------- | --------------- | ------------------------- | ------------------------ |
| **Tutorials**     | Learning        | "Can you teach me to...?" | Teaching through doing   |
| **How-To Guides** | Problem-solving | "How do I...?"            | Achieving specific goals |
| **Explanation**   | Understanding   | "Why...?"                 | Context and background   |
| **Reference**     | Information     | "What is...?"             | Facts and specifications |

This structure ensures documentation serves its actual purpose rather than trying to be all things at once.

## Architecture Overview

XZe is built with a modular, API-first architecture:

```text
xze (binary)
├── xze-cli         CLI interface (pure API client)
├── xze-serve       REST API server (primary interface)
├── xze-sdk         Dual interface (HTTP client + direct API)
└── xze-core        Core business logic (no interface dependencies)
```

**Event Sources**:

- GitHub/GitLab webhooks
- Kafka/Redpanda message streaming

**Processing**:

- AI analysis using Ollama models
- Diátaxis-structured documentation generation
- Automatic Git operations and PR creation

For complete details, see [architecture.md](reference/architecture.md).

## Project Status

**Current Phase**: Architecture defined, implementation in progress

See [implementations.md](explanation/implementations.md) for detailed implementation history.

## Contributing

We welcome contributions! Please see:

- [AGENTS.md](../AGENTS.md) - **Required reading** for all AI agents and developers
- [Architecture](reference/architecture.md) - **Canonical source of truth** for system design
- [Implementations Log](explanation/implementations.md) - Record of all implementations

### Documentation Guidelines

When contributing documentation:

- **Categorize correctly**: Use the Diátaxis decision tree
- **Follow conventions**: lowercase_with_underscores.md for filenames (except README.md)
- **Update logs**: Append implementation summaries to [implementations.md](explanation/implementations.md)
- **Link appropriately**: Cross-reference between documentation types
- **No emojis in files**: Keep documentation accessible and professional

## Getting Help

- **GitHub Issues**: Report bugs and request features
- **Documentation Issues**: Report unclear or missing documentation
- **Architecture Questions**: Consult [architecture.md](reference/architecture.md) first

## Related Resources

### External Documentation

- [Diátaxis Framework](https://diataxis.fr/) - Documentation framework we follow
- [Ollama](https://ollama.ai/) - AI model platform we use
- [Rust Documentation](https://doc.rust-lang.org/) - Rust programming language

### Project Files

- [AGENTS.md](../AGENTS.md) - AI agent development guidelines
- [PLAN.md](../PLAN.md) - Project planning guidelines (if exists)
- [Cargo.toml](../Cargo.toml) - Rust workspace configuration

## About This Documentation

**Last Updated**: 2025-01-07

**Framework**: [Diátaxis](https://diataxis.fr/)

This documentation is itself an example of the Diátaxis framework that XZe implements. As XZe evolves, this documentation grows with it, demonstrating best practices for structured technical documentation.

---

**Ready to start?** Begin with the [Architecture Specification](reference/architecture.md) to understand XZe's complete design.
