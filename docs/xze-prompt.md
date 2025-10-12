# XZe Pipeline Documentation Tool

## Overview

XZe is a tool written in rust that uses open source models from ollama to analyze a services source code and doc in the service git repository and creates documentation for the project. The tool uses the Diátaxis Documentation Framework as the documentation layout for the pipeline-documentation repository. The tool can analyze the pipeline-documentation and assess if the documentation for the pipeline services exists and is up to date. It will make changes to the documentation as needed. In local mode the user will pass the tool the paths to the repositories that should be evaluated. In auto-mode the user will provide the tool a yaml file with the location of all the git repositories of the services that make up the pipeline. As repositories are updated the tool will eval the changes to the repo and see if there is a need to update the documentation. The tool will create PRs for the pipeline-documentation repossitory as changes are made so the Human engineers can review the changes.

## Features

### Modes of operation

- VSCode Agent that can use Copilot
- CLI
- Server

### Analyze repositories source code, configuration, and existing documentation

- Use models from ollama to analyze the contents of a services repository
- Use models from ollama to evaluate the Documentation in the pipeline-documentation repository 
- Use models from ollama to update the documentation

### Create Documentation

- Create Reference Docs for the services from the source code
- Create How-To Docs for the services from the source code
- Create Tutorials for the services from the source code
- Create Explanations for the architecture from the source code

### Logging

- Log all work to stdout using JSON structured logging

### Update Documentation

- Update Reference Docs for the services from the source code
- Update How-To Docs for the services from the source code
- Update Tutorials for the services from the source code
- Update Explanations for the architecture from the source code

#### Git Features

- Clone git repositories
- Create branches
- Commit changes
- Review changes


## Diátaxis is a way of thinking about and doing documentation.

It prescribes approaches to content, architecture and form that emerge from a systematic approach to understanding the needs of documentation users.

Diátaxis identifies four distinct needs, and four corresponding forms of documentation - tutorials, how-to guides, technical reference and explanation. It places them in a systematic relationship, and proposes that documentation should itself be organised around the structures of those needs.

## Usage

The tool should be able to be used from CLI or as a service

## Deployment

- docker
- docker-compose
- kubernetes