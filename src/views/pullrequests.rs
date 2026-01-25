use crate::config::Theme;
use crate::git::PullRequestInfo;
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

use super::loading::{LoadingState, DEFAULT_TIMEOUT, SPINNER_FRAMES};

pub struct PullRequestsView {
    pub prs: Vec<PullRequestInfo>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
    pub loading_state: LoadingState<()>,
}

impl PullRequestsView {
    pub fn new() -> Self {
        Self {
            prs: Vec::new(),
            selected: 0,
            offset: 0,
            h_offset: 0,
            max_content_width: 0,
            view_width: 0,
            search_query: None,
            search_results: Vec::new(),
            loading_state: LoadingState::NotLoaded,
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

    pub fn start_loading(&mut self) {
        self.loading_state = LoadingState::start_loading();
    }

    /// Start background refresh (no spinner, keeps showing old data)
    pub fn start_background_refresh(&mut self) {
        self.loading_state = LoadingState::start_background_refresh();
    }

    pub fn set_loaded(&mut self, prs: Vec<PullRequestInfo>) {
        self.prs = prs;
        self.loading_state = LoadingState::Loaded(());
        if !self.prs.is_empty() && self.selected >= self.prs.len() {
            self.selected = self.prs.len() - 1;
        }
    }

    pub fn set_error(&mut self, error: String) {
        // On background refresh error, just stay in loaded state with old data
        if !self.loading_state.is_background_refreshing() {
            self.loading_state = LoadingState::Error(error);
        } else {
            self.loading_state = LoadingState::Loaded(());
        }
    }

    pub fn set_timeout(&mut self) {
        // On background refresh timeout, just stay in loaded state with old data
        if !self.loading_state.is_background_refreshing() {
            self.loading_state = LoadingState::Timeout;
        } else {
            self.loading_state = LoadingState::Loaded(());
        }
    }

    pub fn check_timeout(&mut self) -> bool {
        if self.loading_state.check_timeout(DEFAULT_TIMEOUT) {
            // On background refresh timeout, just stay in loaded state
            if self.loading_state.is_background_refreshing() {
                self.loading_state = LoadingState::Loaded(());
            } else {
                self.loading_state = LoadingState::Timeout;
            }
            true
        } else {
            false
        }
    }

    pub fn tick_spinner(&mut self) {
        self.loading_state.tick_spinner();
    }

    pub fn can_retry(&self) -> bool {
        self.loading_state.can_retry()
    }

    pub fn is_loading(&self) -> bool {
        self.loading_state.is_loading()
    }

    pub fn is_refreshing(&self) -> bool {
        self.loading_state.is_refreshing()
    }

    pub fn update(&mut self, prs: Vec<PullRequestInfo>) {
        self.set_loaded(prs);
    }

    pub fn selected_pr(&self) -> Option<&PullRequestInfo> {
        self.prs.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.prs.is_empty() && self.selected + 1 < self.prs.len() {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.prs.is_empty() {
            self.selected = self.prs.len() - 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        if index < self.prs.len() {
            self.selected = index;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        let matches: Vec<usize> = self
            .prs
            .iter()
            .enumerate()
            .filter(|(_, pr)| {
                pr.title.to_lowercase().contains(&query_lower)
                    || pr.author.login.to_lowercase().contains(&query_lower)
                    || pr.head_ref_name.to_lowercase().contains(&query_lower)
                    || pr.base_ref_name.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();

        self.search_results = matches;

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
            self.selected = *self.search_results.last().unwrap();
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let title = match &self.loading_state {
            LoadingState::Loading { spinner_frame, .. } => {
                format!(" PRs {} ", SPINNER_FRAMES[*spinner_frame])
            }
            LoadingState::Timeout => " PRs (timeout) ".to_string(),
            LoadingState::Error(_) => " PRs (error) ".to_string(),
            _ => format!(" PRs ({}) ", self.prs.len()),
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
        let content_width = inner.width.saturating_sub(1);

        // Handle special loading states
        match &self.loading_state {
            LoadingState::NotLoaded => {
                let msg = "Press R to load";
                let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(x, y, msg, Style::new().fg(theme.border_unfocused));
                return;
            }
            LoadingState::Loading { spinner_frame, .. } => {
                let spinner = SPINNER_FRAMES[*spinner_frame];
                let msg = format!("{} Loading pull requests...", spinner);
                let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(x, y, &msg, Style::new().fg(theme.staged));
                return;
            }
            LoadingState::Timeout => {
                let msg1 = "Request timed out";
                let msg2 = "Press R to retry";
                let x1 = inner.x + (inner.width.saturating_sub(msg1.len() as u16)) / 2;
                let x2 = inner.x + (inner.width.saturating_sub(msg2.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(
                    x1,
                    y.saturating_sub(1),
                    msg1,
                    Style::new().fg(theme.diff_remove),
                );
                buf.set_string(x2, y, msg2, Style::new().fg(theme.border_unfocused));
                return;
            }
            LoadingState::Error(err) => {
                let msg1 = format!("Error: {}", err);
                let msg2 = "Press R to retry";
                let x1 = inner.x + 1;
                let x2 = inner.x + (inner.width.saturating_sub(msg2.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string_truncated(
                    x1,
                    y.saturating_sub(1),
                    &msg1,
                    content_width,
                    Style::new().fg(theme.diff_remove),
                );
                buf.set_string(x2, y, msg2, Style::new().fg(theme.border_unfocused));
                return;
            }
            LoadingState::Loaded(_) | LoadingState::BackgroundRefreshing { .. } => {
                // Continue with normal rendering (background refresh shows old data)
            }
        }

        // Adjust offset for normal content rendering
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        self.view_width = content_width as usize;
        self.max_content_width = self
            .prs
            .iter()
            .map(|pr| {
                let number_width = format!("#{}", pr.number).len();
                let state_width = format!("[{}] ", pr.state).len();
                let title_width = pr.title.chars().count();
                let author_width = format!(" ({})", pr.author.login).len();
                let branches_width = format!(" {} <- {}", pr.base_ref_name, pr.head_ref_name).len();
                let stats_width = format!(" +{} -{}", pr.additions, pr.deletions).len();
                number_width
                    + state_width
                    + title_width
                    + author_width
                    + branches_width
                    + stats_width
                    + 2
            })
            .max()
            .unwrap_or(0)
            + 2;

        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        if self.prs.is_empty() {
            let msg = "No pull requests";
            buf.set_string(
                inner.x + 1,
                inner.y,
                msg,
                Style::new().fg(theme.border_unfocused),
            );
            return;
        }

        for (i, pr) in self.prs.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            let state_color = if pr.is_draft {
                theme.border_unfocused
            } else {
                match pr.state.as_str() {
                    "OPEN" => theme.diff_add,
                    "MERGED" => theme.diff_hunk,
                    "CLOSED" => theme.diff_remove,
                    _ => theme.foreground,
                }
            };

            let style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(state_color)
            };

            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, style);
            }

            let state_display = if pr.is_draft {
                "DRAFT".to_string()
            } else {
                pr.state.clone()
            };

            let line = format!(
                "#{} [{}] {} ({}) {} <- {} +{} -{}",
                pr.number,
                state_display,
                pr.title,
                pr.author.login,
                pr.base_ref_name,
                pr.head_ref_name,
                pr.additions,
                pr.deletions
            );

            let display_line: String = line.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_line, content_width, style);
        }

        let scrollbar = Scrollbar::new(self.prs.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
