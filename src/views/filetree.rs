use crate::git::{FileTreeEntry, FileTreeStatus};
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct FileTreeView {
    pub entries: Vec<FileTreeEntry>,
    pub flat_entries: Vec<FlatEntry>,
    pub selected: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct FlatEntry {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    pub status: Option<FileTreeStatus>,
}

impl FileTreeView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            flat_entries: Vec::new(),
            selected: 0,
            offset: 0,
        }
    }

    pub fn update(&mut self, entries: Vec<FileTreeEntry>) {
        self.entries = entries;
        self.rebuild_flat_list();
        if self.selected >= self.flat_entries.len() && !self.flat_entries.is_empty() {
            self.selected = self.flat_entries.len() - 1;
        }
    }

    fn rebuild_flat_list(&mut self) {
        self.flat_entries.clear();
        self.flatten_entries(&self.entries.clone(), 0);
    }

    fn flatten_entries(&mut self, entries: &[FileTreeEntry], depth: usize) {
        for entry in entries {
            self.flat_entries.push(FlatEntry {
                path: entry.path.clone(),
                name: entry.name.clone(),
                depth,
                is_dir: entry.is_dir,
                expanded: entry.expanded,
                status: entry.status,
            });

            if entry.is_dir && entry.expanded {
                self.flatten_entries(&entry.children, depth + 1);
            }
        }
    }

    pub fn selected_entry(&self) -> Option<&FlatEntry> {
        self.flat_entries.get(self.selected)
    }

    pub fn toggle_expand(&mut self) {
        if let Some(entry) = self.flat_entries.get(self.selected) {
            if entry.is_dir {
                let path = entry.path.clone();
                self.toggle_dir_expanded(&path);
                self.rebuild_flat_list();
            }
        }
    }

    fn toggle_dir_expanded(&mut self, path: &str) {
        fn toggle_in_entries(entries: &mut [FileTreeEntry], path: &str) -> bool {
            for entry in entries {
                if entry.path == path {
                    entry.expanded = !entry.expanded;
                    return true;
                }
                if entry.is_dir && toggle_in_entries(&mut entry.children, path) {
                    return true;
                }
            }
            false
        }
        toggle_in_entries(&mut self.entries, path);
    }

    pub fn move_up(&mut self) {
        if self.flat_entries.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.flat_entries.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.flat_entries.is_empty() {
            return;
        }
        if self.selected + 1 < self.flat_entries.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let title = " Files ";

        let block = Block::new()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let height = inner.height as usize;

        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1);

        if self.flat_entries.is_empty() {
            let msg = "Empty";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, entry) in self.flat_entries.iter().skip(self.offset).take(height).enumerate() {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir {
                    if entry.expanded { " " } else { " " }
                } else {
                    " "
                };

                let status_color = match entry.status {
                    Some(FileTreeStatus::Modified) => theme.unstaged,
                    Some(FileTreeStatus::Added) => theme.staged,
                    Some(FileTreeStatus::Deleted) => theme.diff_remove,
                    Some(FileTreeStatus::Untracked) => theme.untracked,
                    Some(FileTreeStatus::Ignored) => theme.untracked,
                    None => theme.foreground,
                };

                let style = if is_selected {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else {
                    Style::new().fg(status_color)
                };

                let line = format!("{}{}{}", indent, icon, entry.name);
                buf.set_string_truncated(inner.x, y, &line, content_width, style);
            }
        }

        let scrollbar = Scrollbar::new(self.flat_entries.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
