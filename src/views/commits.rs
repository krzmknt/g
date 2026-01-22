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
                    c.graph_chars.len() + refs_len + c.short_id.len() + 1 + c.author.len() + 3 + c.message.chars().count() + 1 + c.relative_time().len()
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

            let base_style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, base_style);
            }

            // Build left part: hash + message (without time)
            let time_str = commit.relative_time();
            let left_part = format!("{} {}", commit.short_id, commit.message);

            // Calculate positions for right-aligned time
            let time_len = time_str.chars().count();

            // Apply horizontal scroll to left part
            let display_left: String = left_part.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_left, content_width.saturating_sub(time_len as u16 + 1), base_style);

            // Right-align time (gray)
            if self.h_offset == 0 {
                let time_x = inner.x + content_width.saturating_sub(time_len as u16);
                let time_style = if is_selected && focused {
                    base_style
                } else {
                    Style::new().fg(theme.commit_time)
                };
                buf.set_string(time_x, y, &time_str, time_style);
            }

            // Overlay colored parts if not selected
            if !(is_selected && focused) && !is_search_match {
                self.render_compact_colors(buf, inner.x, y, content_width.saturating_sub(time_len as u16 + 1), commit, theme);
            }
        }
    }

    fn render_compact_colors(&self, buf: &mut Buffer, x: u16, y: u16, width: u16, commit: &CommitInfo, theme: &Theme) {
        let hash_len = commit.short_id.chars().count();

        // Calculate visible portion after h_offset
        if self.h_offset < hash_len {
            let visible_start = self.h_offset;
            let hash_part: String = commit.short_id.chars().skip(visible_start).collect();
            let hash_display_len = hash_part.chars().count().min(width as usize);
            if hash_display_len > 0 {
                let truncated: String = hash_part.chars().take(hash_display_len).collect();
                buf.set_string(x, y, &truncated, Style::new().fg(theme.commit_hash));
            }
        }
    }

    fn render_detailed(&self, inner: Rect, buf: &mut Buffer, theme: &Theme, height: usize, content_width: u16, focused: bool) {
        for (i, commit) in self.commits.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            let base_style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, base_style);
            }

            // Build refs with [] and categorize by type
            let refs_str = if !commit.refs.is_empty() {
                format!("[{}] ", commit.refs.join(", "))
            } else {
                String::new()
            };
            let author_truncated: String = commit.author.chars().take(15).collect();
            let time_str = commit.relative_time();
            let time_len = time_str.chars().count();

            // Build left part (without time)
            let left_part = format!("{} {}{} {}", commit.short_id, refs_str, author_truncated, commit.message);

            // Apply horizontal scroll to left part
            let display_left: String = left_part.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_left, content_width.saturating_sub(time_len as u16 + 1), base_style);

            // Right-align time (gray)
            if self.h_offset == 0 {
                let time_x = inner.x + content_width.saturating_sub(time_len as u16);
                let time_style = if is_selected && focused {
                    base_style
                } else {
                    Style::new().fg(theme.commit_time)
                };
                buf.set_string(time_x, y, &time_str, time_style);
            }

            // Overlay colored parts if not selected
            if !(is_selected && focused) && !is_search_match {
                self.render_detailed_colors(buf, inner.x, y, content_width.saturating_sub(time_len as u16 + 1), commit, &refs_str, &author_truncated, theme);
            }
        }
    }

    fn render_detailed_colors(&self, buf: &mut Buffer, x: u16, y: u16, width: u16, commit: &CommitInfo, refs_str: &str, author: &str, theme: &Theme) {
        let hash_len = commit.short_id.chars().count();
        let refs_len = refs_str.chars().count();
        let author_len = author.chars().count();

        // Hash (cyan)
        if self.h_offset < hash_len {
            let start = self.h_offset;
            let part: String = commit.short_id.chars().skip(start).collect();
            let display_len = part.chars().count().min(width as usize);
            if display_len > 0 {
                let truncated: String = part.chars().take(display_len).collect();
                buf.set_string(x, y, &truncated, Style::new().fg(theme.commit_hash));
            }
        }

        // Refs with branch-type coloring
        let pos = hash_len + 1; // after hash + space
        if refs_len > 0 && self.h_offset < pos + refs_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos { 0 } else { pos - self.h_offset };
            if screen_offset < width as usize && start < refs_len {
                // Determine ref color based on content
                let ref_color = self.get_ref_color(&commit.refs, theme);
                let part: String = refs_str.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(x + screen_offset as u16, y, &truncated, Style::new().fg(ref_color));
                }
            }
        }

        // Author (gray)
        let pos2 = pos + refs_len;
        if self.h_offset < pos2 + author_len {
            let start = self.h_offset.saturating_sub(pos2);
            let screen_offset = if self.h_offset > pos2 { 0 } else { pos2 - self.h_offset };
            if screen_offset < width as usize && start < author_len {
                let part: String = author.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(x + screen_offset as u16, y, &truncated, Style::new().fg(theme.commit_author));
                }
            }
        }
    }

    fn get_ref_color(&self, refs: &[String], theme: &Theme) -> crate::tui::Color {
        // Check if any ref is HEAD or current branch (green)
        for r in refs {
            if r == "HEAD" || r.starts_with("HEAD ->") {
                return theme.branch_current;
            }
        }
        // Check if any ref is remote (purple)
        for r in refs {
            if r.contains('/') || r.starts_with("origin") || r.starts_with("upstream") {
                return theme.branch_remote;
            }
        }
        // Default to local branch color (blue)
        theme.branch_local
    }

    fn render_graph(&self, inner: Rect, buf: &mut Buffer, theme: &Theme, height: usize, content_width: u16, focused: bool) {
        // Graph colors for different branches
        let graph_colors = [
            crate::tui::Color::Rgb(243, 139, 168),  // red
            crate::tui::Color::Rgb(166, 227, 161),  // green
            crate::tui::Color::Rgb(249, 226, 175),  // yellow
            crate::tui::Color::Rgb(137, 180, 250),  // blue
            crate::tui::Color::Rgb(203, 166, 247),  // mauve
            crate::tui::Color::Rgb(137, 220, 235),  // cyan
        ];

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

            let base_style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, base_style);
            }

            // Build refs with [] and categorize by type
            let refs_str = if !commit.refs.is_empty() {
                format!(" [{}]", commit.refs.join(", "))
            } else {
                String::new()
            };

            // Get time for right-align
            let time_str = commit.relative_time();
            let time_len = time_str.chars().count();

            // Build left part: graph + hash + refs + author + message (without time)
            let left_part = format!("{}{}{} {} - {}", commit.graph_chars, commit.short_id, refs_str, commit.author, commit.message);

            // Apply horizontal scroll to left part
            let display_left: String = left_part.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_left, content_width.saturating_sub(time_len as u16 + 1), base_style);

            // Right-align time (gray)
            if self.h_offset == 0 {
                let time_x = inner.x + content_width.saturating_sub(time_len as u16);
                let time_style = if is_selected && focused {
                    base_style
                } else {
                    Style::new().fg(theme.commit_time)
                };
                buf.set_string(time_x, y, &time_str, time_style);
            }

            // Overlay colored parts if not selected
            if !(is_selected && focused) && !is_search_match {
                self.render_graph_colors(buf, inner.x, y, content_width.saturating_sub(time_len as u16 + 1), commit, &refs_str, theme, &graph_colors);
            }
        }
    }

    fn render_graph_colors(&self, buf: &mut Buffer, x: u16, y: u16, width: u16, commit: &GraphCommit, refs_str: &str, theme: &Theme, graph_colors: &[crate::tui::Color]) {
        let graph_len = commit.graph_chars.chars().count();
        let hash_len = commit.short_id.chars().count();
        let refs_len = refs_str.chars().count();
        let author_len = commit.author.chars().count();

        let mut pos = 0;

        // Graph (colorful - based on column position)
        if self.h_offset < pos + graph_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos { 0 } else { pos - self.h_offset };
            for (ci, ch) in commit.graph_chars.chars().enumerate() {
                if ci < start {
                    continue;
                }
                let screen_pos = screen_offset + (ci - start);
                if screen_pos >= width as usize {
                    break;
                }
                // Color based on character position (column)
                let color = if ch == '*' || ch == '|' || ch == '/' || ch == '\\' || ch == '_' {
                    graph_colors[ci % graph_colors.len()]
                } else {
                    theme.foreground
                };
                buf.set_string(x + screen_pos as u16, y, &ch.to_string(), Style::new().fg(color));
            }
        }
        pos += graph_len;

        // Hash (cyan) - now comes after graph
        if self.h_offset < pos + hash_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos { 0 } else { pos - self.h_offset };
            if screen_offset < width as usize && start < hash_len {
                let part: String = commit.short_id.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(x + screen_offset as u16, y, &truncated, Style::new().fg(theme.commit_hash));
                }
            }
        }
        pos += hash_len;

        // Refs with branch-type coloring - now comes after hash
        if refs_len > 0 && self.h_offset < pos + refs_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos { 0 } else { pos - self.h_offset };
            if screen_offset < width as usize && start < refs_len {
                // Determine ref color based on content
                let ref_color = self.get_graph_ref_color(&commit.refs, theme);
                let part: String = refs_str.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(x + screen_offset as u16, y, &truncated, Style::new().fg(ref_color));
                }
            }
        }
        pos += refs_len + 1; // +1 for space before author

        // Author (gray)
        if self.h_offset < pos + author_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos { 0 } else { pos - self.h_offset };
            if screen_offset < width as usize && start < author_len {
                let part: String = commit.author.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(x + screen_offset as u16, y, &truncated, Style::new().fg(theme.commit_author));
                }
            }
        }
    }

    fn get_graph_ref_color(&self, refs: &[String], theme: &Theme) -> crate::tui::Color {
        // Check if any ref is HEAD or current branch (green)
        for r in refs {
            if r == "HEAD" || r.starts_with("HEAD ->") {
                return theme.branch_current;
            }
        }
        // Check if any ref is remote (purple)
        for r in refs {
            if r.contains('/') || r.starts_with("origin") || r.starts_with("upstream") {
                return theme.branch_remote;
            }
        }
        // Default to local branch color (blue)
        theme.branch_local
    }
}
