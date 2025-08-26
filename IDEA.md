# Ken: AI-Powered Bug Triage and Developer Assistant

## Overview

Ken is an AI-powered developer assistant designed to streamline bug triage, issue management, and team coordination within GitLab. It combines natural language processing, intelligent ownership suggestions, and context-aware summaries to make managing issues easier and faster. Ken will be available as both a **CLI tool** and a **GitLab bot**, ensuring flexibility in how developers and teams interact with it.

## Goals

* Reduce friction in creating, assigning, and tracking issues.
* Provide actionable insights into workload distribution and progress.
* Integrate seamlessly with GitLab’s existing features while enhancing usability.

## Key Features

### 1. Natural Language Issue Creation

* Developers can describe an issue in plain English (or Japanese, etc.).
* Ken automatically converts this into a structured GitLab issue using a predefined format (e.g., title, description, labels, severity, reproduction steps).
* Example:

  ```bash
  ken issue "App crashes when uploading large files"
  ```

  → Creates issue with title, description, and categorized labels.

### 2. Smart Assignment Suggestions

* Based on historical GitLab data, Ken suggests the most relevant developer/team to handle the issue.
* Uses patterns from past issues (e.g., who fixed similar bugs, code ownership, commit history).
* Provides confidence scores for assignments.

### 3. User Workload Summarization

* Summarizes what each developer is currently working on:

  * Open issues assigned to them.
  * Recent updates and progress.
  * Any potential blockers.
* Example command:

  ```bash
  ken summary @alice
  ```

  → Shows Alice’s active issues and recent updates.

### 4. Issue Summarization & Root Cause Hints

* Summarizes lengthy issue reports for quick understanding.
* Suggests potential root causes by analyzing error logs, tags, and commit history.
* Can link to relevant issues or merge requests for context.

### 5. Duplicate Detection

* Flags potential duplicate issues by comparing new reports against existing issues.
* Suggests merging or linking related issues.

### 6. Progress Insights & Reporting

* Generates a daily/weekly digest for the team:

  * New issues created.
  * Issues resolved.
  * Top contributors.
  * Open critical bugs.
* Can be delivered via CLI command or bot message in GitLab.

### 7. Multi-Interface Availability

* **CLI Tool**: For developers who prefer terminal workflows.
* **GitLab Bot**: For teams who want updates and interactions directly within GitLab.

## Example CLI Workflow

```bash
# Create a new issue from natural language
ken issue "API response time spikes during peak traffic"

# Get assignment suggestion
ken suggest 123  # (where 123 is the issue ID)

# Summarize developer activity
ken summary @bob

# Generate weekly report
ken report --weekly
```

## Example Bot Workflow

* Commenting on a new issue:

  > @ken summarize
  > → Bot replies with a short summary + suggested assignee.

* Commenting on a user profile:

  > @ken what is @charlie working on?
  > → Bot replies with active tasks and recent updates.

## Implementation Plan

### Phase 1: Core Features (MVP)
1. **LLM API Client** - Integration with internal Fixstars LLM API (http://llm-api.fixstars.com/)
2. **GitLab API Integration** - Basic CRUD operations for issues
3. **Natural Language Issue Creation** - Convert plain text to structured GitLab issues via CLI
4. **Issue Summarization** - Use LLM to summarize lengthy issue descriptions

### Phase 2: Enhancement Features
5. **Smart Assignment Suggestion** - Analyze git history and past issues for ownership patterns
6. **User Workload Summarization** - Display developer's active issues and recent progress
7. **Auto-labeling** - Automatically categorize issues based on content analysis

### Phase 3: Advanced Features
8. **Duplicate Detection** - Identify similar existing issues using semantic similarity
9. **Progress Reports Generation** - Create team digests (daily/weekly summaries)
10. **GitLab Bot Interface** - Interactive bot for GitLab comments and mentions

## Technical Considerations

* **LLM Backend**: Internal Fixstars API (http://llm-api.fixstars.com/) with ChatGPT-like interface
* **Permissions**: Requires GitLab API tokens with appropriate scopes (issues, projects, users)
* **Integration**: Works across projects but respects GitLab access controls
* **Language**: Rust CLI for performance and reliability
* **Deployment**: 
  - Phase 1-2: Standalone CLI tool
  - Phase 3: Dockerized bot for GitLab integration

## Why Ken?

While GitLab already offers issue templates, assignment, and reporting, these features require manual effort. Ken adds intelligence, automation, and natural language interaction—removing friction and giving developers more time to code.

## Future Extensions

* **Code Insights**: Suggest relevant files or lines of code linked to the issue.
* **Predictive Resolution Time**: Estimate how long an issue may take based on past trends.
* **Cross-Project Summaries**: Summarize issues across multiple GitLab projects.
* **Slack/Teams Integration**: Extend beyond GitLab for cross-platform communication.

---

**Name:** Ken (賢)
