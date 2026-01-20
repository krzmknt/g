# UI Layout Design

## Overview

Multi-pane layout inspired by lazygit, providing simultaneous visibility of different Git aspects.

## Main Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│ g - [repo-name] | branch: main | ↑2 ↓0 | ✓ clean                        │
├───────────────────────┬─────────────────────────────────────────────────┤
│                       │                                                 │
│   [1] Status          │                                                 │
│   ─────────────────   │                                                 │
│   Staged (2)          │   [4] Main Panel                                │
│     M  src/app.rs     │   ───────────────────────────────               │
│     A  src/new.rs     │                                                 │
│   Unstaged (1)        │   (Content changes based on                     │
│     M  README.md      │    selected item in left panels)                │
│                       │                                                 │
├───────────────────────┤   - Diff view                                   │
│                       │   - Commit details                              │
│   [2] Branches        │   - File content                                │
│   ─────────────────   │   - Help                                        │
│ ● main                │                                                 │
│   feature/auth        │                                                 │
│   feature/ui          │                                                 │
│   bugfix/123          │                                                 │
│                       │                                                 │
├───────────────────────┤                                                 │
│                       │                                                 │
│   [3] Commits         │                                                 │
│   ─────────────────   │                                                 │
│   a1b2c3d Add auth    │                                                 │
│   e4f5g6h Fix bug     │                                                 │
│   i7j8k9l Initial     │                                                 │
│                       │                                                 │
├───────────────────────┴─────────────────────────────────────────────────┤
│ [?] help  [q] quit  [1-4] switch panel  [Enter] select  [/] search      │
└─────────────────────────────────────────────────────────────────────────┘
```

## Panel System

### Panel Types

```rust
pub enum PanelType {
    Status,     // Staged/Unstaged files
    Branches,   // Branch list
    Commits,    // Commit history
    Main,       // Diff/details view
    Stash,      // Stash list (toggle)
    Tags,       // Tag list (toggle)
}
```

### Panel Structure

```rust
pub struct Panel {
    pub panel_type: PanelType,
    pub title: String,
    pub focused: bool,
    pub rect: Rect,
    pub content: PanelContent,
}

pub enum PanelContent {
    List(ListState),
    Diff(DiffState),
    Text(TextState),
}

pub struct ListState {
    pub items: Vec<ListItem>,
    pub selected: usize,
    pub offset: usize,
}
```

## Layout Manager

### Responsive Layout

```rust
pub struct LayoutManager {
    pub panels: Vec<Panel>,
    pub focused_panel: usize,
}

impl LayoutManager {
    pub fn calculate_layout(&mut self, width: u16, height: u16) {
        // Reserve space for header and footer
        let content_height = height.saturating_sub(3);  // 1 header + 2 footer

        // Calculate widths
        let left_width = (width as f32 * 0.30).min(40.0) as u16;
        let right_width = width.saturating_sub(left_width + 1);  // 1 for separator

        // Left side: split into 3 panels vertically
        let panel_height = content_height / 3;

        self.panels[0].rect = Rect::new(0, 1, left_width, panel_height);           // Status
        self.panels[1].rect = Rect::new(0, 1 + panel_height, left_width, panel_height);  // Branches
        self.panels[2].rect = Rect::new(0, 1 + panel_height * 2, left_width, content_height - panel_height * 2);  // Commits

        // Right side: main panel
        self.panels[3].rect = Rect::new(left_width + 1, 1, right_width, content_height);  // Main
    }
}
```

### Minimum Size Handling

```rust
const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 24;

impl LayoutManager {
    pub fn check_size(&self, width: u16, height: u16) -> Result<(), SizeError> {
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            return Err(SizeError::TooSmall {
                current: (width, height),
                minimum: (MIN_WIDTH, MIN_HEIGHT),
            });
        }
        Ok(())
    }
}
```

## Header Bar

```
┌─────────────────────────────────────────────────────────────────────────┐
│ g - [repo-name] | branch: main | ↑2 ↓0 | ✓ clean                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### Components

