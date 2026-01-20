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
}

impl BranchesView {
    pub fn new() -> Self {
        Self {
            local: Vec::new(),
            remote: Vec::new(),
            show_remote: false,
            selected: 0,
            offset: 0,
        }
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
        let len = self.visible_branches().len();
        if len == 0 {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            // Wrap to last item
            self.selected = len - 1;
        }
    }

    pub fn move_down(&mut self) {
        let len = self.visible_branches().len();
        if len == 0 {
            return;
        }
        if self.selected + 1 < len {
            self.selected += 1;
        } else {
            // Wrap to first item
            self.selected = 0;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border };

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

        let branches = self.visible_branches();
        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        for (i, branch) in branches.iter().skip(self.offset).take(height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = self.selected == self.offset + i;

            let indicator = if branch.is_head { "*" } else { " " };

            let name_color = match branch.branch_type {
                BranchType::Local if branch.is_head => theme.branch_current,
                BranchType::Local => theme.branch_local,
                BranchType::Remote => theme.branch_remote,
            };

            let style = if is_selected {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else {
                Style::new().fg(name_color)
            };

            // Format: * branch-name  +1 -2
            let mut line = format!("{} {}", indicator, branch.name);

            if branch.ahead > 0 || branch.behind > 0 {
                line.push_str(&format!("  +{} -{}", branch.ahead, branch.behind));
            }

            buf.set_string_truncated(inner.x, y, &line, content_width, style);
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(branches.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }
}
