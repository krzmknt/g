use crate::config::Theme;
use crate::git::ReleaseInfo;
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

use super::loading::{LoadingState, DEFAULT_TIMEOUT, SPINNER_FRAMES};

pub struct ReleasesView {
    pub releases: Vec<ReleaseInfo>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
    pub loading_state: LoadingState<()>,
}

impl ReleasesView {
    pub fn new() -> Self {
        Self {
            releases: Vec::new(),
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

    pub fn set_loaded(&mut self, releases: Vec<ReleaseInfo>) {
        self.releases = releases;
        self.loading_state = LoadingState::Loaded(());
        if !self.releases.is_empty() && self.selected >= self.releases.len() {
            self.selected = self.releases.len() - 1;
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

    pub fn update(&mut self, releases: Vec<ReleaseInfo>) {
        self.set_loaded(releases);
    }

    pub fn selected_release(&self) -> Option<&ReleaseInfo> {
        self.releases.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.releases.is_empty() && self.selected + 1 < self.releases.len() {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.releases.is_empty() {
            self.selected = self.releases.len() - 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        if index < self.releases.len() {
            self.selected = index;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        let matches: Vec<usize> = self
            .releases
            .iter()
            .enumerate()
            .filter(|(_, release)| {
                release.tag_name.to_lowercase().contains(&query_lower)
                    || release.name.to_lowercase().contains(&query_lower)
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
                format!(" Releases {} ", SPINNER_FRAMES[*spinner_frame])
            }
            LoadingState::Timeout => " Releases (timeout) ".to_string(),
            LoadingState::Error(_) => " Releases (error) ".to_string(),
            _ => format!(" Releases ({}) ", self.releases.len()),
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
                let msg = format!("{} Loading releases...", spinner);
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

        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        self.view_width = content_width as usize;
        self.max_content_width = self
            .releases
            .iter()
            .map(|release| {
                let tag_width = release.tag_name.chars().count();
                let name_width = if release.name != release.tag_name {
                    release.name.chars().count() + 3
                } else {
                    0
                };
                let status_width = if release.is_draft {
                    8 // " [DRAFT]"
                } else if release.is_prerelease {
                    13 // " [PRERELEASE]"
                } else {
                    0
                };
                tag_width + name_width + status_width + 2
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

        if self.releases.is_empty() {
            let msg = "No releases";
            buf.set_string(
                inner.x + 1,
                inner.y,
                msg,
                Style::new().fg(theme.border_unfocused),
            );
            return;
        }

        for (i, release) in self
            .releases
            .iter()
            .skip(self.offset)
            .take(height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            let status_color = if release.is_draft {
                theme.border_unfocused
            } else if release.is_prerelease {
                theme.staged
            } else {
                theme.diff_add
            };

            let style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(status_color)
            };

            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, style);
            }

            let name_part = if release.name != release.tag_name && !release.name.is_empty() {
                format!(" - {}", release.name)
            } else {
                String::new()
            };

            let status_part = if release.is_draft {
                " [DRAFT]"
            } else if release.is_prerelease {
                " [PRERELEASE]"
            } else {
                ""
            };

            let line = format!("{}{}{}", release.tag_name, name_part, status_part);

            let display_line: String = line.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_line, content_width, style);
        }

        let scrollbar = Scrollbar::new(self.releases.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
