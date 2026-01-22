use crate::git::{BranchInfo, BranchType};
use crate::tui::{Buffer, Rect, Style};
use crate::config::Theme;
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
}

impl BranchesView {
    pub fn new() -> Self {
        Self {
            local: Vec::new(),
            remote: Vec::new(),
            show_remote: false,
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
        self.max_content_width > self.view_width &&
            self.h_offset < self.max_content_width.saturating_sub(self.view_width)
    }

    pub fn scroll_left(&mut self) {
        self.h_offset = self.h_offset.saturating_sub(4);
    }

    pub fn scroll_right(&mut self) {
        self.h_offset += 4;
    }

    pub fn update(&mut self, branches: Vec<BranchInfo>) {
        self.local.clear();
        self.remote.clear();

        for branch in branches {
            match branch.branch_type {
                BranchType::Local => self.local.push(branch),
                BranchType::Remote => self.remote.push(branch),
            }
        }

        // Select current branch
        for (i, branch) in self.local.iter().enumerate() {
            if branch.is_head {
                self.selected = i;
                break;
            }
        }
    }

    pub fn toggle_remote(&mut self) {
        self.show_remote = !self.show_remote;
    }

    pub fn visible_branches(&self) -> Vec<&BranchInfo> {
        let mut branches: Vec<&BranchInfo> = self.local.iter().collect();
        if self.show_remote {
            branches.extend(self.remote.iter());
        }
        branches
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
        let len = self.visible_branches().len();
        if len > 0 && self.selected + 1 < len {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        let len = self.visible_branches().len();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_unfocused };

        let block = Block::new()
            .title(" Branches ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
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

        // Calculate max content width and store view width
        self.view_width = content_width as usize;
        {
            let branches = self.visible_branches();
            self.max_content_width = branches.iter().map(|branch| {
                let indicator_width = 2; // "* " or "  "
                let commit_width = branch.last_commit.short_id.len() + 1;
                let name_width = branch.name.chars().count();
                let ahead_behind_width = if branch.ahead > 0 || branch.behind > 0 {
                    format!("  +{} -{}", branch.ahead, branch.behind).len()
                } else {
                    0
                };
                indicator_width + commit_width + name_width + ahead_behind_width
            }).max().unwrap_or(0) + 2; // +2 for scrollbar (1) + margin (1)
        }

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        let branches = self.visible_branches();
        for (i, branch) in branches.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;

            let indicator = if branch.is_head { "*" } else { " " };

            let name_color = match branch.branch_type {
                BranchType::Local if branch.is_head => theme.branch_current,
                BranchType::Local => theme.branch_local,
                BranchType::Remote => theme.branch_remote,
            };

            let style = if is_selected && focused {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else {
                Style::new().fg(name_color)
            };

            // Fill full line width when selected and focused
            if is_selected && focused {
                let blank_line = " ".repeat(content_width as usize);
                buf.set_string(inner.x, y, &blank_line, style);
            }

            // Format: * commit_id branch-name  +1 -2
            let mut line = format!("{} {} {}", indicator, branch.last_commit.short_id, branch.name);

            if branch.ahead > 0 || branch.behind > 0 {
                line.push_str(&format!("  +{} -{}", branch.ahead, branch.behind));
            }

            // Apply horizontal scroll
            let display_line: String = line.chars().skip(self.h_offset).collect();
            buf.set_string_truncated(inner.x, y, &display_line, content_width, style);
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(branches.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
