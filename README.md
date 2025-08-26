# Ken - GitLab Issue Management Assistant

## Commands

### Authentication

#### Login
```bash
ken auth login
```

**Expected Output:**
```console
🔐 Ken - GitLab Authentication Setup

GitLab Authentication Setup
----------------------------
Enter your GitLab URL (e.g., https://gitlab.com): repos.fixstars.com

To create a personal access token:
1. Go to https://repos.fixstars.com/profile/personal_access_tokens
2. Create a token with 'api' scope
3. Copy the token and paste it below

Enter your GitLab personal access token: 

Enter default project ID (optional, press Enter to skip): aibooster

Verifying credentials...
✓ Successfully authenticated as: username
✅ Configuration saved successfully!

You can now use Ken to manage GitLab issues.
```

#### Check Status
```bash
ken auth status
```

**Expected Output:**
```
✅ Authenticated to: https://repos.fixstars.com
Default project: aibooster
✓ Successfully authenticated as: username
Token is valid and working.
```

#### Logout
```bash
ken auth logout
```

**Expected Output:**
```
✅ Logged out successfully. Credentials removed.
```

### Project Management

#### List Available Projects
```bash
ken project list
ken project list --search "aibooster"
ken project list --mine
```

**Expected Output:**
```
📋 Fetching projects from GitLab...

📂 Available Projects:
─────────────────────
  • aibooster (ID: 12345, Path: namespace/aibooster)
  • project-name (ID: 67890, Path: namespace/project-name)
  ... and 15 more projects

💡 Tip: Use 'ken project set <project_id>' to set a default project
   You can use either the numeric ID or the path (namespace/project)
```

#### Set Default Project
```bash
ken project set "namespace/project-name"
# or
ken project set 12345
```

**Expected Output:**
```
✅ Default project set to: namespace/project-name
```

#### Show Current Default Project
```bash
ken project current
```

**Expected Output:**
```
📁 Current default project: namespace/project-name
```

#### Update Project Context
```bash
ken project update-context
```

**Expected Output:**
```
🔄 Updating context for project: namespace/project-name
🔄 Refreshing project context...
✅ Context updated successfully!
Project context refreshed successfully with 15 labels, 8 users, and 3 milestones.

📊 Context Summary:
## Project Context for namespace/project-name

**Available Labels:**
- `bug`: Issues that are bugs (5)
- `feature`: New feature requests (3)
- `po-related`: Product Owner related tasks (2)
...

**Project Members:**
- `irdali.durrani` (Developer): Irdali Durrani
- `john.doe` (Maintainer): John Doe
...

*Context last updated: 2025-01-XX...*
```

### Issue Queries

#### Query Issues with Natural Language
```bash
ken query "What issues are assigned to irdali.durrani?"
ken query "Show me all open issues"
ken query "Find issues related to authentication"

# Query specific project (overrides default)
ken query "What are the critical bugs?" --project "namespace/other-project"
```

**Expected Output:**
```
📁 Using default project: namespace/project-name
🔍 Processing query: What issues are assigned to irdali.durrani?

Based on the search results, here are the issues assigned to irdali.durrani:
1. Issue #123: Fix authentication bug (Status: opened)
2. Issue #124: Implement new feature (Status: opened)
...
```

## Recommended Workflow

### 1. First Time Setup
```bash
# Authenticate with GitLab
ken auth login

# Set default project
ken project set "your-namespace/project-name"

# Update project context (fetches labels, users, team info)
ken project update-context
```

### 2. Daily Usage
```bash
# Smart queries that use project context
ken query "What issues is the backend team working on?"
ken query "Show me high priority bugs"
ken query "What issues are assigned to PO unit?"

# Update context when project structure changes
ken project update-context
```

**Key Benefits:**
- **Context-aware**: Ken knows your project's actual labels, users, and patterns
- **No guessing**: Uses real GitLab data instead of assuming label names
- **Smart routing**: Automatically tries multiple search strategies based on your project's structure
- **Cached efficiency**: Context is fetched once and reused until refreshed

### Issue Management (Coming Soon)

#### Create Issue from Natural Language
```bash
ken issue "The app crashes when uploading files larger than 10MB"
```

#### Summarize Issue
```bash
ken summarize <issue_id>
```

#### Suggest Assignee
```bash
ken suggest <issue_id>
```

#### Check User Workload
```bash
ken workload @username
```