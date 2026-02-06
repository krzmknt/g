---
title: Chart Operations and Features
status: Done
assignee: Documentation Team
reviewer: ''
planned_start: '2026-05-13'
planned_end: '2026-05-15'
actual_start: ''
actual_end: ''
dependencies:
  - 0003_documentation/2024-03-18-0003_tables-and-data
pinned: false
---
## Chart Operations and Features

Comprehensive guide to using Mica's chart features and interface operations.

### Gantt Chart Navigation

#### Basic Controls
- **Zoom In/Out**: Use mouse wheel or zoom buttons
- **Pan**: Click and drag to move around the timeline
- **Fit to Screen**: Double-click to fit all tasks in view
- **Reset View**: Return to default zoom and position

#### Timeline Navigation
- **Time Scale**: Switch between days, weeks, months, quarters
- **Date Range**: Set custom start and end dates
- **Today Indicator**: Current date highlighted on timeline
- **Scroll**: Horizontal scroll for extended timelines

### Task Visualization

#### Task Bars
Tasks are displayed as horizontal bars showing:
- **Duration**: Bar length represents task duration
- **Progress**: Filled portion shows completion percentage
- **Status Colors**:
  - 🟢 Green: Completed tasks
  - 🔵 Blue: In Progress tasks
  - 🟡 Yellow: Backlog/Planned tasks
  - 🔴 Red: Blocked/Overdue tasks

#### Dependencies
- **Arrows**: Visual connections between dependent tasks
- **Critical Path**: Highlighted sequence of dependent tasks
- **Lag/Lead Time**: Gaps or overlaps between connected tasks

### Interactive Features

#### Task Selection
- **Single Click**: Select a task to view details
- **Multi-Select**: Ctrl+Click to select multiple tasks
- **Range Select**: Shift+Click to select task ranges

#### Task Editing
- **Drag to Move**: Drag task bars to change dates
- **Resize**: Drag task edges to adjust duration
- **Quick Edit**: Double-click to open task editor
- **Bulk Edit**: Edit multiple selected tasks

#### Filtering and Grouping
- **Filter by Status**: Show/hide tasks by completion status
- **Filter by Assignee**: View tasks for specific team members
- **Filter by Date Range**: Focus on specific time periods
- **Group by Category**: Organize tasks by project or category

### Chart Types and Views

#### Standard Gantt View
```
[Task 1    ████████████░░░░] 75%
[Task 2       ████████████████] 100%
[Task 3          ░░░░░░░░░░░░░░] 0%
Timeline: |---|---|---|---|---|
          Jan Feb Mar Apr May
```

#### Milestone View
- **Diamond Markers**: Key project milestones
- **Zero Duration**: Milestone tasks with no duration
- **Critical Dates**: Important deadlines and deliverables

#### Resource View
- **Workload Distribution**: See task allocation per person
- **Capacity Planning**: Identify over/under-allocated resources
- **Team Timeline**: View individual team member schedules

### Advanced Features

#### Critical Path Analysis
- **Longest Path**: Sequence of tasks that determines project duration
- **Float/Slack**: Available delay time for non-critical tasks
- **Critical Tasks**: Tasks that directly impact project completion

#### Baseline Comparison
- **Original Plan**: Compare current schedule with baseline
- **Variance Analysis**: Identify schedule deviations
- **Progress Tracking**: Monitor actual vs. planned progress

#### Resource Management
- **Allocation Tracking**: Monitor resource utilization
- **Conflict Detection**: Identify resource over-allocation
- **Leveling**: Automatic resource conflict resolution

### Export and Sharing

#### Export Options
- **PNG/JPG**: High-resolution chart images
- **PDF**: Printable document format
- **CSV**: Raw data export for analysis
- **Excel**: Spreadsheet format with data

#### Sharing Features
- **Public Links**: Share read-only chart views
- **Embed Code**: Include charts in external documents
- **Print View**: Optimized layout for printing
- **Presentation Mode**: Full-screen chart display

### Customization Options

#### Visual Themes
- **Color Schemes**: Choose from predefined color palettes
- **Dark/Light Mode**: Switch between interface themes
- **Custom Colors**: Define colors for specific categories
- **Branding**: Add logos and custom styling

#### Layout Settings
- **Column Width**: Adjust task list column sizes
- **Row Height**: Change task bar thickness
- **Font Size**: Adjust text readability
- **Grid Lines**: Show/hide timeline grid

### Keyboard Shortcuts

| Action | Shortcut | Description |
|:-------|:---------|:------------|
| Zoom In | Ctrl + Plus | Increase timeline zoom |
| Zoom Out | Ctrl + Minus | Decrease timeline zoom |
| Fit to Screen | Ctrl + 0 | Show all tasks |
| Select All | Ctrl + A | Select all visible tasks |
| Copy | Ctrl + C | Copy selected tasks |
| Paste | Ctrl + V | Paste tasks |
| Undo | Ctrl + Z | Undo last action |
| Redo | Ctrl + Y | Redo last undone action |
| Find | Ctrl + F | Search for tasks |
| Print | Ctrl + P | Print chart |

### Performance Tips

#### Large Projects
- **Lazy Loading**: Tasks load as needed for better performance
- **Viewport Optimization**: Only visible tasks are rendered
- **Data Pagination**: Split large datasets into manageable chunks

#### Browser Optimization
- **Memory Management**: Efficient handling of large timelines
- **Caching**: Improved loading times for repeat views
- **Progressive Loading**: Gradual data loading for smooth experience

### Troubleshooting

#### Common Issues
- **Slow Performance**: Reduce visible date range or filter tasks
- **Missing Dependencies**: Check task relationships and dates
- **Overlapping Tasks**: Review resource allocation and schedules
- **Export Problems**: Ensure modern browser with adequate memory

#### Best Practices
1. **Regular Saves**: Save changes frequently
2. **Backup Data**: Export project data regularly
3. **Browser Updates**: Keep browser updated for best performance
4. **Screen Resolution**: Use adequate screen size for complex charts

Master these chart operations to effectively manage and visualize your projects!
