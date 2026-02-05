use crate::config::Theme;
use crate::git::{CommitInfo, GraphCommit, GraphLine};
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitsViewMode {
    Compact,  // hash + message + time
    Detailed, // hash + author + refs + message + time
    Graph,    // ASCII graph + refs + hash + author + message
}

use std::collections::HashSet;

pub struct CommitsView {
    pub commits: Vec<CommitInfo>,
    pub graph_lines: Vec<GraphLine>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize, // horizontal scroll offset
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
    pub view_mode: CommitsViewMode,
    pub max_content_width: usize,
    pub view_width: usize,
    /// Branch name to highlight (e.g., from focused PR)
    pub highlight_branch: Option<String>,
    /// Commit IDs to highlight (from focused PR)
    pub highlight_commits: HashSet<String>,
    /// Current HEAD branch name (for coloring)
    pub current_branch: Option<String>,
    /// User-marked commits (for multi-select)
    pub marked_commits: HashSet<String>,
}

impl CommitsView {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            graph_lines: Vec::new(),
            selected: 0,
            offset: 0,
            h_offset: 0,
            search_query: None,
            search_results: Vec::new(),
            view_mode: CommitsViewMode::Compact,
            max_content_width: 0,
            view_width: 0,
            highlight_branch: None,
            highlight_commits: HashSet::new(),
            current_branch: None,
            marked_commits: HashSet::new(),
        }
    }

    pub fn set_highlight_branch(&mut self, branch: Option<String>) {
        self.highlight_branch = branch;
    }

    /// Set the current HEAD branch name (for coloring)
    pub fn set_current_branch(&mut self, branch: Option<String>) {
        self.current_branch = branch;
    }

    /// Set commit IDs to highlight (from PR commits)
    pub fn set_highlight_commits(&mut self, commit_ids: Vec<String>) {
        crate::debug!(
            "set_highlight_commits: {} commits, first={:?}",
            commit_ids.len(),
            commit_ids.first()
        );
        self.highlight_commits = commit_ids.into_iter().collect();
    }

    /// Get color for a single ref
    fn get_single_ref_color(&self, ref_name: &str, theme: &Theme) -> crate::tui::Color {
        // Current branch (green)
        // Handle "local -> remote" format by extracting the local part
        if let Some(ref current) = self.current_branch {
            // Check direct match
            if ref_name == current {
                return theme.branch_current;
            }
            // Check "HEAD -> branch" format
            if ref_name == format!("HEAD -> {}", current) {
                return theme.branch_current;
            }
            // Check "branch -> origin/branch" format (local -> remote tracking)
            if ref_name.starts_with(current) && ref_name.contains(" -> ") {
                return theme.branch_current;
            }
        }
        // HEAD pointer
        if ref_name == "HEAD" || ref_name.starts_with("HEAD ->") {
            return theme.branch_current;
        }
        // Tag (yellow - use commit_refs color)
        if ref_name.starts_with("tag:") {
            return theme.commit_refs;
        }
        // Remote branch (purple/magenta) - starts with known remote prefixes
        // Remote branches are typically "origin/branch" or "upstream/branch"
        // Check for common remote prefixes followed by /
        if ref_name.starts_with("origin/")
            || ref_name.starts_with("upstream/")
            || ref_name.starts_with("remote/")
            || (ref_name.contains('/')
                && !ref_name.contains("feature/")
                && !ref_name.contains("bugfix/")
                && !ref_name.contains("hotfix/")
                && !ref_name.contains("release/"))
        {
            // Additional heuristic: if the first segment before / is short (like origin, upstream)
            // it's likely a remote. Feature branches like feature/xyz have longer prefixes.
            if let Some(slash_pos) = ref_name.find('/') {
                let prefix = &ref_name[..slash_pos];
                // Common remote names are short (origin, upstream, fork, etc.)
                // Feature branch prefixes are typically longer (feature, bugfix, hotfix, release)
                if prefix.len() <= 8
                    && ![
                        "feature", "bugfix", "hotfix", "release", "fix", "chore", "docs", "test",
                        "refactor",
                    ]
                    .contains(&prefix)
                {
                    return theme.branch_remote;
                }
            }
        }
        // Local branch (blue)
        theme.branch_local
    }

    /// Clear commit highlighting
    pub fn clear_highlight_commits(&mut self) {
        self.highlight_commits.clear();
    }

    /// Toggle mark on the currently selected commit
    pub fn toggle_mark(&mut self) {
        let commit_id = match self.view_mode {
            CommitsViewMode::Graph => self
                .graph_lines
                .get(self.selected)
                .and_then(|line| line.as_commit())
                .map(|c| c.id.clone()),
            _ => self.commits.get(self.selected).map(|c| c.id.clone()),
        };
        if let Some(id) = commit_id {
            if self.marked_commits.contains(&id) {
                self.marked_commits.remove(&id);
            } else {
                self.marked_commits.insert(id);
            }
        }
    }

    /// Clear all marks
    pub fn clear_marks(&mut self) {
        self.marked_commits.clear();
    }

    /// Get marked commit IDs
    pub fn get_marked_commits(&self) -> &HashSet<String> {
        &self.marked_commits
    }

    /// Check if a commit is marked
    fn is_marked(&self, commit_id: &str) -> bool {
        self.marked_commits.contains(commit_id)
    }

    /// Check if a commit should be highlighted (by commit ID from GitHub API only)
    fn is_highlighted(&self, commit_id: &str, _refs: &[String]) -> bool {
        // Only highlight commits that are in the PR (from GitHub API)
        // This excludes local unpushed commits

        // Check by full commit ID first
        if self.highlight_commits.contains(commit_id) {
            return true;
        }
        // Check if any highlight commit starts with this commit_id (or vice versa)
        // This handles both full ID and short ID comparisons
        for highlight_id in &self.highlight_commits {
            if commit_id.starts_with(highlight_id) || highlight_id.starts_with(commit_id) {
                return true;
            }
        }
        false
    }

    pub fn set_view_mode(&mut self, mode: CommitsViewMode) {
        self.view_mode = mode;
    }

    pub fn can_scroll_left(&self) -> bool {
        self.h_offset > 0
    }

    pub fn can_scroll_right(&self) -> bool {
        if self.view_width == 0 {
            return self.max_content_width > 0;
        }
        self.max_content_width > self.view_width
            && self.h_offset < self.max_content_width.saturating_sub(self.view_width)
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

    /// Update commits without resetting scroll position (for background refresh)
    pub fn update_preserve_scroll(&mut self, commits: Vec<CommitInfo>) {
        let len = commits.len();
        self.commits = commits;
        // Ensure selected is within bounds
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
        // Ensure offset is valid
        if self.offset > self.selected {
            self.offset = self.selected;
        }
        self.search_query = None;
        self.search_results.clear();
    }

    pub fn update_graph(&mut self, graph_lines: Vec<GraphLine>) {
        self.graph_lines = graph_lines;
    }

    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.commits.get(self.selected)
    }

    pub fn commit_count(&self) -> usize {
        match self.view_mode {
            CommitsViewMode::Graph => self.graph_lines.len(),
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

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        let len = self.commit_count();
        if index < len {
            self.selected = index;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        match self.view_mode {
            CommitsViewMode::Graph => {
                for (i, line) in self.graph_lines.iter().enumerate() {
                    if let Some(commit) = line.as_commit() {
                        if commit.message.to_lowercase().contains(&query_lower)
                            || commit.author.to_lowercase().contains(&query_lower)
                            || commit.short_id.to_lowercase().contains(&query_lower)
                        {
                            self.search_results.push(i);
                        }
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
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let mode_indicator = match self.view_mode {
            CommitsViewMode::Compact => "compact",
            CommitsViewMode::Detailed => "detailed",
            CommitsViewMode::Graph => "graph",
        };

        let commit_count = self.commit_count();

        let title = if let Some(ref query) = self.search_query {
            format!(
                " Commits [/{} - {} matches] [{}] ",
                query,
                self.search_results.len(),
                mode_indicator
            )
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
                self.commits
                    .iter()
                    .map(|c| {
                        // hash + space + message + space + time
                        c.short_id.len()
                            + 1
                            + c.message.chars().count()
                            + 1
                            + c.relative_time().len()
                    })
                    .max()
                    .unwrap_or(0)
            }
            CommitsViewMode::Detailed => self
                .commits
                .iter()
                .map(|c| {
                    let refs_len = if c.refs.is_empty() {
                        0
                    } else {
                        c.refs.join(", ").len() + 3
                    };
                    c.short_id.len()
                        + 1
                        + refs_len
                        + 15
                        + 1
                        + c.message.chars().count()
                        + 1
                        + c.relative_time().len()
                })
                .max()
                .unwrap_or(0),
            CommitsViewMode::Graph => self
                .graph_lines
                .iter()
                .map(|line| match line {
                    GraphLine::Connector(s) => s.len(),
                    GraphLine::Commit(c) => {
                        let refs_len = if c.refs.is_empty() {
                            0
                        } else {
                            c.refs.join(", ").len() + 3
                        };
                        c.graph_chars.len()
                            + refs_len
                            + c.short_id.len()
                            + 1
                            + c.author.len()
                            + 3
                            + c.message.chars().count()
                            + 1
                            + c.relative_time().len()
                    }
                })
                .max()
                .unwrap_or(0),
        } + 2; // +2 for scrollbar (1) + margin (1)

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
            CommitsViewMode::Compact => {
                self.render_compact(inner, buf, theme, height, content_width, focused)
            }
            CommitsViewMode::Detailed => {
                self.render_detailed(inner, buf, theme, height, content_width, focused)
            }
            CommitsViewMode::Graph => {
                self.render_graph(inner, buf, theme, height, content_width, focused)
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(commit_count, height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn render_compact(
        &self,
        inner: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        height: usize,
        content_width: u16,
        focused: bool,
    ) {
        for (i, commit) in self
            .commits
            .iter()
            .skip(self.offset)
            .take(height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));
            let is_pr_highlight = self.is_highlighted(&commit.id, &commit.refs);
            let is_marked = self.is_marked(&commit.id);

            let base_style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_marked {
                Style::new().fg(theme.foreground).bg(theme.diff_remove_bg)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else if is_pr_highlight {
                Style::new().fg(theme.foreground).bg(theme.diff_add_bg)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Fill full line width when selected/focused, marked, or PR highlighted
            if (is_selected && focused) || is_marked || is_pr_highlight {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, base_style);
            }

            // Mark indicator
            let mark_prefix = if is_marked { "● " } else { "" };
            let time_str = commit.relative_time();
            let left_part = format!("{}{} {}", mark_prefix, commit.short_id, commit.message);

            // Calculate positions for right-aligned time
            let time_len = time_str.chars().count();

            // Apply horizontal scroll to left part
            let display_left: String = left_part.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(
                inner.x,
                y,
                &display_left,
                content_width.saturating_sub(time_len as u16 + 1),
                base_style,
            );

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

            // Overlay colored parts if not selected and not highlighted
            if !(is_selected && focused) && !is_search_match && !is_pr_highlight {
                self.render_compact_colors(
                    buf,
                    inner.x,
                    y,
                    content_width.saturating_sub(time_len as u16 + 1),
                    commit,
                    theme,
                );
            }
        }
    }

    fn render_compact_colors(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        commit: &CommitInfo,
        theme: &Theme,
    ) {
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

    fn render_detailed(
        &self,
        inner: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        height: usize,
        content_width: u16,
        focused: bool,
    ) {
        for (i, commit) in self
            .commits
            .iter()
            .skip(self.offset)
            .take(height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));
            let is_pr_highlight = self.is_highlighted(&commit.id, &commit.refs);
            let is_marked = self.is_marked(&commit.id);

            let base_style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_marked {
                Style::new().fg(theme.foreground).bg(theme.diff_remove_bg)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else if is_pr_highlight {
                Style::new().fg(theme.foreground).bg(theme.diff_add_bg)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Fill full line width when selected/focused, marked, or PR highlighted
            if (is_selected && focused) || is_marked || is_pr_highlight {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, base_style);
            }

            // Mark indicator
            let mark_prefix = if is_marked { "● " } else { "" };

            // Build refs with [] and show local -> remote tracking
            let refs_str = if !commit.refs.is_empty() {
                format!("[{}] ", Self::format_refs(&commit.refs))
            } else {
                String::new()
            };
            let author_truncated: String = commit.author.chars().take(15).collect();
            let time_str = commit.relative_time();
            let time_len = time_str.chars().count();

            // Build left part (without time)
            let left_part = format!(
                "{}{} {}{} {}",
                mark_prefix, commit.short_id, refs_str, author_truncated, commit.message
            );

            // Apply horizontal scroll to left part
            let display_left: String = left_part.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(
                inner.x,
                y,
                &display_left,
                content_width.saturating_sub(time_len as u16 + 1),
                base_style,
            );

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
                self.render_detailed_colors(
                    buf,
                    inner.x,
                    y,
                    content_width.saturating_sub(time_len as u16 + 1),
                    commit,
                    &refs_str,
                    &author_truncated,
                    theme,
                );
            }
        }
    }

    fn render_detailed_colors(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        commit: &CommitInfo,
        refs_str: &str,
        author: &str,
        theme: &Theme,
    ) {
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

        // Refs with individual branch-type coloring
        // refs_str format is "[main, origin/main] " (opening bracket, refs, closing bracket, trailing space)
        let pos = hash_len + 1; // after hash + space
        if !commit.refs.is_empty() && self.h_offset < pos + refs_len {
            let screen_start = if self.h_offset > pos {
                0
            } else {
                pos - self.h_offset
            };

            let visible_start = self.h_offset.saturating_sub(pos);
            let mut current_pos = 0usize; // position within refs_str

            // Opening bracket "["
            let bracket_open = "[";
            let bracket_open_len = bracket_open.chars().count();
            if current_pos + bracket_open_len > visible_start
                && current_pos < visible_start + width as usize
            {
                let skip = visible_start.saturating_sub(current_pos);
                let screen_x = if current_pos >= visible_start {
                    screen_start + (current_pos - visible_start)
                } else {
                    screen_start
                };
                if screen_x < width as usize {
                    let available = (width as usize).saturating_sub(screen_x);
                    let part: String = bracket_open.chars().skip(skip).take(available).collect();
                    if !part.is_empty() {
                        let color = if let Some(first_ref) = commit.refs.first() {
                            self.get_single_ref_color(first_ref, theme)
                        } else {
                            theme.foreground
                        };
                        buf.set_string(x + screen_x as u16, y, &part, Style::new().fg(color));
                    }
                }
            }
            current_pos += bracket_open_len;

            // Render each ref with its own color
            // Handle "local -> remote" format by splitting and coloring separately
            for (i, ref_name) in commit.refs.iter().enumerate() {
                let separator = if i == 0 { "" } else { ", " };

                // Check if this ref has "local -> remote" format
                if let Some(arrow_pos) = ref_name.find(" -> ") {
                    let local_part = &ref_name[..arrow_pos];
                    let arrow_and_remote = &ref_name[arrow_pos..]; // " -> origin/xxx"

                    // Render separator
                    if !separator.is_empty() {
                        let sep_end = current_pos + separator.len();
                        if sep_end > visible_start && current_pos < visible_start + width as usize {
                            let skip = visible_start.saturating_sub(current_pos);
                            let screen_x = if current_pos >= visible_start {
                                screen_start + (current_pos - visible_start)
                            } else {
                                screen_start
                            };
                            if screen_x < width as usize {
                                let available = (width as usize).saturating_sub(screen_x);
                                let part: String =
                                    separator.chars().skip(skip).take(available).collect();
                                if !part.is_empty() {
                                    buf.set_string(
                                        x + screen_x as u16,
                                        y,
                                        &part,
                                        Style::new().fg(theme.branch_local),
                                    );
                                }
                            }
                        }
                        current_pos += separator.len();
                    }

                    // Render local part with proper color (green for current branch)
                    let local_color = self.get_single_ref_color(local_part, theme);
                    let local_len = local_part.chars().count();
                    let local_end = current_pos + local_len;
                    if local_end > visible_start && current_pos < visible_start + width as usize {
                        let skip = visible_start.saturating_sub(current_pos);
                        let screen_x = if current_pos >= visible_start {
                            screen_start + (current_pos - visible_start)
                        } else {
                            screen_start
                        };
                        if screen_x < width as usize {
                            let available = (width as usize).saturating_sub(screen_x);
                            let part: String =
                                local_part.chars().skip(skip).take(available).collect();
                            if !part.is_empty() {
                                buf.set_string(
                                    x + screen_x as u16,
                                    y,
                                    &part,
                                    Style::new().fg(local_color),
                                );
                            }
                        }
                    }
                    current_pos = local_end;

                    // Render " -> remote" part (purple)
                    let remote_len = arrow_and_remote.chars().count();
                    let remote_end = current_pos + remote_len;
                    if remote_end > visible_start && current_pos < visible_start + width as usize {
                        let skip = visible_start.saturating_sub(current_pos);
                        let screen_x = if current_pos >= visible_start {
                            screen_start + (current_pos - visible_start)
                        } else {
                            screen_start
                        };
                        if screen_x < width as usize {
                            let available = (width as usize).saturating_sub(screen_x);
                            let part: String = arrow_and_remote
                                .chars()
                                .skip(skip)
                                .take(available)
                                .collect();
                            if !part.is_empty() {
                                buf.set_string(
                                    x + screen_x as u16,
                                    y,
                                    &part,
                                    Style::new().fg(theme.branch_remote),
                                );
                            }
                        }
                    }
                    current_pos = remote_end;
                } else {
                    // Regular ref without " -> "
                    let ref_display = format!("{}{}", separator, ref_name);
                    let ref_len = ref_display.chars().count();
                    let ref_end = current_pos + ref_len;

                    if ref_end > visible_start && current_pos < visible_start + width as usize {
                        let skip = visible_start.saturating_sub(current_pos);
                        let screen_x = if current_pos >= visible_start {
                            screen_start + (current_pos - visible_start)
                        } else {
                            screen_start
                        };

                        if screen_x < width as usize {
                            let available = (width as usize).saturating_sub(screen_x);
                            let part: String =
                                ref_display.chars().skip(skip).take(available).collect();
                            if !part.is_empty() {
                                let color = self.get_single_ref_color(ref_name, theme);
                                buf.set_string(
                                    x + screen_x as u16,
                                    y,
                                    &part,
                                    Style::new().fg(color),
                                );
                            }
                        }
                    }
                    current_pos = ref_end;
                }
            }

            // Closing bracket "] " (with trailing space)
            let close_str = "] ";
            let close_pos = current_pos;
            if close_pos + close_str.len() > visible_start
                && close_pos < visible_start + width as usize
            {
                let skip = visible_start.saturating_sub(close_pos);
                let screen_x = if close_pos >= visible_start {
                    screen_start + (close_pos - visible_start)
                } else {
                    screen_start
                };
                if screen_x < width as usize {
                    let available = (width as usize).saturating_sub(screen_x);
                    let part: String = close_str.chars().skip(skip).take(available).collect();
                    if !part.is_empty() {
                        let color = if let Some(last_ref) = commit.refs.last() {
                            self.get_single_ref_color(last_ref, theme)
                        } else {
                            theme.foreground
                        };
                        buf.set_string(x + screen_x as u16, y, &part, Style::new().fg(color));
                    }
                }
            }
        }

        // Author (gray)
        let pos2 = pos + refs_len;
        if self.h_offset < pos2 + author_len {
            let start = self.h_offset.saturating_sub(pos2);
            let screen_offset = if self.h_offset > pos2 {
                0
            } else {
                pos2 - self.h_offset
            };
            if screen_offset < width as usize && start < author_len {
                let part: String = author.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(
                        x + screen_offset as u16,
                        y,
                        &truncated,
                        Style::new().fg(theme.commit_author),
                    );
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
            // Check if it's the current branch
            if let Some(ref current) = self.current_branch {
                if r == current {
                    return theme.branch_current;
                }
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

    /// Format refs to show local -> remote tracking relationship
    /// e.g., [feature/A, origin/feature/A] becomes [feature/A -> origin/feature/A]
    fn format_refs(refs: &[String]) -> String {
        if refs.is_empty() {
            return String::new();
        }

        // Separate local branches, remote branches, and other refs (tags)
        let mut local_branches: Vec<String> = Vec::new();
        let mut remote_branches: Vec<&str> = Vec::new();
        let mut other_refs: Vec<&str> = Vec::new();

        for r in refs {
            let r = r.as_str();
            if r.starts_with("HEAD -> ") {
                // Extract branch name from "HEAD -> main", treat as local branch
                local_branches.push(r[8..].to_string());
            } else if r == "HEAD" || r == "origin/HEAD" {
                // Skip HEAD and origin/HEAD entirely
                continue;
            } else if r.starts_with("tag: ") {
                other_refs.push(r);
            } else if r.starts_with("origin/") || r.starts_with("upstream/") {
                // Remote tracking branch
                remote_branches.push(r);
            } else if r.contains('/') {
                // Other remote (e.g., other-remote/branch)
                remote_branches.push(r);
            } else {
                // Local branch
                local_branches.push(r.to_string());
            }
        }

        let mut result: Vec<String> = Vec::new();

        // Add other refs first (tags)
        for r in &other_refs {
            result.push(r.to_string());
        }

        // Match local branches with their remote tracking branches
        let mut used_remotes: std::collections::HashSet<&str> = std::collections::HashSet::new();

        for local in &local_branches {
            // Look for matching remote: origin/<local> or upstream/<local>
            let origin_match = format!("origin/{}", local);
            let upstream_match = format!("upstream/{}", local);

            let matching_remote = remote_branches
                .iter()
                .find(|&&r| r == origin_match || r == upstream_match);

            if let Some(&remote) = matching_remote {
                // Found a matching remote, display as "local -> remote"
                result.push(format!("{} -> {}", local, remote));
                used_remotes.insert(remote);
            } else {
                // No matching remote, just display the local branch
                result.push(local.to_string());
            }
        }

        // Add remaining remote branches that don't have a matching local
        for remote in &remote_branches {
            if used_remotes.contains(remote) {
                continue;
            }

            // Extract branch name from remote (e.g., "origin/feature" -> "feature")
            let branch_name = if remote.starts_with("origin/") {
                &remote[7..]
            } else if remote.starts_with("upstream/") {
                &remote[9..]
            } else if let Some(pos) = remote.find('/') {
                &remote[pos + 1..]
            } else {
                *remote
            };

            // Check if there's a matching local branch (already handled above)
            let has_local = local_branches.iter().any(|l| l == branch_name);
            if has_local {
                continue;
            }

            result.push(remote.to_string());
        }

        result.join(", ")
    }

    fn render_graph(
        &self,
        inner: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        height: usize,
        content_width: u16,
        focused: bool,
    ) {
        // Graph colors for different branches
        let graph_colors = [
            crate::tui::Color::Rgb(243, 139, 168), // red
            crate::tui::Color::Rgb(166, 227, 161), // green
            crate::tui::Color::Rgb(249, 226, 175), // yellow
            crate::tui::Color::Rgb(137, 180, 250), // blue
            crate::tui::Color::Rgb(203, 166, 247), // mauve
            crate::tui::Color::Rgb(137, 220, 235), // cyan
        ];

        if self.graph_lines.is_empty() {
            let msg = "No commits";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
            return;
        }

        for (i, line) in self
            .graph_lines
            .iter()
            .skip(self.offset)
            .take(height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            match line {
                GraphLine::Connector(graph_str) => {
                    // Connector-only line (e.g., "|\", "|/")
                    let base_style = if is_selected && focused {
                        Style::new().fg(theme.selection_text).bg(theme.selection)
                    } else {
                        Style::new().fg(theme.foreground)
                    };

                    if is_selected && focused {
                        let blank_line = " ".repeat(content_width as usize);
                        buf.set_string(inner.x, y, &blank_line, base_style);
                    }

                    let display_str: String = graph_str.chars().skip(self.h_offset).collect();
                    buf.set_string_truncated(inner.x, y, &display_str, content_width, base_style);

                    // Color the graph characters
                    if !(is_selected && focused) {
                        self.render_connector_colors(
                            buf,
                            inner.x,
                            y,
                            content_width,
                            graph_str,
                            &graph_colors,
                        );
                    }
                }
                GraphLine::Commit(commit) => {
                    let is_pr_highlight = self.is_highlighted(&commit.id, &commit.refs);
                    let is_marked = self.is_marked(&commit.id);

                    let base_style = if is_selected && focused {
                        Style::new().fg(theme.selection_text).bg(theme.selection)
                    } else if is_marked {
                        Style::new().fg(theme.foreground).bg(theme.diff_remove_bg)
                    } else if is_search_match {
                        Style::new().fg(theme.diff_hunk)
                    } else if is_pr_highlight {
                        Style::new().fg(theme.foreground).bg(theme.diff_add_bg)
                    } else {
                        Style::new().fg(theme.foreground)
                    };

                    // Fill full line width when selected/focused, marked, or PR highlighted
                    if (is_selected && focused) || is_marked || is_pr_highlight {
                        let blank_line = " ".repeat(content_width as usize);
                        buf.set_string(inner.x, y, &blank_line, base_style);
                    }

                    // Mark indicator
                    let mark_prefix = if is_marked { "● " } else { "" };

                    // Build refs with [] and show local -> remote tracking
                    let refs_str = if !commit.refs.is_empty() {
                        format!(" [{}]", Self::format_refs(&commit.refs))
                    } else {
                        String::new()
                    };

                    // Get time for right-align
                    let time_str = commit.relative_time();
                    let time_len = time_str.chars().count();

                    // Build left part: mark + graph + space + hash + refs + author + message (without time)
                    let left_part = format!(
                        "{}{} {}{} {} {}",
                        mark_prefix,
                        commit.graph_chars,
                        commit.short_id,
                        refs_str,
                        commit.author,
                        commit.message
                    );

                    // Apply horizontal scroll to left part
                    let display_left: String = left_part.chars().skip(self.h_offset).collect();
                    buf.set_string_truncated(
                        inner.x,
                        y,
                        &display_left,
                        content_width.saturating_sub(time_len as u16 + 1),
                        base_style,
                    );

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
                        self.render_graph_colors(
                            buf,
                            inner.x,
                            y,
                            content_width.saturating_sub(time_len as u16 + 1),
                            commit,
                            &refs_str,
                            theme,
                            &graph_colors,
                        );
                    }
                }
            }
        }
    }

    fn render_connector_colors(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        graph_str: &str,
        graph_colors: &[crate::tui::Color],
    ) {
        let start = self.h_offset;
        let screen_offset = 0;

        for (ci, ch) in graph_str.chars().enumerate() {
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
                continue; // Skip spaces
            };
            buf.set_string(
                x + screen_pos as u16,
                y,
                &ch.to_string(),
                Style::new().fg(color),
            );
        }
    }

    fn render_graph_colors(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        commit: &GraphCommit,
        refs_str: &str,
        theme: &Theme,
        graph_colors: &[crate::tui::Color],
    ) {
        let graph_len = commit.graph_chars.chars().count();
        let hash_len = commit.short_id.chars().count();
        let refs_len = refs_str.chars().count();
        let author_len = commit.author.chars().count();

        let mut pos = 0;

        // Graph (colorful - based on column position)
        if self.h_offset < pos + graph_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos {
                0
            } else {
                pos - self.h_offset
            };
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
                buf.set_string(
                    x + screen_pos as u16,
                    y,
                    &ch.to_string(),
                    Style::new().fg(color),
                );
            }
        }
        pos += graph_len + 1; // +1 for space after graph

        // Hash (cyan) - now comes after graph + space
        if self.h_offset < pos + hash_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos {
                0
            } else {
                pos - self.h_offset
            };
            if screen_offset < width as usize && start < hash_len {
                let part: String = commit.short_id.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(
                        x + screen_offset as u16,
                        y,
                        &truncated,
                        Style::new().fg(theme.commit_hash),
                    );
                }
            }
        }
        pos += hash_len;

        // Refs with individual branch-type coloring - now comes after hash
        // refs_str format is " [main, origin/main]" so we need to match that
        if !commit.refs.is_empty() && self.h_offset < pos + refs_len {
            let screen_start = if self.h_offset > pos {
                0
            } else {
                pos - self.h_offset
            };

            // Render each ref with its own color
            // Format: " [ref1, ref2, ...]" - leading space, opening bracket, refs separated by ", ", closing bracket
            let mut current_ref_pos = 0usize; // position within refs_str

            // Leading " [" (2 chars)
            let bracket_open = " [";
            let bracket_open_len = bracket_open.chars().count();
            let visible_start = self.h_offset.saturating_sub(pos);

            if current_ref_pos + bracket_open_len > visible_start
                && current_ref_pos < visible_start + width as usize
            {
                let skip = visible_start.saturating_sub(current_ref_pos);
                let screen_x = if current_ref_pos >= visible_start {
                    screen_start + (current_ref_pos - visible_start)
                } else {
                    screen_start
                };
                if screen_x < width as usize {
                    let available = (width as usize).saturating_sub(screen_x);
                    let part: String = bracket_open.chars().skip(skip).take(available).collect();
                    if !part.is_empty() {
                        // Use first ref's color for the opening bracket
                        let color = if let Some(first_ref) = commit.refs.first() {
                            self.get_single_ref_color(first_ref, theme)
                        } else {
                            theme.foreground
                        };
                        buf.set_string(x + screen_x as u16, y, &part, Style::new().fg(color));
                    }
                }
            }
            current_ref_pos += bracket_open_len;

            // Render each ref with color - handle "local -> remote" format
            for (i, ref_name) in commit.refs.iter().enumerate() {
                let separator = if i == 0 { "" } else { ", " };

                // Check if this ref has "local -> remote" format
                if let Some(arrow_pos) = ref_name.find(" -> ") {
                    let local_part = &ref_name[..arrow_pos];
                    let arrow_and_remote = &ref_name[arrow_pos..];

                    // Render separator
                    if !separator.is_empty() {
                        let sep_end = current_ref_pos + separator.len();
                        if sep_end > visible_start
                            && current_ref_pos < visible_start + width as usize
                        {
                            let skip = visible_start.saturating_sub(current_ref_pos);
                            let screen_x = if current_ref_pos >= visible_start {
                                screen_start + (current_ref_pos - visible_start)
                            } else {
                                screen_start
                            };
                            if screen_x < width as usize {
                                let available = (width as usize).saturating_sub(screen_x);
                                let part: String =
                                    separator.chars().skip(skip).take(available).collect();
                                if !part.is_empty() {
                                    buf.set_string(
                                        x + screen_x as u16,
                                        y,
                                        &part,
                                        Style::new().fg(theme.branch_local),
                                    );
                                }
                            }
                        }
                        current_ref_pos += separator.len();
                    }

                    // Render local part with proper color (green for current branch)
                    let local_color = self.get_single_ref_color(local_part, theme);
                    let local_len = local_part.chars().count();
                    let local_end = current_ref_pos + local_len;
                    if local_end > visible_start && current_ref_pos < visible_start + width as usize
                    {
                        let skip = visible_start.saturating_sub(current_ref_pos);
                        let screen_x = if current_ref_pos >= visible_start {
                            screen_start + (current_ref_pos - visible_start)
                        } else {
                            screen_start
                        };
                        if screen_x < width as usize {
                            let available = (width as usize).saturating_sub(screen_x);
                            let part: String =
                                local_part.chars().skip(skip).take(available).collect();
                            if !part.is_empty() {
                                buf.set_string(
                                    x + screen_x as u16,
                                    y,
                                    &part,
                                    Style::new().fg(local_color),
                                );
                            }
                        }
                    }
                    current_ref_pos = local_end;

                    // Render " -> remote" part (purple)
                    let remote_len = arrow_and_remote.chars().count();
                    let remote_end = current_ref_pos + remote_len;
                    if remote_end > visible_start
                        && current_ref_pos < visible_start + width as usize
                    {
                        let skip = visible_start.saturating_sub(current_ref_pos);
                        let screen_x = if current_ref_pos >= visible_start {
                            screen_start + (current_ref_pos - visible_start)
                        } else {
                            screen_start
                        };
                        if screen_x < width as usize {
                            let available = (width as usize).saturating_sub(screen_x);
                            let part: String = arrow_and_remote
                                .chars()
                                .skip(skip)
                                .take(available)
                                .collect();
                            if !part.is_empty() {
                                buf.set_string(
                                    x + screen_x as u16,
                                    y,
                                    &part,
                                    Style::new().fg(theme.branch_remote),
                                );
                            }
                        }
                    }
                    current_ref_pos = remote_end;
                } else {
                    // Regular ref without " -> "
                    let ref_display = format!("{}{}", separator, ref_name);
                    let ref_len = ref_display.chars().count();
                    let ref_end = current_ref_pos + ref_len;

                    if ref_end > visible_start && current_ref_pos < visible_start + width as usize {
                        let skip = visible_start.saturating_sub(current_ref_pos);
                        let screen_x = if current_ref_pos >= visible_start {
                            screen_start + (current_ref_pos - visible_start)
                        } else {
                            screen_start
                        };

                        if screen_x < width as usize {
                            let available = (width as usize).saturating_sub(screen_x);
                            let part: String =
                                ref_display.chars().skip(skip).take(available).collect();
                            if !part.is_empty() {
                                let color = self.get_single_ref_color(ref_name, theme);
                                buf.set_string(
                                    x + screen_x as u16,
                                    y,
                                    &part,
                                    Style::new().fg(color),
                                );
                            }
                        }
                    }
                    current_ref_pos = ref_end;
                }
            }

            // Closing bracket "]"
            let close_pos = current_ref_pos;
            if close_pos + 1 > visible_start && close_pos < visible_start + width as usize {
                let screen_x = if close_pos >= visible_start {
                    screen_start + (close_pos - visible_start)
                } else {
                    screen_start
                };
                if screen_x < width as usize {
                    // Use last ref's color for the closing bracket
                    let color = if let Some(last_ref) = commit.refs.last() {
                        self.get_single_ref_color(last_ref, theme)
                    } else {
                        theme.foreground
                    };
                    buf.set_string(x + screen_x as u16, y, "]", Style::new().fg(color));
                }
            }
        }
        pos += refs_len + 1; // +1 for space before author

        // Author (gray)
        if self.h_offset < pos + author_len {
            let start = self.h_offset.saturating_sub(pos);
            let screen_offset = if self.h_offset > pos {
                0
            } else {
                pos - self.h_offset
            };
            if screen_offset < width as usize && start < author_len {
                let part: String = commit.author.chars().skip(start).collect();
                let max_len = (width as usize).saturating_sub(screen_offset);
                let display_len = part.chars().count().min(max_len);
                if display_len > 0 {
                    let truncated: String = part.chars().take(display_len).collect();
                    buf.set_string(
                        x + screen_offset as u16,
                        y,
                        &truncated,
                        Style::new().fg(theme.commit_author),
                    );
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
            // Check if it's the current branch
            if let Some(ref current) = self.current_branch {
                if r == current {
                    return theme.branch_current;
                }
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
