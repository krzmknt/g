use crate::git::ConflictEntry;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct ConflictView {
    pub conflicts: Vec<ConflictEntry>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
}

impl ConflictView {
    pub fn new() -> Self {
        Self {
            conflicts: Vec::new(),
            selected: 0,
            offset: 0,
            h_offset: 0,
            max_content_width: 0,
            view_width: 0,
        }
    }

    pub fn can_scroll_left(&self) -> bool {
        self.h_offset > 0
    }

    pub fn can_scroll_right(&self) -> bool {
        if self.view_width == 0 {
            return self.max_content_width > 0;
        }
        self.max_content_width > self.view_width &&
            self.h_offset < self.max_content_width.saturating_sub(self.view_width)
    }

    pub fn scroll_left(&mut self) {
        self.h_offset = self.h_offset.saturating_sub(4);
    }

    pub fn scroll_right(&mut self) {
        self.h_offset += 4;
    }

    pub fn update(&mut self, conflicts: Vec<ConflictEntry>) {
        self.conflicts = conflicts;
        if self.selected >= self.conflicts.len() && !self.conflicts.is_empty() {
            self.selected = self.conflicts.len() - 1;
        }
    }

    pub fn selected_conflict(&self) -> Option<&ConflictEntry> {
        self.conflicts.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.conflicts.is_empty() && self.selected + 1 < self.conflicts.len() {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.conflicts.is_empty() {
            self.selected = self.conflicts.len() - 1;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let title = format!(" Conflicts ({}) ", self.conflicts.len());

        let block = Block::new()
            .title(&title)
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

        // Calculate max content width and store view width
        self.view_width = content_width as usize;
        self.max_content_width = self.conflicts.iter().map(|conflict| {
            format!(" {} ({})", conflict.path, conflict.conflict_type).chars().count()
        }).max().unwrap_or(0) + 2; // +2 for scrollbar (1) + margin (1)

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        if self.conflicts.is_empty() {
            let msg = "No conflicts";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.staged));
        } else {
            for (i, conflict) in self.conflicts.iter().skip(self.offset).take(height).enumerate() {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                let style = if is_selected && focused {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else {
                    Style::new().fg(theme.diff_remove)
                };

                // Fill full line width when selected and focused
                if is_selected && focused {
                    let blank_line = " ".repeat(content_width as usize);
                    buf.set_string(inner.x, y, &blank_line, style);
                }

                let line = format!(" {} ({})", conflict.path, conflict.conflict_type);
                // Apply horizontal scroll
                let display_line: String = line.chars().skip(self.h_offset).collect();
                buf.set_string_truncated(inner.x, y, &display_line, content_width, style);
            }
        }

        let scrollbar = Scrollbar::new(self.conflicts.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
