use crate::config::Theme;
use crate::git::SubmoduleInfo;
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

pub struct SubmodulesView {
    pub submodules: Vec<SubmoduleInfo>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub max_content_width: usize,
    pub view_width: usize,
}

impl SubmodulesView {
    pub fn new() -> Self {
        Self {
            submodules: Vec::new(),
            selected: 0,
            offset: 0,
            h_offset: 0,
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
        self.max_content_width > self.view_width
            && self.h_offset < self.max_content_width.saturating_sub(self.view_width)
    }

    pub fn scroll_left(&mut self) {
        self.h_offset = self.h_offset.saturating_sub(4);
    }

    pub fn scroll_right(&mut self) {
        self.h_offset += 4;
    }

    pub fn update(&mut self, submodules: Vec<SubmoduleInfo>) {
        self.submodules = submodules;
        if self.selected >= self.submodules.len() && !self.submodules.is_empty() {
            self.selected = self.submodules.len() - 1;
        }
    }

    pub fn selected_submodule(&self) -> Option<&SubmoduleInfo> {
        self.submodules.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.submodules.is_empty() && self.selected + 1 < self.submodules.len() {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.submodules.is_empty() {
            self.selected = self.submodules.len() - 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        if index < self.submodules.len() {
            self.selected = index;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let title = format!(" Submodules ({}) ", self.submodules.len());

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

        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1);

        // Calculate max content width and store view width
        self.view_width = content_width as usize;
        self.max_content_width = self
            .submodules
            .iter()
            .map(|sm| {
                let status_width = 2; // "✓ " or "○ "
                let name_width = sm.name.chars().count();
                let path_width = sm.path.chars().count() + 3; // " (path)"
                let head_width = sm.head.as_ref().map(|h| h.len() + 3).unwrap_or(4); // " [head]" or " [-]"
                status_width + name_width + path_width + head_width
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

        if self.submodules.is_empty() {
            let msg = "No submodules";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, sm) in self
                .submodules
                .iter()
                .skip(self.offset)
                .take(height)
                .enumerate()
            {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;

                let style = if is_selected && focused {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if sm.is_initialized {
                    Style::new().fg(theme.staged)
                } else {
                    Style::new().fg(theme.untracked)
                };

                // Fill full line width when selected and focused
                if is_selected && focused {
                    let blank_line = " ".repeat(content_width as usize);
                    buf.set_string(inner.x, y, &blank_line, style);
                }

                let status = if sm.is_initialized { "✓" } else { "○" };
                let head = sm.head.as_deref().unwrap_or("-");
                let line = format!("{} {} ({}) [{}]", status, sm.name, sm.path, head);
                // Apply horizontal scroll
                let display_line: String = line.chars().skip(self.h_offset).collect();
                buf.set_string_truncated(inner.x, y, &display_line, content_width, style);
            }
        }

        let scrollbar = Scrollbar::new(self.submodules.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
