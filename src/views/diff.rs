use crate::git::{DiffInfo, FileDiff, LineType};
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    Inline,
    SideBySide,
}

pub struct DiffView {
    pub diff: DiffInfo,
    pub current_file: usize,
    pub scroll: usize,
    pub h_offset: usize,
    pub show_line_numbers: bool,
    pub mode: DiffMode,
    pub max_content_width: usize,
    pub view_width: usize,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            diff: DiffInfo { files: Vec::new() },
            current_file: 0,
            scroll: 0,
            h_offset: 0,
            show_line_numbers: true,
            mode: DiffMode::Inline,
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

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        // Set to a large number, rendering will clamp it
        self.scroll = usize::MAX / 2;
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

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DiffMode::Inline => DiffMode::SideBySide,
            DiffMode::SideBySide => DiffMode::Inline,
        };
        self.scroll = 0;
    }

    pub fn next_hunk(&mut self) {
        // TODO: Implement hunk navigation
    }

    pub fn prev_hunk(&mut self) {
        // TODO: Implement hunk navigation
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let mode_indicator = match self.mode {
            DiffMode::Inline => "inline",
            DiffMode::SideBySide => "split",
        };

        let title = if let Some(file) = self.current_file() {
            format!(" Diff: {} (+{} -{}) [{}] ",
                file.path,
                file.additions(),
                file.deletions(),
                mode_indicator
            )
        } else {
            format!(" Diff [{}] ", mode_indicator)
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

        match self.mode {
            DiffMode::Inline => self.render_inline(inner, buf, theme),
            DiffMode::SideBySide => self.render_side_by_side(inner, buf, theme),
        }
    }

    fn render_inline(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
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

        // Calculate max content width and store view width
        self.view_width = (content_area_width.saturating_sub(line_num_width)) as usize;
        self.max_content_width = lines.iter().map(|(_, _, _, content)| {
            content.trim_end_matches('\n').chars().count() + 1 // +1 for prefix
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

    fn render_side_by_side(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
        // Build paired lines for side-by-side view
        let paired_lines: Vec<(Option<(u32, String)>, Option<(u32, String)>)> = {
            let Some(file) = self.current_file() else {
                let msg = "No changes to display";
                let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
                return;
            };

            let mut pairs = Vec::new();

            for hunk in &file.hunks {
                // Hunk header spans both sides
                pairs.push((
                    Some((0, hunk.header.clone())),
                    Some((0, hunk.header.clone()))
                ));

                // Collect deletions and additions separately, then pair them
                let mut deletions: Vec<(u32, String)> = Vec::new();
                let mut additions: Vec<(u32, String)> = Vec::new();

                for line in &hunk.lines {
                    match line.line_type {
                        LineType::Context => {
                            // Flush any pending deletions/additions
                            Self::flush_pairs(&mut pairs, &mut deletions, &mut additions);

                            let old_no = line.old_lineno.unwrap_or(0);
                            let new_no = line.new_lineno.unwrap_or(0);
                            let content = line.content.trim_end_matches('\n').to_string();
                            pairs.push((
                                Some((old_no, content.clone())),
                                Some((new_no, content))
                            ));
                        }
                        LineType::Deletion => {
                            let line_no = line.old_lineno.unwrap_or(0);
                            let content = line.content.trim_end_matches('\n').to_string();
                            deletions.push((line_no, content));
                        }
                        LineType::Addition => {
                            let line_no = line.new_lineno.unwrap_or(0);
                            let content = line.content.trim_end_matches('\n').to_string();
                            additions.push((line_no, content));
                        }
                    }
                }

                // Flush remaining
                Self::flush_pairs(&mut pairs, &mut deletions, &mut additions);
            }

            pairs
        };

        // Adjust scroll
        let max_scroll = paired_lines.len().saturating_sub(inner.height as usize);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }

        let visible_height = inner.height as usize;
        let total_width = inner.width.saturating_sub(1); // Leave space for scrollbar
        let half_width = total_width / 2;
        let line_num_width: u16 = 4;

        // Calculate max content width for side-by-side mode
        self.view_width = (half_width.saturating_sub(line_num_width + 1)) as usize;
        self.max_content_width = paired_lines.iter().map(|(left, right)| {
            let left_len = left.as_ref().map(|(_, c)| c.chars().count()).unwrap_or(0);
            let right_len = right.as_ref().map(|(_, c)| c.chars().count()).unwrap_or(0);
            left_len.max(right_len)
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

        // Draw separator line
        let sep_x = inner.x + half_width;
        for y in inner.y..inner.y + inner.height {
            buf.set_string(sep_x, y, "|", Style::new().fg(theme.border));
        }

        for (i, (left, right)) in paired_lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let left_content_width = half_width.saturating_sub(line_num_width + 1);
            let right_content_width = half_width.saturating_sub(line_num_width + 2);

            // Left side (old/deletion)
            if let Some((line_no, content)) = left {
                let is_hunk_header = content.starts_with("@@");
                let is_deletion = right.is_none() || (right.is_some() && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c));

                let style = if is_hunk_header {
                    Style::new().fg(theme.diff_hunk).bold()
                } else if is_deletion && !right.is_some() {
                    Style::new().fg(theme.diff_remove)
                } else if is_deletion && right.is_some() && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c) {
                    Style::new().fg(theme.diff_remove)
                } else {
                    Style::new().fg(theme.foreground)
                };

                // Line number
                if *line_no > 0 {
                    let num_str = format!("{:>3} ", line_no);
                    buf.set_string(inner.x, y, &num_str, Style::new().fg(theme.untracked).dim());
                } else {
                    buf.set_string(inner.x, y, "    ", Style::new().fg(theme.untracked).dim());
                }

                // Content
                buf.set_string_truncated(inner.x + line_num_width, y, content, left_content_width, style);
            }

            // Right side (new/addition)
            let right_x = sep_x + 1;
            if let Some((line_no, content)) = right {
                let is_hunk_header = content.starts_with("@@");
                let is_addition = left.is_none() || (left.is_some() && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c));

                let style = if is_hunk_header {
                    Style::new().fg(theme.diff_hunk).bold()
                } else if is_addition && !left.is_some() {
                    Style::new().fg(theme.diff_add)
                } else if is_addition && left.is_some() && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c) {
                    Style::new().fg(theme.diff_add)
                } else {
                    Style::new().fg(theme.foreground)
                };

                // Line number
                if *line_no > 0 {
                    let num_str = format!("{:>3} ", line_no);
                    buf.set_string(right_x, y, &num_str, Style::new().fg(theme.untracked).dim());
                } else {
                    buf.set_string(right_x, y, "    ", Style::new().fg(theme.untracked).dim());
                }

                // Content
                buf.set_string_truncated(right_x + line_num_width, y, content, right_content_width, style);
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(paired_lines.len(), visible_height, self.scroll);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn flush_pairs(
        pairs: &mut Vec<(Option<(u32, String)>, Option<(u32, String)>)>,
        deletions: &mut Vec<(u32, String)>,
        additions: &mut Vec<(u32, String)>,
    ) {
        let max_len = deletions.len().max(additions.len());
        for i in 0..max_len {
            let left = deletions.get(i).cloned();
            let right = additions.get(i).cloned();
            pairs.push((left, right));
        }
        deletions.clear();
        additions.clear();
    }
}
