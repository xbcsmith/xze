# Diataxis Automation

This document explains how XZe automates the organization of documentation using the Diataxis framework.

## The Diataxis Framework

[Diataxis](https://diataxis.fr/) is a systematic framework for technical documentation authoring. It identifies four distinct modes of documentation needs:

| Type | Purpose | Orientation |
|------|---------|-------------|
| **Tutorials** | Learning | Learning-oriented |
| **How-To Guides** | Solving a problem | Task-oriented |
| **Reference** | Information | Information-oriented |
| **Explanation** | Understanding | Understanding-oriented |

## Automated Classification

XZe uses Large Language Models (LLMs) to automatically classify existing documentation files into these categories.

### How it works

1.  **Content Sampling**: The system reads the beginning of each file (first 500 characters) to understand its tone and content.
2.  **LLM Analysis**: A prompt is sent to the LLM describing the four Diataxis types.
3.  **Decision**: The LLM determines the most likely category for the file and provides a reasoning.
4.  **Path Proposal**: Based on the category, the system proposes a new file path (e.g., moving `api.md` to `reference/api.md`).

## Benefits

- **Consistency**: Ensures all documentation follows a standard structure.
- **Discoverability**: Users know where to look for specific types of information.
- **Maintainability**: Keeps the documentation codebase organized as it grows.
