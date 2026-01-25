use crate::config::Theme;
use crate::tui::{Buffer, Rect, Style};
use crate::widgets::{Block, Borders, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelType {
    Status,
    Branches,
    Commits,
    Stash,
    Diff,
    Tags,
    Remotes,
    Worktrees,
    Submodules,
    Blame,
    Files,
    Conflicts,
    PullRequests,
    Issues,
    Actions,
    Releases,
}

impl PanelType {
    pub fn all() -> &'static [PanelType] {
        &[
            PanelType::Status,
            PanelType::Branches,
            PanelType::Commits,
            PanelType::Stash,
            PanelType::Diff,
            PanelType::Tags,
            PanelType::Remotes,
            PanelType::Worktrees,
            PanelType::Submodules,
            PanelType::Blame,
            PanelType::Files,
            PanelType::Conflicts,
            PanelType::PullRequests,
            PanelType::Issues,
            PanelType::Actions,
            PanelType::Releases,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PanelType::Status => "Status",
            PanelType::Branches => "Branches",
            PanelType::Commits => "Commits",
            PanelType::Stash => "Stash",
            PanelType::Diff => "Preview",
            PanelType::Tags => "Tags",
            PanelType::Remotes => "Remotes",
            PanelType::Worktrees => "Worktrees",
            PanelType::Submodules => "Submodules",
            PanelType::Blame => "Blame",
            PanelType::Files => "Files",
            PanelType::Conflicts => "Conflicts",
            PanelType::PullRequests => "PRs",
            PanelType::Issues => "Issues",
            PanelType::Actions => "Actions",
            PanelType::Releases => "Releases",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            PanelType::Status => "1",
            PanelType::Branches => "2",
            PanelType::Commits => "3",
            PanelType::Stash => "4",
            PanelType::Diff => "5",
            PanelType::Tags => "6",
            PanelType::Remotes => "7",
            PanelType::Worktrees => "8",
            PanelType::Submodules => "9",
            PanelType::Blame => "b",
            PanelType::Files => "f",
            PanelType::Conflicts => "x",
            PanelType::PullRequests => "p",
            PanelType::Issues => "i",
            PanelType::Actions => "a",
            PanelType::Releases => "e",
        }
    }
}

pub struct MenuView {
    pub panels: Vec<PanelType>,
    pub selected: usize,
    pub visible: bool,
}

impl MenuView {
    pub fn new() -> Self {
        Self {
            panels: PanelType::all().to_vec(),
            selected: 0,
            visible: false,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn selected_panel(&self) -> Option<PanelType> {
        self.panels.get(self.selected).copied()
    }

    pub fn move_up(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.panels.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        if self.selected + 1 < self.panels.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: &Theme,
        current_panel: Option<PanelType>,
    ) {
        if !self.visible {
            return;
        }

        let block = Block::new()
            .title(" Panels ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme.border_focused));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        for (i, panel) in self.panels.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let y = inner.y + i as u16;
            let is_selected = self.selected == i;
            let is_current = current_panel == Some(*panel);

            let style = if is_selected {
                Style::new().fg(theme.selection_text).bg(theme.selection)
            } else if is_current {
                Style::new().fg(theme.branch_current).bold()
            } else {
                Style::new().fg(theme.foreground)
            };

            let marker = if is_current { "* " } else { "  " };
            let line = format!("{}{} {}", marker, panel.shortcut(), panel.name());
            buf.set_string_truncated(inner.x, y, &line, inner.width, style);
        }
    }
}
