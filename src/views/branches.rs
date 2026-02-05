use crate::config::Theme;
use crate::git::{BranchInfo, BranchType};
use crate::tui::{Buffer, Color, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct BranchesView {
    pub local: Vec<BranchInfo>,
    pub remote: Vec<BranchInfo>,
    pub show_remote: bool,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
}

impl BranchesView {
    pub fn new(show_remote: bool) -> Self {
        Self {
            local: Vec::new(),
            remote: Vec::new(),
            show_remote,
            selected: 0,
            offset: 0,
            h_offset: 0,
            max_content_width: 0,
            view_width: 0,
            search_query: None,
            search_results: Vec::new(),
        }
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

    pub fn update(&mut self, branches: Vec<BranchInfo>) {
        self.update_internal(branches, true);
    }

    /// Update without resetting scroll position (for background refresh)
    pub fn update_preserve_scroll(&mut self, branches: Vec<BranchInfo>) {
        self.update_internal(branches, false);
    }

    fn update_internal(&mut self, branches: Vec<BranchInfo>, reset_scroll: bool) {
        let old_selected = self.selected;
        let old_show_remote = self.show_remote;

        self.local.clear();
        self.remote.clear();

        // Branches are already sorted by commit time (most recent first) in repository.rs
        for branch in branches {
            match branch.branch_type {
                BranchType::Local => self.local.push(branch),
                BranchType::Remote => self.remote.push(branch),
            }
        }

        if reset_scroll {
            // Select current branch
            for (i, branch) in self.local.iter().enumerate() {
                if branch.is_head {
                    self.selected = i;
                    break;
                }
            }
        } else {
            // Preserve scroll position, clamp to valid range
            let section_len = if old_show_remote {
                self.local.len() + self.remote.len()
            } else {
                self.local.len()
            };
            if section_len > 0 {
                self.selected = old_selected.min(section_len.saturating_sub(1));
            } else {
                self.selected = 0;
            }
        }
    }

    pub fn toggle_remote(&mut self) {
        self.show_remote = !self.show_remote;
    }

    pub fn visible_branches(&self) -> Vec<&BranchInfo> {
        let mut branches: Vec<&BranchInfo> = self.local.iter().collect();
        if self.show_remote {
            // Filter out remote branches that are tracked by local branches
            // This matches the logic in build_display_items() for consistency
            let tracked_remotes: std::collections::HashSet<&str> = self
                .local
                .iter()
                .filter_map(|b| b.upstream.as_ref().map(|u| u.name.as_str()))
                .collect();

            branches.extend(
                self.remote
                    .iter()
                    .filter(|b| !tracked_remotes.contains(b.name.as_str())),
            );
        }
        branches
    }

    /// Get the count of visible items
    fn visible_items_count(&self) -> usize {
        let count = self.local.len();
        if self.show_remote {
            // Filter out remote branches that are tracked by local branches
            // This matches the logic in build_display_items() for consistency
            let tracked_remotes: std::collections::HashSet<&str> = self
                .local
                .iter()
                .filter_map(|b| b.upstream.as_ref().map(|u| u.name.as_str()))
                .collect();

            count
                + self
                    .remote
                    .iter()
                    .filter(|b| !tracked_remotes.contains(b.name.as_str()))
                    .count()
        } else {
            count
        }
    }

    pub fn selected_branch(&self) -> Option<&BranchInfo> {
        self.visible_branches().get(self.selected).copied()
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let len = self.visible_items_count();
        if len > 0 && self.selected + 1 < len {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        let len = self.visible_items_count();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        let len = self.visible_items_count();
        if index < len {
            self.selected = index;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        for (i, branch) in self.local.iter().enumerate() {
            if branch.name.to_lowercase().contains(&query_lower) {
                self.search_results.push(i);
            }
        }
        if self.show_remote {
            let offset = self.local.len();
            for (i, branch) in self.remote.iter().enumerate() {
                if branch.name.to_lowercase().contains(&query_lower) {
                    self.search_results.push(offset + i);
                }
            }
        }

        // Jump to first result
        if let Some(&first) = self.search_results.first() {
            self.selected = first;
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_results.clear();
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

        let detail_indicator = if self.show_remote { " +Remotes" } else { "" };

        let title = format!(" Branches{} ", detail_indicator);
        let block = Block::new()
            .title(&title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        // Build display items - we need to collect all info upfront to avoid borrow issues
        let display_items = self.build_display_items(theme);
        let item_count = display_items.len();

        // Adjust offset
        let height = inner.height as usize;
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        // Calculate max content width and store view width
        self.view_width = content_width as usize;
        self.max_content_width = display_items
            .iter()
            .map(|item| {
                let upstream_len = item
                    .upstream_info
                    .as_ref()
                    .map(|(s, _)| s.chars().count())
                    .unwrap_or(0);
                item.line.chars().count() + upstream_len
            })
            .max()
            .unwrap_or(0)
            + 2; // +2 for scrollbar (1) + margin (1)

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        for (i, item) in display_items
            .iter()
            .skip(self.offset)
            .take(height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            let style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(item.color)
            };

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, style);
            }

            // Build full line with upstream info for horizontal scroll calculation
            let full_line = if let Some((ref upstream, _)) = item.upstream_info {
                format!("{}{}", item.line, upstream)
            } else {
                item.line.clone()
            };

            // Apply horizontal scroll
            let main_line_len = item.line.chars().count();

            if self.h_offset >= full_line.chars().count() {
                // Entire line is scrolled out of view
                continue;
            }

            if self.h_offset >= main_line_len {
                // Main line is scrolled out, only upstream info visible
                if let Some((ref upstream, upstream_color)) = item.upstream_info {
                    let upstream_offset = self.h_offset - main_line_len;
                    let display_upstream: String = upstream.chars().skip(upstream_offset).collect();
                    let upstream_style = if is_selected && focused {
                        Style::new().fg(theme.selection_text).bg(theme.selection)
                    } else if is_search_match {
                        Style::new().fg(theme.diff_hunk)
                    } else {
                        Style::new().fg(upstream_color)
                    };
                    buf.set_string_truncated(
                        inner.x,
                        y,
                        &display_upstream,
                        content_width,
                        upstream_style,
                    );
                }
            } else {
                // Main line is at least partially visible
                let display_main: String = item.line.chars().skip(self.h_offset).collect();
                buf.set_string_truncated(inner.x, y, &display_main, content_width, style);

                // Render upstream info if present and there's space
                if let Some((ref upstream, upstream_color)) = item.upstream_info {
                    let main_display_len = display_main.chars().count().min(content_width as usize);
                    if main_display_len < content_width as usize {
                        let remaining_width = content_width as usize - main_display_len;
                        let upstream_style = if is_selected && focused {
                            Style::new().fg(theme.selection_text).bg(theme.selection)
                        } else if is_search_match {
                            Style::new().fg(theme.diff_hunk)
                        } else {
                            Style::new().fg(upstream_color)
                        };
                        buf.set_string_truncated(
                            inner.x + main_display_len as u16,
                            y,
                            upstream,
                            remaining_width as u16,
                            upstream_style,
                        );
                    }
                }
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(item_count, height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    /// Build display items for rendering - collects all needed data upfront
    fn build_display_items(&self, theme: &Theme) -> Vec<DisplayItem> {
        let mut items = Vec::new();

        // show_remote acts as "detailed view" toggle:
        // - When false: Only local branches, no upstream info
        // - When true (detailed): Local branches with upstream info + untracked remote branches
        let show_upstream = self.show_remote;

        // Collect tracked remote branch names to filter them out (only needed in detailed view)
        let tracked_remotes: std::collections::HashSet<&str> = self
            .local
            .iter()
            .filter_map(|b| b.upstream.as_ref().map(|u| u.name.as_str()))
            .collect();

        // Branches are already sorted by commit time (most recent first) in repository.rs
        // This matches the order of commits in the commits graph view
        for branch in &self.local {
            items.push(self.branch_to_display_item(branch, theme, show_upstream));
        }
        if self.show_remote {
            for branch in &self.remote {
                // Skip remote branches that are tracked by local branches
                if !tracked_remotes.contains(branch.name.as_str()) {
                    items.push(self.branch_to_display_item(branch, theme, false));
                }
            }
        }

        items
    }

    fn branch_to_display_item(
        &self,
        branch: &BranchInfo,
        theme: &Theme,
        show_upstream: bool,
    ) -> DisplayItem {
        let color = match branch.branch_type {
            BranchType::Local if branch.is_head => theme.branch_current,
            BranchType::Local => theme.branch_local,
            BranchType::Remote => theme.branch_remote,
        };

        let indicator = if branch.is_head { "*" } else { " " };
        let mut line = format!(
            "{} {} {}",
            indicator, branch.last_commit.short_id, branch.name
        );

        // Add ahead/behind info
        if branch.ahead > 0 || branch.behind > 0 {
            line.push_str(&format!(" +{} -{}", branch.ahead, branch.behind));
        }

        // Add upstream tracking info for local branches (only when show_upstream is true)
        let upstream_info = if show_upstream {
            if let Some(ref upstream) = branch.upstream {
                Some((
                    format!(" -> {} {}", upstream.short_id, upstream.name),
                    theme.branch_remote,
                ))
            } else {
                None
            }
        } else {
            None
        };

        DisplayItem {
            line,
            color,
            upstream_info,
        }
    }
}

/// Pre-built display item to avoid borrow issues during rendering
struct DisplayItem {
    line: String,
    color: Color,
    /// Optional upstream info (text, color) to render after main line
    upstream_info: Option<(String, Color)>,
}
