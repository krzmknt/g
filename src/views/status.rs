use crate::config::Theme;
use crate::git::{FileStatus, StatusEntry};
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, ListState, Scrollbar, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    Staged,
    Unstaged,
    Untracked,
}

pub struct StatusView {
    pub staged: Vec<StatusEntry>,
    pub unstaged: Vec<StatusEntry>,
    pub untracked: Vec<StatusEntry>,
    pub section: Section,
    pub list_state: ListState,
    pub scroll: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
    pub search_query: Option<String>,
    pub search_results: Vec<(Section, usize)>, // (section, index within section)
}

impl StatusView {
    pub fn new() -> Self {
        Self {
            staged: Vec::new(),
            unstaged: Vec::new(),
            untracked: Vec::new(),
            section: Section::Unstaged,
            list_state: ListState::new().with_selected(Some(0)),
            scroll: 0,
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

    pub fn update(&mut self, entries: Vec<StatusEntry>) {
        self.update_internal(entries, true);
    }

    /// Update without resetting scroll position (for background refresh)
    pub fn update_preserve_scroll(&mut self, entries: Vec<StatusEntry>) {
        self.update_internal(entries, false);
    }

    fn update_internal(&mut self, entries: Vec<StatusEntry>, reset_scroll: bool) {
        let old_section = self.section;
        let old_selected = self.list_state.selected();

        self.staged.clear();
        self.unstaged.clear();
        self.untracked.clear();

        for entry in entries {
            if entry.staged.is_changed() {
                self.staged.push(entry.clone());
            }
            if entry.unstaged == FileStatus::Untracked {
                self.untracked.push(entry);
            } else if entry.unstaged.is_changed() {
                self.unstaged.push(entry);
            }
        }

        if reset_scroll {
            // Select first available section
            if !self.staged.is_empty() {
                self.section = Section::Staged;
            } else if !self.unstaged.is_empty() {
                self.section = Section::Unstaged;
            } else if !self.untracked.is_empty() {
                self.section = Section::Untracked;
            }
            self.list_state.select(Some(0));
        } else {
            // Preserve section if still has items, otherwise switch
            let section_len = match old_section {
                Section::Staged => self.staged.len(),
                Section::Unstaged => self.unstaged.len(),
                Section::Untracked => self.untracked.len(),
            };
            if section_len == 0 {
                // Need to switch section
                if !self.staged.is_empty() {
                    self.section = Section::Staged;
                } else if !self.unstaged.is_empty() {
                    self.section = Section::Unstaged;
                } else if !self.untracked.is_empty() {
                    self.section = Section::Untracked;
                }
                self.list_state.select(Some(0));
            } else {
                // Keep section, clamp selected
                self.section = old_section;
                let new_selected = old_selected.unwrap_or(0).min(section_len.saturating_sub(1));
                self.list_state.select(Some(new_selected));
            }
        }
    }

    pub fn current_items(&self) -> &[StatusEntry] {
        match self.section {
            Section::Staged => &self.staged,
            Section::Unstaged => &self.unstaged,
            Section::Untracked => &self.untracked,
        }
    }

    pub fn selected_entry(&self) -> Option<&StatusEntry> {
        self.list_state
            .selected()
            .and_then(|i| self.current_items().get(i))
    }

    pub fn move_up(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected - 1));
            } else {
                // At top of current section, try to move to previous section
                self.previous_section();
            }
        }
    }

    pub fn move_down(&mut self) {
        let len = self.current_items().len();
        if let Some(selected) = self.list_state.selected() {
            if selected + 1 < len {
                self.list_state.select(Some(selected + 1));
            } else {
                // At bottom of current section, try to move to next section
                self.next_section();
            }
        }
    }

    pub fn move_to_top(&mut self) {
        self.list_state.select(Some(0));
    }

    pub fn move_to_bottom(&mut self) {
        let len = self.current_items().len();
        if len > 0 {
            self.list_state.select(Some(len - 1));
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        // Calculate absolute row from scroll position
        let abs_row = self.scroll + row;

        // Track position through sections
        let mut pos = 0;

        // Staged section
        if !self.staged.is_empty() {
            if abs_row == pos {
                // Clicked on header - select first item in staged
                self.section = Section::Staged;
                self.list_state.select(Some(0));
                return;
            }
            pos += 1; // header
            if abs_row < pos + self.staged.len() {
                self.section = Section::Staged;
                self.list_state.select(Some(abs_row - pos));
                return;
            }
            pos += self.staged.len();
        }

        // Unstaged section
        if !self.unstaged.is_empty() {
            if abs_row == pos {
                // Clicked on header - select first item in unstaged
                self.section = Section::Unstaged;
                self.list_state.select(Some(0));
                return;
            }
            pos += 1; // header
            if abs_row < pos + self.unstaged.len() {
                self.section = Section::Unstaged;
                self.list_state.select(Some(abs_row - pos));
                return;
            }
            pos += self.unstaged.len();
        }

        // Untracked section
        if !self.untracked.is_empty() {
            if abs_row == pos {
                // Clicked on header - select first item in untracked
                self.section = Section::Untracked;
                self.list_state.select(Some(0));
                return;
            }
            pos += 1; // header
            if abs_row < pos + self.untracked.len() {
                self.section = Section::Untracked;
                self.list_state.select(Some(abs_row - pos));
                return;
            }
        }
    }

    fn next_section(&mut self) {
        match self.section {
            Section::Staged if !self.unstaged.is_empty() => {
                self.section = Section::Unstaged;
                self.list_state.select(Some(0));
            }
            Section::Staged if !self.untracked.is_empty() => {
                self.section = Section::Untracked;
                self.list_state.select(Some(0));
            }
            Section::Unstaged if !self.untracked.is_empty() => {
                self.section = Section::Untracked;
                self.list_state.select(Some(0));
            }
            // Wrap around: from last section back to first
            Section::Untracked if !self.staged.is_empty() => {
                self.section = Section::Staged;
                self.list_state.select(Some(0));
            }
            Section::Untracked if !self.unstaged.is_empty() => {
                self.section = Section::Unstaged;
                self.list_state.select(Some(0));
            }
            Section::Unstaged if !self.staged.is_empty() => {
                self.section = Section::Staged;
                self.list_state.select(Some(0));
            }
            _ => {
                // Wrap within same section
                self.list_state.select(Some(0));
            }
        }
    }

    fn previous_section(&mut self) {
        match self.section {
            Section::Untracked if !self.unstaged.is_empty() => {
                self.section = Section::Unstaged;
                self.list_state
                    .select(Some(self.unstaged.len().saturating_sub(1)));
            }
            Section::Untracked if !self.staged.is_empty() => {
                self.section = Section::Staged;
                self.list_state
                    .select(Some(self.staged.len().saturating_sub(1)));
            }
            Section::Unstaged if !self.staged.is_empty() => {
                self.section = Section::Staged;
                self.list_state
                    .select(Some(self.staged.len().saturating_sub(1)));
            }
            // Wrap around: from first section back to last
            Section::Staged if !self.untracked.is_empty() => {
                self.section = Section::Untracked;
                self.list_state
                    .select(Some(self.untracked.len().saturating_sub(1)));
            }
            Section::Staged if !self.unstaged.is_empty() => {
                self.section = Section::Unstaged;
                self.list_state
                    .select(Some(self.unstaged.len().saturating_sub(1)));
            }
            Section::Unstaged if !self.untracked.is_empty() => {
                self.section = Section::Untracked;
                self.list_state
                    .select(Some(self.untracked.len().saturating_sub(1)));
            }
            _ => {
                // Wrap within same section
                let len = self.current_items().len();
                if len > 0 {
                    self.list_state.select(Some(len - 1));
                }
            }
        }
    }

    pub fn total_items(&self) -> usize {
        // Count all items including section headers
        let mut total = 0;
        if !self.staged.is_empty() {
            total += 1 + self.staged.len(); // header + items
        }
        if !self.unstaged.is_empty() {
            total += 1 + self.unstaged.len();
        }
        if !self.untracked.is_empty() {
            total += 1 + self.untracked.len();
        }
        total
    }

    pub fn ensure_visible(&mut self, visible_height: usize) {
        // Get the absolute position of the selected item
        let selected_abs = self.get_absolute_position();

        if selected_abs < self.scroll {
            self.scroll = selected_abs;
        } else if selected_abs >= self.scroll + visible_height {
            self.scroll = selected_abs.saturating_sub(visible_height - 1);
        }
    }

    fn get_absolute_position(&self) -> usize {
        let selected = self.list_state.selected().unwrap_or(0);
        let mut pos = 0;

        match self.section {
            Section::Staged => {
                pos += 1; // header
                pos += selected;
            }
            Section::Unstaged => {
                if !self.staged.is_empty() {
                    pos += 1 + self.staged.len();
                }
                pos += 1; // header
                pos += selected;
            }
            Section::Untracked => {
                if !self.staged.is_empty() {
                    pos += 1 + self.staged.len();
                }
                if !self.unstaged.is_empty() {
                    pos += 1 + self.unstaged.len();
                }
                pos += 1; // header
                pos += selected;
            }
        }
        pos
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        // Search in staged
        for (i, entry) in self.staged.iter().enumerate() {
            if entry.path.to_lowercase().contains(&query_lower) {
                self.search_results.push((Section::Staged, i));
            }
        }

        // Search in unstaged
        for (i, entry) in self.unstaged.iter().enumerate() {
            if entry.path.to_lowercase().contains(&query_lower) {
                self.search_results.push((Section::Unstaged, i));
            }
        }

        // Search in untracked
        for (i, entry) in self.untracked.iter().enumerate() {
            if entry.path.to_lowercase().contains(&query_lower) {
                self.search_results.push((Section::Untracked, i));
            }
        }

        // Jump to first result
        if let Some(&(section, idx)) = self.search_results.first() {
            self.section = section;
            self.list_state.select(Some(idx));
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

        let current = (self.section, self.list_state.selected().unwrap_or(0));

        // Find next result after current position
        if let Some(pos) = self
            .search_results
            .iter()
            .position(|&(sec, idx)| sec > current.0 || (sec == current.0 && idx > current.1))
        {
            let (section, idx) = self.search_results[pos];
            self.section = section;
            self.list_state.select(Some(idx));
        } else {
            // Wrap around to first result
            let (section, idx) = self.search_results[0];
            self.section = section;
            self.list_state.select(Some(idx));
        }
    }

    pub fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        let current = (self.section, self.list_state.selected().unwrap_or(0));

        // Find previous result before current position
        if let Some(pos) = self
            .search_results
            .iter()
            .rposition(|&(sec, idx)| sec < current.0 || (sec == current.0 && idx < current.1))
        {
            let (section, idx) = self.search_results[pos];
            self.section = section;
            self.list_state.select(Some(idx));
        } else {
            // Wrap around to last result
            let (section, idx) = *self.search_results.last().unwrap();
            self.section = section;
            self.list_state.select(Some(idx));
        }
    }

    fn is_search_match(&self, section: Section, index: usize) -> bool {
        self.search_results.contains(&(section, index))
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let block = Block::new()
            .title(" Status ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 {
            return;
        }

        let visible_height = inner.height as usize;
        self.ensure_visible(visible_height);

        // Build all lines
        let mut lines: Vec<(String, Style, bool)> = Vec::new(); // (text, style, is_selected)

        // Staged section (always green)
        if !self.staged.is_empty() {
            let header = format!("Staged ({})", self.staged.len());
            lines.push((header, Style::new().bold().fg(theme.staged), false));

            for (i, entry) in self.staged.iter().enumerate() {
                let is_selected =
                    self.section == Section::Staged && self.list_state.selected() == Some(i);
                let is_search_match = self.is_search_match(Section::Staged, i);
                let status_char = entry.staged.symbol();
                let line = format!("  {}  {}", status_char, entry.path);
                let style = if is_selected && focused {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if is_search_match {
                    Style::new().fg(theme.diff_hunk)
                } else {
                    Style::new().fg(theme.staged)
                };
                lines.push((line, style, is_selected && focused));
            }
        }

        // Unstaged section
        if !self.unstaged.is_empty() {
            let header = format!("Unstaged ({})", self.unstaged.len());
            lines.push((header, Style::new().bold().fg(theme.foreground), false));

            for (i, entry) in self.unstaged.iter().enumerate() {
                let is_selected =
                    self.section == Section::Unstaged && self.list_state.selected() == Some(i);
                let is_search_match = self.is_search_match(Section::Unstaged, i);
                let status_char = entry.unstaged.symbol();
                let status_color = Self::status_color(status_char, theme);
                let line = format!("  {}  {}", status_char, entry.path);
                let style = if is_selected && focused {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if is_search_match {
                    Style::new().fg(theme.diff_hunk)
                } else {
                    Style::new().fg(status_color)
                };
                lines.push((line, style, is_selected && focused));
            }
        }

        // Untracked section
        if !self.untracked.is_empty() {
            let header = format!("Untracked ({})", self.untracked.len());
            lines.push((header, Style::new().bold().fg(theme.foreground), false));

            for (i, entry) in self.untracked.iter().enumerate() {
                let is_selected =
                    self.section == Section::Untracked && self.list_state.selected() == Some(i);
                let is_search_match = self.is_search_match(Section::Untracked, i);
                let status_char = entry.unstaged.symbol();
                let status_color = Self::status_color(status_char, theme);
                let line = format!("  {}  {}", status_char, entry.path);
                let style = if is_selected && focused {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if is_search_match {
                    Style::new().fg(theme.diff_hunk)
                } else {
                    Style::new().fg(status_color)
                };
                lines.push((line, style, is_selected && focused));
            }
        }

        // Render visible lines
        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        // Calculate max content width and store view width
        self.view_width = content_width as usize;
        self.max_content_width = lines
            .iter()
            .map(|(line, _, _)| line.chars().count())
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

        for (i, (line, style, is_highlighted)) in lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            // Fill full line width with background color when highlighted
            if *is_highlighted {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, *style);
            }
            // Apply horizontal scroll
            let display_line: String = line.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_line, content_width, *style);
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(lines.len(), visible_height, self.scroll);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn status_color(status_char: char, theme: &Theme) -> crate::tui::Color {
        match status_char {
            'M' => theme.unstaged,
            'A' => theme.staged,
            'D' => theme.diff_remove,
            '?' => theme.untracked,
            _ => theme.foreground,
        }
    }

    pub fn staged_count(&self) -> usize {
        self.staged.len()
    }

    pub fn is_empty(&self) -> bool {
        self.staged.is_empty() && self.unstaged.is_empty() && self.untracked.is_empty()
    }
}
