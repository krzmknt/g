use crate::git::GraphCommit;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct LogGraphView {
    pub commits: Vec<GraphCommit>,
    pub selected: usize,
    pub offset: usize,
}

impl LogGraphView {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            selected: 0,
            offset: 0,
        }
    }

    pub fn update(&mut self, commits: Vec<GraphCommit>) {
        self.commits = commits;
        if self.selected >= self.commits.len() && !self.commits.is_empty() {
            self.selected = self.commits.len() - 1;
        }
    }

    pub fn selected_commit(&self) -> Option<&GraphCommit> {
        self.commits.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.commits.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.commits.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.commits.is_empty() {
            return;
        }
        if self.selected + 1 < self.commits.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let title = format!(" Log Graph ({}) ", self.commits.len());

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

        if self.commits.is_empty() {
            let msg = "No commits";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, commit) in self.commits.iter().skip(self.offset).take(height).enumerate() {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                // Draw graph chars
                let graph_width = commit.graph_chars.len() as u16;
                let graph_style = Style::new().fg(theme.branch_local);
                buf.set_string(inner.x, y, &commit.graph_chars, graph_style);

                // Draw refs if any
                let mut x_offset = inner.x + graph_width;
                if !commit.refs.is_empty() {
                    let refs_str = format!("({})", commit.refs.join(", "));
                    let refs_width = refs_str.len().min((content_width - graph_width) as usize / 3);
                    let refs_display = if refs_str.len() > refs_width {
                        format!("{}..)", &refs_str[..refs_width.saturating_sub(3)])
                    } else {
                        refs_str
                    };
                    buf.set_string(x_offset, y, &refs_display, Style::new().fg(theme.branch_current).bold());
                    x_offset += refs_display.len() as u16 + 1;
                }

                // Draw commit info
                let base_style = if is_selected {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else {
                    Style::new().fg(theme.foreground)
                };

                let remaining = content_width.saturating_sub(x_offset - inner.x);
                let info = format!("{} {} - {}", commit.short_id, commit.author, commit.message);
                buf.set_string_truncated(x_offset, y, &info, remaining, base_style);
            }
        }

        let scrollbar = Scrollbar::new(self.commits.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
