use crate::git::BlameInfo;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct BlameView {
    pub blame: Option<BlameInfo>,
    pub selected: usize,
    pub offset: usize,
}

impl BlameView {
    pub fn new() -> Self {
        Self {
            blame: None,
            selected: 0,
            offset: 0,
        }
    }

    pub fn update(&mut self, blame: BlameInfo) {
        let line_count = blame.lines.len();
        self.blame = Some(blame);
        if self.selected >= line_count && line_count > 0 {
            self.selected = line_count - 1;
        }
    }

    pub fn clear(&mut self) {
        self.blame = None;
        self.selected = 0;
        self.offset = 0;
    }

    pub fn move_up(&mut self) {
        if let Some(ref blame) = self.blame {
            if blame.lines.is_empty() {
                return;
            }
            if self.selected > 0 {
                self.selected -= 1;
            } else {
                self.selected = blame.lines.len() - 1;
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(ref blame) = self.blame {
            if blame.lines.is_empty() {
                return;
            }
            if self.selected + 1 < blame.lines.len() {
                self.selected += 1;
            } else {
                self.selected = 0;
            }
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let title = match &self.blame {
            Some(blame) => format!(" Blame: {} ", blame.path),
            None => " Blame ".to_string(),
        };

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

        let Some(ref blame) = self.blame else {
            let msg = "Select a file to view blame";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
            return;
        };

        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1);

        for (i, line) in blame.lines.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;

            let base_style = if is_selected {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Format: commit_id author date | line_num | content
            let meta = format!("{} {:8} {} | {:4} | ",
                line.commit_id,
                &line.author[..line.author.len().min(8)],
                line.date,
                line.line_number
            );

            let meta_width = meta.len() as u16;
            buf.set_string(inner.x, y, &meta, Style::new().fg(theme.untracked).dim());

            let content_start = inner.x + meta_width;
            let remaining_width = content_width.saturating_sub(meta_width);
            buf.set_string_truncated(content_start, y, &line.content, remaining_width, base_style);
        }

        let scrollbar = Scrollbar::new(blame.lines.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