| Component             | Description                 |
| --------------------- | --------------------------- |
| `g`                   | App name                    |
| `[repo-name]`         | Current repository name     |
| `branch: main`        | Current branch              |
| `↑2 ↓0`               | Commits ahead/behind remote |
| `✓ clean` / `● dirty` | Working tree status         |

```rust
pub struct HeaderBar {
    pub repo_name: String,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub is_clean: bool,
}

impl Widget for HeaderBar {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let status_icon = if self.is_clean { "✓" } else { "●" };
        let status_text = if self.is_clean { "clean" } else { "dirty" };

        let text = format!(
            " g - [{}] | branch: {} | ↑{} ↓{} | {} {}",
            self.repo_name,
            self.branch,
            self.ahead,
            self.behind,
            status_icon,
            status_text
        );

        buf.set_string(area.x, area.y, &text, Style::default().bold());
    }
}
```

## Footer Bar (Help Line)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ [?] help  [q] quit  [1-4] switch panel  [Enter] select  [/] search      │
└─────────────────────────────────────────────────────────────────────────┘
```

### Context-Sensitive Help

```rust
pub struct FooterBar {
    pub hints: Vec<KeyHint>,
}

pub struct KeyHint {
    pub key: String,
    pub action: String,
}

impl FooterBar {
    pub fn for_panel(panel_type: PanelType) -> Self {
        let hints = match panel_type {
            PanelType::Status => vec![
                KeyHint::new("Enter", "stage/unstage"),
                KeyHint::new("a", "stage all"),
                KeyHint::new("c", "commit"),
            ],
            PanelType::Branches => vec![
                KeyHint::new("Enter", "checkout"),
                KeyHint::new("n", "new branch"),
                KeyHint::new("d", "delete"),
            ],
            // ... other panels
        };

        Self { hints }
    }
}
```

## Status Panel

```
┌─────────────────────┐
│ [1] Status          │
│ ─────────────────── │
│ Staged (2)          │
│   M  src/app.rs     │
│   A  src/new.rs     │
│ Unstaged (1)        │
│   M  README.md      │
│ Untracked (1)       │
│   ?  notes.txt      │
└─────────────────────┘
```

### Status Icons

| Icon | Meaning   |
| ---- | --------- |
| `M`  | Modified  |
| `A`  | Added     |
| `D`  | Deleted   |
| `R`  | Renamed   |
| `C`  | Copied    |
| `?`  | Untracked |
| `!`  | Ignored   |

```rust
pub struct StatusPanel {
    pub staged: Vec<StatusItem>,
    pub unstaged: Vec<StatusItem>,
    pub untracked: Vec<StatusItem>,
    pub selected_section: Section,
    pub selected_index: usize,
}

pub enum Section {
    Staged,
    Unstaged,
    Untracked,
}

pub struct StatusItem {
    pub path: String,
    pub status: FileStatus,
}
```

## Branch Panel

```
┌─────────────────────┐
│ [2] Branches        │
│ ─────────────────── │
│ Local               │
│ ● main              │
│   feature/auth      │
│   feature/ui        │
│ Remote              │
│   origin/main       │
│   origin/develop    │
└─────────────────────┘
```

### Visual Indicators

- `●` Current branch
- Color coding: local vs remote

```rust
pub struct BranchPanel {
    pub local_branches: Vec<BranchItem>,
    pub remote_branches: Vec<BranchItem>,
    pub show_remote: bool,
    pub selected: usize,
}

pub struct BranchItem {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub ahead_behind: Option<(usize, usize)>,
}
```

## Commit Panel

```
┌─────────────────────┐
│ [3] Commits         │
│ ─────────────────── │
│ a1b2c3d Add auth    │
│ e4f5g6h Fix bug #12 │
│ i7j8k9l Update deps │
│ m1n2o3p Initial     │
└─────────────────────┘
```

```rust
pub struct CommitPanel {
    pub commits: Vec<CommitItem>,
    pub selected: usize,
    pub offset: usize,
    pub search_query: Option<String>,
}

