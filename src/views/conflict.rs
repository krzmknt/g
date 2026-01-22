use crate::git::ConflictEntry;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct ConflictView {
    pub conflicts: Vec<ConflictEntry>,
    pub selected: usize,
    pub offset: usize,
}

impl ConflictView {
    pub fn new() -> Self {
        Self {
            conflicts: Vec::new(),
            selected: 0,
            offset: 0,
        }
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
        if self.conflicts.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.conflicts.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.conflicts.is_empty() {
            return;
        }
        if self.selected + 1 < self.conflicts.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
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

        if self.conflicts.is_empty() {
            let msg = "No conflicts";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.staged));
        } else {
            for (i, conflict) in self.conflicts.iter().skip(self.offset).take(height).enumerate() {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                let style = if is_selected {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else {
                    Style::new().fg(theme.diff_remove)
                };

                let line = format!(" {} ({})", conflict.path, conflict.conflict_type);
                buf.set_string_truncated(inner.x, y, &line, content_width, style);
            }
        }

        let scrollbar = Scrollbar::new(self.conflicts.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
