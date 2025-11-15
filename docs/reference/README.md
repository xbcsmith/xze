# Reference

This directory contains technical reference documentation for the XZe project following the Diátaxis framework.

## What is Reference?

Reference documentation provides precise, technical specifications. It is:

- Information-oriented
- Accurate and up-to-date
- Structured for lookup, not learning
- Comprehensive in coverage

## Contents

### Architecture

- [architecture.md](architecture.md) - **Canonical architecture specification** (SOURCE OF TRUTH)

### Future Content

As XZe is implemented, this directory will contain:

- **API Reference**: REST API endpoint specifications (auto-generated from OpenAPI)
- **Configuration Reference**: Complete xze.yaml configuration options
- **CLI Reference**: Command-line interface documentation
- **Data Models**: Request/response schemas, internal data structures
- **Error Codes**: Complete error reference with remediation steps

## Current System

XZe is an API-first, event-driven documentation generation system. See [architecture.md](architecture.md) for:

- System overview and design principles
- Component design (API, SDK, CLI, core services)
- Data models and configuration schemas
- Event-driven architecture (webhooks, Kafka)
- API specifications and versioning
- Deployment architecture (Docker, Kubernetes)
- Security, monitoring, testing strategies

## API Versioning

All XZe APIs use version prefix: `/api/v1/*`

OpenAPI specification will be available at: `GET /api/v1/openapi.json`

## Contributing

When adding reference documents:

- Maintain technical accuracy
- Use precise language and specifications
- Include code examples with expected outputs
- Follow lowercase_with_underscores.md naming convention
- Update this README when adding new reference docs