pub struct CommitItem {
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub date: String,
}
```

## Main Panel (Diff View)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ [4] Diff: src/app.rs                                                    │
│ ─────────────────────────────────────────────────────────────────────── │
│ @@ -10,7 +10,8 @@ fn main() {                                            │
│      let config = Config::load();                                       │
│      let app = App::new(config);                                        │
│ -    app.run();                                                         │
│ +    if let Err(e) = app.run() {                                        │
│ +        eprintln!("Error: {}", e);                                     │
│ +    }                                                                  │
│  }                                                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

### Diff Coloring

| Line Type          | Color   |
| ------------------ | ------- |
| Addition (`+`)     | Green   |
| Deletion (`-`)     | Red     |
| Hunk header (`@@`) | Cyan    |
| Context            | Default |

```rust
pub struct DiffPanel {
    pub file_path: String,
    pub hunks: Vec<Hunk>,
    pub scroll_offset: usize,
    pub selected_hunk: Option<usize>,
}

impl DiffPanel {
    fn line_style(&self, line: &DiffLine) -> Style {
        match line.line_type {
            LineType::Addition => Style::default().fg(Color::Green),
            LineType::Deletion => Style::default().fg(Color::Red),
            LineType::Context => Style::default(),
        }
    }
}
```

## Modal Dialogs

### Confirmation Dialog

```
┌───────────────────────────────┐
│  Delete branch 'feature/x'?  │
│                               │
│      [Y] Yes    [N] No        │
└───────────────────────────────┘
```

### Input Dialog

```
┌───────────────────────────────┐
│  Commit message:              │
│  ┌───────────────────────────┐│
│  │ Fix authentication bug    ││
│  └───────────────────────────┘│
│      [Enter] confirm          │
└───────────────────────────────┘
```

### Search Dialog

```
┌───────────────────────────────┐
│  Search commits:              │
│  /fix bug█                    │
│                               │
│  [Enter] search  [Esc] cancel │
└───────────────────────────────┘
```

```rust
pub enum Dialog {
    Confirm {
        message: String,
        on_confirm: Action,
    },
    Input {
        prompt: String,
        value: String,
        cursor: usize,
        on_submit: fn(String) -> Action,
    },
    Search {
        query: String,
        cursor: usize,
    },
}
```

## Help Overlay

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Help                                       │
├─────────────────────────────────────────────────────────────────────────┤
│  Navigation                    │  Actions                               │
│  ───────────                   │  ───────                               │
│  j/↓     Move down             │  Enter   Select/toggle                 │
│  k/↑     Move up               │  Space   Stage/unstage                 │
│  h/←     Previous panel        │  c       Commit                        │
│  l/→     Next panel            │  p       Push                          │
│  1-4     Jump to panel         │  P       Pull                          │
│  g       Go to top             │  m       Merge                         │
│  G       Go to bottom          │  r       Rebase                        │
│                                │  /       Search                        │
│                                │  ?       Toggle help                   │
│                                │  q       Quit                          │
├─────────────────────────────────────────────────────────────────────────┤
│                           Press any key to close                        │
└─────────────────────────────────────────────────────────────────────────┘
```

```rust
pub struct HelpOverlay {
    pub visible: bool,
}

impl Widget for HelpOverlay {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Center the help box
        let width = 75.min(area.width - 4);
        let height = 20.min(area.height - 4);
        let x = (area.width - width) / 2;
        let y = (area.height - height) / 2;

        let rect = Rect::new(x, y, width, height);

        // Draw semi-transparent background
        // Draw bordered box with help content
    }
}
```

## Panel Focus Management

```rust
impl LayoutManager {
    pub fn focus_next(&mut self) {
        self.panels[self.focused_panel].focused = false;
        self.focused_panel = (self.focused_panel + 1) % self.panels.len();
        self.panels[self.focused_panel].focused = true;
    }

    pub fn focus_previous(&mut self) {
        self.panels[self.focused_panel].focused = false;
        self.focused_panel = if self.focused_panel == 0 {
            self.panels.len() - 1
        } else {
            self.focused_panel - 1
        };
        self.panels[self.focused_panel].focused = true;
    }

    pub fn focus_panel(&mut self, index: usize) {
        if index < self.panels.len() {
            self.panels[self.focused_panel].focused = false;
            self.focused_panel = index;
            self.panels[self.focused_panel].focused = true;
        }
    }
}
```
