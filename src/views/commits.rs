use crate::git::{CommitInfo, GraphCommit};
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitsViewMode {
    Compact,   // hash + message + time
    Detailed,  // hash + author + refs + message + time
    Graph,     // ASCII graph + refs + hash + author + message
}

pub struct CommitsView {
    pub commits: Vec<CommitInfo>,
    pub graph_commits: Vec<GraphCommit>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,  // horizontal scroll offset
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
    pub view_mode: CommitsViewMode,
    pub max_content_width: usize,
    pub view_width: usize,
}

impl CommitsView {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            graph_commits: Vec::new(),
            selected: 0,
            offset: 0,
            h_offset: 0,
            search_query: None,
            search_results: Vec::new(),
            view_mode: CommitsViewMode::Compact,
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

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            CommitsViewMode::Compact => CommitsViewMode::Detailed,
            CommitsViewMode::Detailed => CommitsViewMode::Graph,
            CommitsViewMode::Graph => CommitsViewMode::Compact,
        };
    }

    pub fn update(&mut self, commits: Vec<CommitInfo>) {
        self.commits = commits;
        self.selected = 0;
        self.offset = 0;
        self.search_query = None;
        self.search_results.clear();
    }

    pub fn update_graph(&mut self, graph_commits: Vec<GraphCommit>) {
        self.graph_commits = graph_commits;
    }

    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.commits.get(self.selected)
    }

    pub fn commit_count(&self) -> usize {
        match self.view_mode {
            CommitsViewMode::Graph => self.graph_commits.len(),
            _ => self.commits.len(),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let len = self.commit_count();
        if len > 0 && self.selected + 1 < len {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        let len = self.commit_count();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        match self.view_mode {
            CommitsViewMode::Graph => {
                for (i, commit) in self.graph_commits.iter().enumerate() {
                    if commit.message.to_lowercase().contains(&query_lower)
                        || commit.author.to_lowercase().contains(&query_lower)
                        || commit.short_id.to_lowercase().contains(&query_lower)
                    {
                        self.search_results.push(i);
                    }
                }
            }
            _ => {
                for (i, commit) in self.commits.iter().enumerate() {
                    if commit.message.to_lowercase().contains(&query_lower)
                        || commit.author.to_lowercase().contains(&query_lower)
                        || commit.short_id.to_lowercase().contains(&query_lower)
                    {
                        self.search_results.push(i);
                    }
                }
            }
        }

        // Jump to first result
        if let Some(&first) = self.search_results.first() {
            self.selected = first;
        }
    }

    pub fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(pos) = self.search_results.iter().position(|&i| i > self.selected) {
            self.selected = self.search_results[pos];
        } else {
            // Wrap around
            self.selected = self.search_results[0];
        }
    }

    pub fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(pos) = self.search_results.iter().rposition(|&i| i < self.selected) {
            self.selected = self.search_results[pos];
        } else {
            // Wrap around
            self.selected = *self.search_results.last().unwrap();
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let mode_indicator = match self.view_mode {
            CommitsViewMode::Compact => "compact",
            CommitsViewMode::Detailed => "detailed",
            CommitsViewMode::Graph => "graph",
        };

        let commit_count = self.commit_count();

        let title = if let Some(ref query) = self.search_query {
            format!(" Commits [/{} - {} matches] [{}] ", query, self.search_results.len(), mode_indicator)
        } else {
            format!(" Commits ({}) [{}] ", commit_count, mode_indicator)
        };

        let block = Block::new()
            .title(&title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 || inner.width < 20 {
            return;
        }

        // Adjust offset
        let height = inner.height as usize;
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        // Store view width and calculate max content width for scroll limiting
        // +2 for scrollbar (1) + margin (1)
        self.view_width = content_width as usize;
        self.max_content_width = match self.view_mode {
            CommitsViewMode::Compact => {
                self.commits.iter().map(|c| {
                    // hash + space + message + space + time
                    c.short_id.len() + 1 + c.message.chars().count() + 1 + c.relative_time().len()
                }).max().unwrap_or(0)
            }
            CommitsViewMode::Detailed => {
                self.commits.iter().map(|c| {
                    let refs_len = if c.refs.is_empty() { 0 } else { c.refs.join(", ").len() + 3 };
                    c.short_id.len() + 1 + refs_len + 15 + 1 + c.message.chars().count() + 1 + c.relative_time().len()
                }).max().unwrap_or(0)
            }
            CommitsViewMode::Graph => {
                self.graph_commits.iter().map(|c| {
                    let refs_len = if c.refs.is_empty() { 0 } else { c.refs.join(", ").len() + 3 };
                    c.graph_chars.len() + refs_len + c.short_id.len() + 1 + c.author.len() + 3 + c.message.chars().count()
                }).max().unwrap_or(0)
            }
        } + 2;  // +2 for scrollbar (1) + margin (1)

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        match self.view_mode {
            CommitsViewMode::Compact => self.render_compact(inner, buf, theme, height, content_width, focused),
            CommitsViewMode::Detailed => self.render_detailed(inner, buf, theme, height, content_width, focused),
            CommitsViewMode::Graph => self.render_graph(inner, buf, theme, height, content_width, focused),
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(commit_count, height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn render_compact(&self, inner: Rect, buf: &mut Buffer, theme: &Theme, height: usize, content_width: u16, focused: bool) {
        for (i, commit) in self.commits.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, Style::new().bg(theme.selection));
            }

            // Build segments: (text, color)
            let time_str = commit.relative_time();
            let segments: Vec<(&str, crate::tui::Color)> = vec![
                (&commit.short_id, theme.commit_hash),
                (" ", theme.foreground),
                (&commit.message, theme.commit_message),
                (" ", theme.foreground),
                (&time_str, theme.commit_time),
            ];

            // Render with colors (respecting h_offset and selection)
            self.render_colored_line(buf, inner.x, y, content_width, &segments, is_selected && focused, is_search_match, theme);
        }
    }

    fn render_detailed(&self, inner: Rect, buf: &mut Buffer, theme: &Theme, height: usize, content_width: u16, focused: bool) {
        for (i, commit) in self.commits.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, Style::new().bg(theme.selection));
            }

            // Build segments: hash + refs + author + message + time
            let refs_str = if !commit.refs.is_empty() {
                format!("({}) ", commit.refs.join(", "))
            } else {
                String::new()
            };
            let author_truncated: String = commit.author.chars().take(15).collect();
            let time_str = commit.relative_time();

            // Use owned strings for segments that need formatting
            let segments_owned: Vec<(String, crate::tui::Color)> = vec![
                (commit.short_id.clone(), theme.commit_hash),
                (" ".to_string(), theme.foreground),
                (refs_str, theme.commit_refs),
                (author_truncated, theme.commit_author),
                (" ".to_string(), theme.foreground),
                (commit.message.clone(), theme.commit_message),
                (" ".to_string(), theme.foreground),
                (time_str, theme.commit_time),
            ];

            // Convert to references for rendering
            let segments: Vec<(&str, crate::tui::Color)> = segments_owned.iter()
                .map(|(s, c)| (s.as_str(), *c))
                .collect();

            self.render_colored_line(buf, inner.x, y, content_width, &segments, is_selected && focused, is_search_match, theme);
        }
    }

    fn render_graph(&self, inner: Rect, buf: &mut Buffer, theme: &Theme, height: usize, content_width: u16, focused: bool) {
        if self.graph_commits.is_empty() {
            let msg = "No commits";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
            return;
        }

        for (i, commit) in self.graph_commits.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, Style::new().bg(theme.selection));
            }

            // Build segments: graph + refs + hash + author + message
            let refs_str = if !commit.refs.is_empty() {
                format!("({}) ", commit.refs.join(", "))
            } else {
                String::new()
            };

            let segments_owned: Vec<(String, crate::tui::Color)> = vec![
                (commit.graph_chars.clone(), theme.diff_hunk),
                (refs_str, theme.commit_refs),
                (commit.short_id.clone(), theme.commit_hash),
                (" ".to_string(), theme.foreground),
                (commit.author.clone(), theme.commit_author),
                (" - ".to_string(), theme.foreground),
                (commit.message.clone(), theme.commit_message),
            ];

            let segments: Vec<(&str, crate::tui::Color)> = segments_owned.iter()
                .map(|(s, c)| (s.as_str(), *c))
                .collect();

            self.render_colored_line(buf, inner.x, y, content_width, &segments, is_selected && focused, is_search_match, theme);
        }
    }

    /// Render a line with multiple colored segments, respecting horizontal scroll
    fn render_colored_line(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        segments: &[(&str, crate::tui::Color)],
        is_selected: bool,
        is_search_match: bool,
        theme: &Theme,
    ) {
        let mut current_x = x;
        let mut chars_skipped = 0;
        let max_x = x + width;

        for (text, color) in segments {
            for ch in text.chars() {
                // Skip characters for horizontal scroll
                if chars_skipped < self.h_offset {
                    chars_skipped += 1;
                    continue;
                }

                // Stop if we've exceeded the width
                if current_x >= max_x {
                    return;
                }

                // Determine style based on selection state
                let style = if is_selected {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if is_search_match {
                    Style::new().fg(theme.diff_hunk)
                } else {
                    Style::new().fg(*color)
                };

                buf.set_string(current_x, y, &ch.to_string(), style);
                current_x += 1;
            }
        }
    }
}
