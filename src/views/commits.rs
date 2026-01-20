use crate::git::CommitInfo;
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct CommitsView {
    pub commits: Vec<CommitInfo>,
    pub selected: usize,
    pub offset: usize,
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
}

impl CommitsView {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            selected: 0,
            offset: 0,
            search_query: None,
            search_results: Vec::new(),
        }
    }

    pub fn update(&mut self, commits: Vec<CommitInfo>) {
        self.commits = commits;
        self.selected = 0;
        self.offset = 0;
        self.search_query = None;
        self.search_results.clear();
    }

    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.commits.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.commits.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            // Wrap to last item
            self.selected = self.commits.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.commits.is_empty() {
            return;
        }
        if self.selected + 1 < self.commits.len() {
            self.selected += 1;
        } else {
            // Wrap to first item
            self.selected = 0;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();
        for (i, commit) in self.commits.iter().enumerate() {
            if commit.message.to_lowercase().contains(&query_lower)
                || commit.author.to_lowercase().contains(&query_lower)
                || commit.short_id.to_lowercase().contains(&query_lower)
            {
                self.search_results.push(i);
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
        let border_color = if focused { theme.border_focused } else { theme.border };

        let title = if let Some(ref query) = self.search_query {
            format!(" Commits [/{} - {} matches] ", query, self.search_results.len())
        } else {
            format!(" Commits ({}) ", self.commits.len())
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

        for (i, commit) in self.commits.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;
            let is_search_match = self.search_results.contains(&(self.offset + i));

            let style = if is_selected {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_search_match {
                Style::new().fg(theme.diff_hunk)
            } else {
                Style::new().fg(theme.foreground)
            };

            // Format: hash  message  author  time
            let hash_style = if is_selected {
                style
            } else {
                Style::new().fg(theme.branch_local)
            };

            // Hash
            buf.set_string(inner.x, y, &commit.short_id, hash_style);

            // Message (truncated)
            let msg_x = inner.x + 9;
            let msg_width = content_width.saturating_sub(30);
            let message = if commit.message.len() > msg_width as usize {
                format!("{}...", &commit.message[..(msg_width as usize).saturating_sub(3)])
            } else {
                commit.message.clone()
            };
            buf.set_string_truncated(msg_x, y, &message, msg_width, style);

            // Author (right-aligned)
            let time_str = commit.relative_time();
            let author_x = inner.x + content_width.saturating_sub(time_str.len() as u16 + 1);
            let time_style = if is_selected {
                style
            } else {
                Style::new().fg(theme.untracked)
            };
            buf.set_string(author_x, y, &time_str, time_style);
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(self.commits.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
