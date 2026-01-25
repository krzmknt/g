use crate::config::Theme;
use crate::git::StashEntry;
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct StashView {
    pub stashes: Vec<StashEntry>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
}

impl StashView {
    pub fn new() -> Self {
        Self {
            stashes: Vec::new(),
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

    pub fn update(&mut self, stashes: Vec<StashEntry>) {
        self.stashes = stashes;
        if self.selected >= self.stashes.len() && !self.stashes.is_empty() {
            self.selected = self.stashes.len() - 1;
        }
    }

    pub fn selected_stash(&self) -> Option<&StashEntry> {
        self.stashes.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.stashes.is_empty() && self.selected + 1 < self.stashes.len() {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.stashes.is_empty() {
            self.selected = self.stashes.len() - 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        if index < self.stashes.len() {
            self.selected = index;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        for (i, stash) in self.stashes.iter().enumerate() {
            if stash.message.to_lowercase().contains(&query_lower) {
                self.search_results.push(i);
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

        let title = format!(" Stash ({}) ", self.stashes.len());

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

        // Adjust offset
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        // Calculate max content width and store view width
        self.view_width = content_width as usize;
        self.max_content_width = self
            .stashes
            .iter()
            .map(|stash| {
                format!("stash@{{{}}} {}", stash.index, stash.message)
                    .chars()
                    .count()
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

        if self.stashes.is_empty() {
            let msg = "No stashes";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, stash) in self
                .stashes
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
                    Style::new().fg(theme.foreground)
                };

                // Fill full line width when selected and focused
                if is_selected && focused {
                    let blank_line = " ".repeat(content_width as usize);
                    buf.set_string(inner.x, y, &blank_line, style);
                }

                // Format: stash@{0}: message
                let line = format!("stash@{{{}}} {}", stash.index, stash.message);
                // Apply horizontal scroll
                let display_line: String = line.chars().skip(self.h_offset).collect();
                buf.set_string_truncated(inner.x, y, &display_line, content_width, style);
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(self.stashes.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
