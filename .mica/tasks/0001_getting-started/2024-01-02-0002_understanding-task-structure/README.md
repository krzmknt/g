---
title: Understanding Task Structure
status: Done
assignee: New User
reviewer: ''
planned_start: '2026-02-24'
planned_end: '2026-02-25'
actual_start: '2024-01-02'
actual_end: '2024-01-03'
dependencies:
  - 0001_getting-started/2024-01-01-0001_welcome-to-mica
pinned: false
---
## Understanding Task Structure

Each task in Mica is a directory containing a `README.md` file with frontmatter and content.

### Directory Naming Convention

Tasks follow this naming pattern:
```
YYYY-MM-DD-NNNN_task-name/
└── README.md
```

- **YYYY-MM-DD**: Creation date
- **NNNN**: Sequential number (0001, 0002, etc.)
- **task-name**: Descriptive name using kebab-case

### Frontmatter Properties

The frontmatter (between `---` marks) defines task metadata:

```yaml
---
title: Task Title
status: InProgress | Completed | Backlog | Blocked
output: ''
assignee: Person Name
reviewer: Reviewer Name (optional)
size: XS | S | M | L | XL
planned_start: 'YYYY-MM-DD'
planned_end: 'YYYY-MM-DD'
actual_start: 'YYYY-MM-DD' (optional)
actual_end: 'YYYY-MM-DD' (optional)
dependencies: ['task-directory-name'] (optional)
pinned: true | false (optional)
---
```

### Content Section

After the frontmatter, write your task description, notes, and documentation using standard Markdown syntax.

### Best Practices

1. **Be Descriptive**: Use clear, descriptive titles and content
2. **Set Realistic Dates**: Plan your timelines carefully
3. **Track Dependencies**: Link related tasks using the dependencies field
4. **Update Status**: Keep task status current as work progresses
5. **Document Everything**: Include relevant notes, decisions, and resources
