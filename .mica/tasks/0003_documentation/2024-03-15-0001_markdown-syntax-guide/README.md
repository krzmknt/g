---
title: Markdown Syntax Guide
status: Done
assignee: Documentation Team
reviewer: ''
planned_start: '2026-05-08'
planned_end: '2026-05-09'
actual_start: '2024-03-15'
actual_end: '2024-03-16'
dependencies: []
pinned: false
---
## Markdown Syntax Guide

This guide demonstrates various Markdown features supported in Mica tasks.

### Text Formatting

**Bold text** using `**bold**` or `__bold__`

*Italic text* using `*italic*` or `_italic_`

***Bold and italic*** using `***text***`

~~Strikethrough~~ using `~~text~~`

`Inline code` using backticks

### Headers

Use `#` for headers. Mica supports H1 through H6:

# H1 Header
## H2 Header
### H3 Header
#### H4 Header
##### H5 Header
###### H6 Header

### Lists

#### Unordered Lists
- Item 1
- Item 2
  - Nested item 2.1
  - Nested item 2.2
- Item 3

#### Ordered Lists
1. First item
2. Second item
   1. Nested item 2.1
   2. Nested item 2.2
3. Third item

#### Task Lists
- [x] Completed task
- [x] Another completed task
- [ ] Incomplete task
- [ ] Another incomplete task

### Links and References

[External link to GitHub](https://github.com)

[Internal link to another task](../getting-started/2024-01-01-0001_welcome-to-mica/README.md)

### Code Blocks

#### Inline Code
Use `backticks` for inline code.

#### Code Blocks with Syntax Highlighting

```javascript
function greetUser(name) {
  console.log(`Hello, ${name}!`);
  return `Welcome to Mica!`;
}

greetUser('Developer');
```

```python
def calculate_progress(completed, total):
    """Calculate completion percentage."""
    if total == 0:
        return 0
    return (completed / total) * 100

progress = calculate_progress(75, 100)
print(f"Progress: {progress}%")
```

```sql
SELECT p.name, p.price, c.name as category
FROM products p
JOIN categories c ON p.category_id = c.id
WHERE p.price > 100
ORDER BY p.price DESC;
```

### Blockquotes

> This is a blockquote. Use it for important notes, quotes, or callouts.
>
> You can have multiple paragraphs in a blockquote.

> **Note**: This is a styled note using blockquote with bold text.

> **Warning**: Use this pattern for warnings or important information.

### Horizontal Rules

Use three or more dashes for horizontal rules:

---

### Escaping Characters

Use backslash to escape special characters: \* \_ \# \[\]

### Best Practices

1. **Consistent Formatting**: Use consistent patterns for similar content
2. **Clear Headers**: Use descriptive headers for better navigation
3. **Code Examples**: Include relevant code examples for technical tasks
4. **Visual Breaks**: Use horizontal rules and spacing for readability
5. **Task Lists**: Use task lists for actionable items and checklists

This guide covers the most commonly used Markdown syntax in Mica!
