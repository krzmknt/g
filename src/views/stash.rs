use crate::git::StashEntry;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct StashView {
    pub stashes: Vec<StashEntry>,
    pub selected: usize,
    pub offset: usize,
}

impl StashView {
    pub fn new() -> Self {
        Self {
            stashes: Vec::new(),
            selected: 0,
            offset: 0,
        }
    }

    pub fn update(&mut self, stashes: Vec<StashEntry>) {
        self.stashes = stashes;
        if self.selected >= self.stashes.len() && !self.stashes.is_empty() {
            self.selected = self.stashes.len() - 1;
        }
    }

    pub fn selected_stash(&self) -> Option<&StashEntry> {
        self.stashes.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.stashes.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            // Wrap to last item
            self.selected = self.stashes.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.stashes.is_empty() {
            return;
        }
        if self.selected + 1 < self.stashes.len() {
            self.selected += 1;
        } else {
            // Wrap to first item
            self.selected = 0;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let title = format!(" Stash ({}) ", self.stashes.len());

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

        // Adjust offset
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        if self.stashes.is_empty() {
            let msg = "No stashes";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, stash) in self.stashes.iter().skip(self.offset).take(height).enumerate() {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                let style = if is_selected {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else {
                    Style::new().fg(theme.foreground)
                };

                // Format: stash@{0}: message
                let line = format!("stash@{{{}}} {}", stash.index, stash.message);
                buf.set_string_truncated(inner.x, y, &line, content_width, style);
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(self.stashes.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
