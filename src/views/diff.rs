use crate::git::{DiffInfo, FileDiff, LineType};
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct DiffView {
    pub diff: DiffInfo,
    pub current_file: usize,
    pub scroll: usize,
    pub show_line_numbers: bool,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            diff: DiffInfo { files: Vec::new() },
            current_file: 0,
            scroll: 0,
            show_line_numbers: true,
        }
    }

    pub fn update(&mut self, diff: DiffInfo) {
        self.diff = diff;
        self.current_file = 0;
        self.scroll = 0;
    }

    pub fn clear(&mut self) {
        self.diff = DiffInfo { files: Vec::new() };
        self.current_file = 0;
        self.scroll = 0;
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        self.diff.files.get(self.current_file)
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    pub fn next_file(&mut self) {
        if self.current_file + 1 < self.diff.files.len() {
            self.current_file += 1;
            self.scroll = 0;
        }
    }

    pub fn prev_file(&mut self) {
        if self.current_file > 0 {
            self.current_file -= 1;
            self.scroll = 0;
        }
    }

    pub fn next_hunk(&mut self) {
        // TODO: Implement hunk navigation
    }

    pub fn prev_hunk(&mut self) {
        // TODO: Implement hunk navigation
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border };

        let title = if let Some(file) = self.current_file() {
            format!(" Diff: {} (+{} -{}) ",
                file.path,
                file.additions(),
                file.deletions()
            )
        } else {
            " Diff ".to_string()
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

        // Collect all lines from all hunks (owned data to avoid borrow issues)
        let lines: Vec<(Option<u32>, Option<u32>, LineType, String)> = {
            let Some(file) = self.current_file() else {
                // No diff to show
                let msg = "No changes to display";
                let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
                return;
            };

            let mut lines = Vec::new();
            for hunk in &file.hunks {
                // Hunk header
                lines.push((None, None, LineType::Context, hunk.header.clone()));

                for line in &hunk.lines {
                    lines.push((
                        line.old_lineno,
                        line.new_lineno,
                        line.line_type,
                        line.content.clone(),
                    ));
                }
            }
            lines
        };

        // Adjust scroll (now safe because we don't hold a borrow on self)
        let max_scroll = lines.len().saturating_sub(inner.height as usize);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }

        let line_num_width = if self.show_line_numbers { 8 } else { 0 };
        let visible_height = inner.height as usize;
        let content_area_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        for (i, (old_line, new_line, line_type, content)) in lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;

            // Line numbers
            if self.show_line_numbers {
                let old_str = old_line.map(|n| format!("{:>3}", n)).unwrap_or_else(|| "   ".to_string());
                let new_str = new_line.map(|n| format!("{:>3}", n)).unwrap_or_else(|| "   ".to_string());
                let line_nums = format!("{} {} |", old_str, new_str);
                buf.set_string(inner.x, y, &line_nums, Style::new().fg(theme.untracked).dim());
            }

            // Line content
            let content_x = inner.x + line_num_width;
            let content_width = content_area_width.saturating_sub(line_num_width);

            let (prefix, style) = match line_type {
                LineType::Addition => ("+", Style::new().fg(theme.diff_add)),
                LineType::Deletion => ("-", Style::new().fg(theme.diff_remove)),
                LineType::Context => {
                    if content.starts_with("@@") {
                        (" ", Style::new().fg(theme.diff_hunk).bold())
                    } else {
                        (" ", Style::new().fg(theme.foreground))
                    }
                }
            };

            buf.set_string(content_x, y, prefix, style);

            // Remove trailing newline from content
            let content = content.trim_end_matches('\n');
            buf.set_string_truncated(content_x + 1, y, content, content_width.saturating_sub(1), style);
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(lines.len(), visible_height, self.scroll);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
