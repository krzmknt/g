---
title: Images and Media Examples
status: Done
assignee: Documentation Team
reviewer: ''
planned_start: '2026-05-10'
planned_end: '2026-05-10'
actual_start: '2024-03-17'
actual_end: '2024-03-17'
dependencies:
  - 0003_documentation/2024-03-15-0001_markdown-syntax-guide
pinned: false
---
## Images and Media Examples

Learn how to include images and media in your Mica tasks.

### Image Syntax

#### Basic Image
```markdown
![Alt text](path/to/image.png)
```

#### Image with Title
```markdown
![Alt text](path/to/image.png "Image title")
```

### Sample Images

#### Project Architecture Diagram
![Project Architecture](../../assets/images/sample-architecture.png "Sample project architecture diagram")

*Figure 1: Sample project architecture showing the relationship between frontend, backend, and database components.*

#### User Interface Mockup
![UI Mockup](../../assets/images/sample-ui-mockup.png "User interface mockup")

*Figure 2: User interface mockup for the main dashboard.*

### Image Best Practices

1. **Descriptive Alt Text**: Always include meaningful alt text for accessibility
2. **Appropriate File Formats**:
   - PNG for screenshots and diagrams
   - JPG for photos
   - SVG for scalable graphics
3. **File Size**: Optimize images to keep file sizes reasonable
4. **Relative Paths**: Use relative paths from the task directory
5. **Captions**: Add descriptive captions below images

### Supported Media Types

| Type | Extensions | Use Case |
|------|------------|----------|
| Images | .png, .jpg, .jpeg, .gif, .svg | Screenshots, diagrams, photos |
| Documents | .pdf | Specifications, reports |
| Videos | .mp4, .webm | Demonstrations, tutorials |

### Image Organization

Organize images in a logical structure:

```
.mica/
├── assets/
│   ├── images/
│   │   ├── screenshots/
│   │   ├── diagrams/
│   │   └── mockups/
│   └── documents/
└── examples/
```

### Embedding External Media

You can also link to external images and media:

```markdown
![External Image](https://example.com/image.png)
```

**Note**: External media requires internet access and may not be available offline.

### Image Accessibility

- Use descriptive alt text that explains the image content
- Ensure sufficient color contrast in diagrams
- Provide text alternatives for complex images
- Consider screen reader users when describing visual content

Visual content enhances task documentation and helps communicate complex ideas effectively!
