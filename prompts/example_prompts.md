# Ken Agent - Example Prompts for Different Use Cases

## 1. Natural Language to Issue Conversion

### User Input:
"The application crashes when uploading files larger than 10MB. It happens consistently on the /api/upload endpoint and shows a timeout error after 30 seconds."

### Prompt to LLM:
```
Convert this bug report into a structured GitLab issue:
"The application crashes when uploading files larger than 10MB. It happens consistently on the /api/upload endpoint and shows a timeout error after 30 seconds."

Create a JSON response with title, description (including reproduction steps, expected behavior, actual behavior), appropriate labels, and severity assessment.
```

### Expected Output:
```json
{
  "title": "File upload fails with timeout for files >10MB on /api/upload",
  "description": "## Description\nThe application crashes when uploading files larger than 10MB through the `/api/upload` endpoint.\n\n## Reproduction Steps\n1. Navigate to file upload interface\n2. Select a file larger than 10MB\n3. Attempt to upload via /api/upload endpoint\n4. Wait for response\n\n## Expected Behavior\nFile should upload successfully regardless of size (within reasonable limits)\n\n## Actual Behavior\n- Upload fails after 30 seconds\n- Timeout error is displayed\n- Application crashes\n\n## Environment\n- Endpoint: `/api/upload`\n- File size threshold: >10MB\n- Timeout: 30 seconds\n\n## Impact\nUsers cannot upload large files, blocking critical functionality",
  "labels": ["bug", "backend", "api", "file-upload", "priority::high"],
  "severity": "S2"
}
```

## 2. Issue Summarization

### User Input:
"Summarize this issue: [long issue description about authentication problems, OAuth integration, token refresh failures, and user session management across multiple paragraphs]"

### Prompt to LLM:
```
Summarize this GitLab issue concisely. Extract:
1. Core problem (2-3 sentences)
2. Key technical details
3. Impact on users
4. Suggested next steps

[Issue content here]
```

## 3. Smart Assignment Suggestion

### User Input:
"Who should handle this authentication bug?"

### Context Provided:
```
Team members:
- Alice: Frontend specialist, currently has 3 open issues
- Bob: Backend/Auth expert, currently has 5 open issues, worked on auth system
- Charlie: Full-stack, currently has 2 open issues, new to team
```

### Prompt to LLM:
```
Based on this context, suggest the best assignee for an authentication bug:
[Team member details]
[Issue details]

Provide assignee recommendation with confidence level and reasoning.
```

## 4. Workload Summary

### User Input:
"What is Bob working on?"

### Prompt to LLM:
```
Summarize Bob's current workload based on these open issues:
1. Issue #123: Critical auth bug (Due: Tomorrow)
2. Issue #145: Implement refresh token (In Progress)
3. Issue #167: API documentation update (Backlog)

Format as: current focus, upcoming deadlines, and potential bottlenecks.
```

## 5. Duplicate Detection

### User Input:
"Check if this issue is a duplicate: Login fails with 401 error"

### Prompt to LLM:
```
Compare this new issue against existing issues and identify potential duplicates:

New issue: "Login fails with 401 error"

Existing issues:
1. #89: "Authentication returns 401 on valid credentials"
2. #92: "User cannot login after password reset"
3. #95: "401 unauthorized error in production"

Assess similarity and recommend if this should be marked as duplicate, related, or unique.
```

## 6. Auto-labeling

### User Input:
"Auto-label this issue: Performance degradation in dashboard loading, takes 15 seconds to render charts with large datasets"

### Prompt to LLM:
```
Analyze this issue description and suggest appropriate GitLab labels:
"Performance degradation in dashboard loading, takes 15 seconds to render charts with large datasets"

Consider: issue type, component affected, priority, technical area.
Return as a list of labels following GitLab conventions.
```

### Expected Output:
```json
{
  "labels": ["performance", "frontend", "dashboard", "charts", "priority::medium", "type::bug"]
}
```

## 7. Progress Report Generation

### Prompt to LLM:
```
Generate a weekly team progress report from this data:
- New issues created: 15
- Issues resolved: 12
- Critical bugs fixed: 3
- In progress: 8
- Team members: 5
- Notable achievements: Completed auth system refactor

Format as a concise summary suitable for team standup.
```

## Tips for Effective Prompts

1. **Be Specific**: Include exact format requirements
2. **Provide Context**: Give relevant background information
3. **Set Boundaries**: Specify length limits and must-include elements
4. **Request Structure**: Ask for JSON/markdown when needed for parsing
5. **Include Examples**: Show desired output format when complex