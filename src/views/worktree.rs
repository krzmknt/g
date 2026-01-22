use crate::git::WorktreeInfo;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct WorktreeView {
    pub worktrees: Vec<WorktreeInfo>,
    pub selected: usize,
    pub offset: usize,
}

impl WorktreeView {
    pub fn new() -> Self {
        Self {
            worktrees: Vec::new(),
            selected: 0,
            offset: 0,
        }
    }

    pub fn update(&mut self, worktrees: Vec<WorktreeInfo>) {
        self.worktrees = worktrees;
        if self.selected >= self.worktrees.len() && !self.worktrees.is_empty() {
            self.selected = self.worktrees.len() - 1;
        }
    }

    pub fn selected_worktree(&self) -> Option<&WorktreeInfo> {
        self.worktrees.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.worktrees.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.worktrees.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.worktrees.is_empty() {
            return;
        }
        if self.selected + 1 < self.worktrees.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let title = format!(" Worktrees ({}) ", self.worktrees.len());

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

        if self.worktrees.is_empty() {
            let msg = "No worktrees";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, wt) in self.worktrees.iter().skip(self.offset).take(height).enumerate() {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                let style = if is_selected {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if wt.is_main {
                    Style::new().fg(theme.branch_current)
                } else {
                    Style::new().fg(theme.foreground)
                };

                let lock_icon = if wt.is_locked { " " } else { "" };
                let main_icon = if wt.is_main { "* " } else { "  " };
                let line = format!("{}{}{} ({})", main_icon, lock_icon, wt.name, wt.path);
                buf.set_string_truncated(inner.x, y, &line, content_width, style);
            }
        }

        let scrollbar = Scrollbar::new(self.worktrees.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
