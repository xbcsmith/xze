# Explanation

This directory contains understanding-oriented documentation for the XZe project following the Diátaxis framework.

## What is Explanation?

Explanation documents provide context, background, and reasoning behind the XZe system. They help you understand:

- Why architectural decisions were made
- How components fit together conceptually
- Trade-offs and alternatives considered
- Design patterns and principles used

## Contents

### Implementation Log

- [implementations.md](implementations.md) - Chronological log of all implementations (REQUIRED: all agents append here)

### Future Content

As XZe development progresses, this directory will contain:

- Architecture decision records (ADRs)
- Design rationale for major features
- Conceptual explanations of the AI-powered documentation system
- Event-driven architecture explanations
- API-first design principles

## Current Architecture

XZe is an AI-powered documentation generation tool with:

- **API-First Design**: REST API (xze-serve) as primary interface
- **Event-Driven**: Webhooks (GitHub/GitLab) and Kafka (Redpanda) trigger documentation updates
- **AI-Powered**: Uses Ollama models to analyze code and generate Diátaxis-structured documentation
- **Modular Crates**: xze-core (domain logic), xze-serve (API), xze-sdk (library), xze-cli (client)

See [architecture.md](../reference/architecture.md) for complete technical specification.

## Contributing

When adding explanation documents:

- Focus on "why" and "what" rather than "how"
- Provide context and background
- Discuss alternatives and trade-offs
- Link to related reference, tutorials, and how-to docs
- Follow lowercase_with_underscores.md naming convention
- Append implementation summaries to `implementations.md`
