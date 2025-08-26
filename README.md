# Ken - GitLab Assistant

AI-powered interactive terminal for GitLab issue management.

## Setup

### Prerequisites
- Rust toolchain: https://rustup.rs/
- Node.js (for GitLab MCP server)

### Installation
```bash
# Clone repository
git clone <your-repo>
cd ken

# Initialize submodules
git submodule update --init --recursive

# Build the application
cargo build --release

# Install GitLab MCP server dependencies
cd gitlab-mcp && npm install && cd ..
```

### Environment Setup
Create GitLab personal access token:
1. Go to your GitLab → Settings → Access Tokens
2. Create token with `api` scope
3. Save the token for login

## Usage

Start the interactive terminal:
```bash
cargo run
```

### Basic Commands
- `/login` - Authenticate with GitLab
- `/projects` - List available projects  
- `/project <id>` - Set default project
- `/update-context` - Fetch project context
- `/context` - View cached context
- `<natural language>` - Query issues with AI
- `/help` - Show all commands
- `exit` - Quit

### Example Session
```
🚀 Ken - GitLab Assistant
Starting interactive mode...

✅ Authenticated to: https://gitlab.com
📁 Current project: my-project
💡 Type '/help' for commands or 'exit' to quit.
⌨️  Use TAB for autocompletion, UP/DOWN for history.

Ken> /login
🔐 GitLab Authentication Setup
Enter your GitLab URL (e.g., https://gitlab.com): https://gitlab.com
Enter your GitLab personal access token: 
✅ Login successful!

Ken> /projects
📋 Fetching projects from GitLab...
📂 Available Projects:
  • my-project (ID: 123, Path: user/my-project)
  • another-project (ID: 456, Path: user/another-project)

Ken> /project 123
✅ Default project set to: 123

Ken> /update-context
🔄 Updating project context from GitLab...
✅ Project context updated and cached successfully!

Ken> show me open bugs
🤖 Processing query...
📝 Response:
Here are the open bugs for your project:
- Issue #45: Login page crashes on mobile
- Issue #67: API timeout errors
- Issue #89: Database connection issues

Ken> exit
👋 Goodbye!
```