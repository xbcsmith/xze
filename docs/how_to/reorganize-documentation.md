# How-To: Reorganize Documentation

This guide explains how to use the automated Diataxis reorganization tool to structure your documentation.

## Overview

The `reorg` command uses an LLM to analyze your documentation files and propose a new directory structure based on the [Diataxis framework](https://diataxis.fr/):
- **tutorials/**: Learning-oriented lessons.
- **how-to/**: Problem-oriented guides.
- **reference/**: Information-oriented technical descriptions.
- **explanation/**: Understanding-oriented discussions.

## Steps

1.  **Generate a Plan**:

    Run the command on your documentation directory. By default, it only prints the proposed plan.

    ```bash
    xze reorg ./docs
    ```

    Output example:
    ```text
    Proposed Reorganization Plan:
    ================================================================================
    MOVE: ./docs/getting_started.md -> ./docs/tutorials/getting_started.md
          Reason: This is a step-by-step lesson for beginners.
    --------------------------------------------------------------------------------
    MOVE: ./docs/api_spec.md -> ./docs/reference/api_spec.md
          Reason: This describes technical details of the API.
    --------------------------------------------------------------------------------
    ```

2.  **Review the Plan**: Check the proposed moves and reasons. Ensure they make sense for your project.

3.  **Execute Dry Run**:

    Simulate the execution to ensure no file conflicts or permission issues.

    ```bash
    xze reorg ./docs --execute --dry-run
    ```

4.  **Apply Changes**:

    Once satisfied, execute the plan to move the files.

    ```bash
    xze reorg ./docs --execute
    ```

## Tips

- **Backup**: Always commit your changes to Git before running the reorganization.
- **Model**: Use a capable model (like `llama3` or `gpt-4`) for better classification accuracy.
