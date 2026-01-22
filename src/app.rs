use std::time::Duration;
use crate::error::Result;
use crate::config::{Config, Theme};
use crate::git::Repository;
use crate::tui::{Terminal, Buffer, Rect, Style, Color};
use crate::input::{EventReader, Event, KeyEvent, KeyCode, Modifiers, MouseEvent, MouseEventKind, MouseButton};
use crate::views::{
    StatusView, BranchesView, CommitsView, CommitsViewMode, DiffView, DiffMode, Section, StashView,
    TagsView, RemotesView, WorktreeView, SubmodulesView, BlameView, FileTreeView,
    ConflictView, MenuView, PanelType,
};
use crate::widgets::{Block, Borders, Widget};

// Re-export PanelType as Panel for backwards compatibility within app
pub use crate::views::PanelType as Panel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Command,
    Input(InputContext),
    Confirm(ConfirmAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    MultiPane,
    SinglePane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    CommitMessage,
    BranchName,
    SearchQuery,
    TagName,
    StashMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    BranchDelete,
    BranchForceDelete,
    Discard,
    Push,
    StashDrop,
    CommitRevert,
}


pub struct App {
    pub repo: Repository,
    pub config: Config,
    pub terminal: Terminal,
    pub event_reader: EventReader,

    pub focused_panel: PanelType,
    pub mode: Mode,
    pub view_mode: ViewMode,

    pub status_view: StatusView,
    pub branches_view: BranchesView,
    pub commits_view: CommitsView,
    pub stash_view: StashView,
    pub diff_view: DiffView,
    pub tags_view: TagsView,
    pub remotes_view: RemotesView,
    pub worktree_view: WorktreeView,
    pub submodules_view: SubmodulesView,
    pub blame_view: BlameView,
    pub filetree_view: FileTreeView,
    pub conflict_view: ConflictView,
    pub menu_view: MenuView,

    pub input_buffer: String,
    pub input_cursor: usize,
    pub message: Option<String>,

    pub should_quit: bool,

    // Border dragging state
    pub drag_state: Option<DragState>,

    // Branch creation source (for "create branch from" feature)
    pub branch_create_from: Option<String>,

    // Confirmation context (stores target name for confirmation dialog)
    pub confirm_target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DragState {
    pub drag_type: DragType,
}

#[derive(Debug, Clone)]
pub enum DragType {
    /// Dragging a column border (affects column width)
    ColumnBorder { col_idx: usize },
    /// Dragging a panel border within a column (affects panel heights)
    PanelBorder { col_idx: usize, panel_idx: usize },
    /// Dragging an intersection (T or cross) - affects both column width and panel heights
    Intersection { col_idx: usize, left_panel_idx: usize, right_panel_idx: usize },
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Repository::discover()?;
        let config = Config::load().unwrap_or_default();
        let terminal = Terminal::new()?;
        let event_reader = EventReader::new();

        Ok(Self {
            repo,
            config,
            terminal,
            event_reader,
            focused_panel: PanelType::Status,
            mode: Mode::Normal,
            view_mode: ViewMode::MultiPane,
            status_view: StatusView::new(),
            branches_view: BranchesView::new(),
            commits_view: CommitsView::new(),
            stash_view: StashView::new(),
            diff_view: DiffView::new(),
            tags_view: TagsView::new(),
            remotes_view: RemotesView::new(),
            worktree_view: WorktreeView::new(),
            submodules_view: SubmodulesView::new(),
            blame_view: BlameView::new(),
            filetree_view: FileTreeView::new(),
            conflict_view: ConflictView::new(),
            menu_view: MenuView::new(),
            input_buffer: String::new(),
            input_cursor: 0,
            message: None,
            should_quit: false,
            drag_state: None,
            branch_create_from: None,
            confirm_target: None,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.terminal.init()?;
        self.refresh_all()?;

        let mut frame_count = 0u64;
        loop {
            debug!("FRAME {}: focused_panel={:?}, drag_state={}",
                frame_count, self.focused_panel, self.drag_state.is_some());
            frame_count += 1;

            self.draw()?;

            let event = self.event_reader.read_event(Duration::from_millis(100))?;

            match event {
                Event::Key(key) => {
                    debug!("KEY EVENT: {:?}", key);
                    self.handle_key(key)?;
                }
                Event::Mouse(mouse) => self.handle_mouse(mouse)?,
                Event::None => {}
                _ => {
                    debug!("OTHER EVENT");
                }
            }

            if self.should_quit {
                break;
            }
        }

        self.terminal.restore()?;
        Ok(())
    }

    fn refresh_all(&mut self) -> Result<()> {
        self.refresh_status()?;
        self.refresh_branches()?;
        self.refresh_commits()?;
        self.refresh_stash()?;
        self.refresh_diff()?;
        self.refresh_tags()?;
        self.refresh_remotes()?;
        self.refresh_worktrees()?;
        self.refresh_submodules()?;
        self.refresh_conflicts()?;
        self.refresh_filetree()?;
        self.refresh_graph_commits()?;
        Ok(())
    }

    fn refresh_stash(&mut self) -> Result<()> {
        let stashes = self.repo.stash_list()?;
        self.stash_view.update(stashes);
        Ok(())
    }

    fn refresh_tags(&mut self) -> Result<()> {
        let tags = self.repo.tags()?;
        self.tags_view.update(tags);
        Ok(())
    }

    fn refresh_remotes(&mut self) -> Result<()> {
        let remotes = self.repo.remote_info()?;
        self.remotes_view.update(remotes);
        Ok(())
    }

    fn refresh_worktrees(&mut self) -> Result<()> {
        let worktrees = self.repo.worktrees()?;
        self.worktree_view.update(worktrees);
        Ok(())
    }

    fn refresh_submodules(&mut self) -> Result<()> {
        let submodules = self.repo.submodules()?;
        self.submodules_view.update(submodules);
        Ok(())
    }

    fn refresh_conflicts(&mut self) -> Result<()> {
        let conflicts = self.repo.conflicts()?;
        self.conflict_view.update(conflicts);
        Ok(())
    }

    fn refresh_filetree(&mut self) -> Result<()> {
        let tree = self.repo.file_tree(self.filetree_view.show_ignored)?;
        self.filetree_view.update(tree);
        Ok(())
    }

    fn refresh_graph_commits(&mut self) -> Result<()> {
        let commits = self.repo.log_graph(self.config.max_commits)?;
        self.commits_view.update_graph(commits);
        Ok(())
    }

    fn refresh_status(&mut self) -> Result<()> {
        let status = self.repo.status()?;
        self.status_view.update(status);
        Ok(())
    }

    fn refresh_branches(&mut self) -> Result<()> {
        let branches = self.repo.branches(self.branches_view.show_remote)?;
        self.branches_view.update(branches);
        Ok(())
    }

    fn refresh_commits(&mut self) -> Result<()> {
        let commits = self.repo.commits(self.config.max_commits)?;
        self.commits_view.update(commits);
        Ok(())
    }

    fn refresh_diff(&mut self) -> Result<()> {
        // Show diff based on status selection
        if let Some(entry) = self.status_view.selected_entry() {
            let diff = match self.status_view.section {
                Section::Staged => self.repo.diff_staged()?,
                _ => self.repo.diff_unstaged()?,
            };

            // Find the file in diff
            if let Some(file_idx) = diff.files.iter().position(|f| f.path == entry.path) {
                self.diff_view.update(diff);
                self.diff_view.current_file = file_idx;
            } else {
                self.diff_view.update(diff);
            }
        } else {
            self.diff_view.clear();
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let theme = self.config.current_theme().clone();
        let focused_panel = self.focused_panel;
        let mode = self.mode;
        let view_mode = self.view_mode;
        let message = self.message.clone();
        let input_buffer = self.input_buffer.clone();
        let branch_create_from = self.branch_create_from.clone();

        // Get repo info for header
        let repo_name = self.repo.name();
        let branch_name = self.repo.head_name().ok().flatten();
        let commit_hash = self.repo.head_commit_short().ok();
        let (ahead, behind) = self.repo.ahead_behind().unwrap_or((0, 0));
        let is_clean = self.repo.is_clean().unwrap_or(true);

        self.terminal.draw(|buf| {
            let area = buf.area;

            // Layout
            let (header, rest) = area.split_horizontal(1);
            let (main, footer) = rest.split_horizontal(rest.height.saturating_sub(3));

            // Header
            Self::render_header(buf, header, &theme, &repo_name, branch_name.as_deref(), commit_hash.as_deref(), ahead, behind, is_clean);

            match view_mode {
                ViewMode::MultiPane => {
                    // Column-based layout: render columns, then panels within each column
                    let mut col_x = main.x;
                    let num_columns = self.config.layout.columns.len();

                    for (col_idx, column) in self.config.layout.columns.iter().enumerate() {
                        // Last column fills remaining width to avoid gaps
                        let col_width = if col_idx == num_columns - 1 {
                            main.x + main.width - col_x
                        } else {
                            (main.width as f32 * column.width) as u16
                        };

                        let mut panel_y = main.y;
                        let num_panels = column.panels.len();

                        for (panel_idx, panel_height) in column.panels.iter().enumerate() {
                            // Last panel fills remaining height to avoid gaps
                            let panel_h = if panel_idx == num_panels - 1 {
                                main.y + main.height - panel_y
                            } else {
                                (main.height as f32 * panel_height.height) as u16
                            };

                            let panel_area = Rect::new(col_x, panel_y, col_width, panel_h);
                            let is_focused = focused_panel == panel_height.panel;

                            match panel_height.panel {
                                PanelType::Status => Self::render_status_panel(buf, panel_area, &theme, is_focused, &mut self.status_view),
                                PanelType::Branches => Self::render_branches_panel(buf, panel_area, &theme, is_focused, &mut self.branches_view),
                                PanelType::Commits => Self::render_commits_panel(buf, panel_area, &theme, is_focused, &mut self.commits_view),
                                PanelType::Stash => Self::render_stash_panel(buf, panel_area, &theme, is_focused, &mut self.stash_view),
                                PanelType::Diff => Self::render_diff_panel(buf, panel_area, &theme, is_focused, &mut self.diff_view),
                                PanelType::Tags => self.tags_view.render(panel_area, buf, &theme, is_focused),
                                PanelType::Remotes => self.remotes_view.render(panel_area, buf, &theme, is_focused),
                                PanelType::Worktrees => self.worktree_view.render(panel_area, buf, &theme, is_focused),
                                PanelType::Submodules => self.submodules_view.render(panel_area, buf, &theme, is_focused),
                                PanelType::Blame => self.blame_view.render(panel_area, buf, &theme, is_focused),
                                PanelType::Files => self.filetree_view.render(panel_area, buf, &theme, is_focused),
                                PanelType::Conflicts => self.conflict_view.render(panel_area, buf, &theme, is_focused),
                            }

                            panel_y += panel_h;
                        }

                        col_x += col_width;
                    }
                }
                ViewMode::SinglePane => {
                    // Full screen single panel
                    match focused_panel {
                        PanelType::Status => Self::render_status_panel(buf, main, &theme, true, &mut self.status_view),
                        PanelType::Branches => Self::render_branches_panel(buf, main, &theme, true, &mut self.branches_view),
                        PanelType::Commits => Self::render_commits_panel(buf, main, &theme, true, &mut self.commits_view),
                        PanelType::Stash => Self::render_stash_panel(buf, main, &theme, true, &mut self.stash_view),
                        PanelType::Diff => Self::render_diff_panel(buf, main, &theme, true, &mut self.diff_view),
                        PanelType::Tags => self.tags_view.render(main, buf, &theme, true),
                        PanelType::Remotes => self.remotes_view.render(main, buf, &theme, true),
                        PanelType::Worktrees => self.worktree_view.render(main, buf, &theme, true),
                        PanelType::Submodules => self.submodules_view.render(main, buf, &theme, true),
                        PanelType::Blame => self.blame_view.render(main, buf, &theme, true),
                        PanelType::Files => self.filetree_view.render(main, buf, &theme, true),
                        PanelType::Conflicts => self.conflict_view.render(main, buf, &theme, true),
                    }
                }
            }

            // Footer - pass scroll state for focused panel
            let (can_scroll_left, can_scroll_right) = match focused_panel {
                PanelType::Files => (self.filetree_view.can_scroll_left(), self.filetree_view.can_scroll_right()),
                PanelType::Commits => (self.commits_view.can_scroll_left(), self.commits_view.can_scroll_right()),
                PanelType::Branches => (self.branches_view.can_scroll_left(), self.branches_view.can_scroll_right()),
                PanelType::Stash => (self.stash_view.can_scroll_left(), self.stash_view.can_scroll_right()),
                PanelType::Tags => (self.tags_view.can_scroll_left(), self.tags_view.can_scroll_right()),
                PanelType::Remotes => (self.remotes_view.can_scroll_left(), self.remotes_view.can_scroll_right()),
                PanelType::Worktrees => (self.worktree_view.can_scroll_left(), self.worktree_view.can_scroll_right()),
                PanelType::Submodules => (self.submodules_view.can_scroll_left(), self.submodules_view.can_scroll_right()),
                PanelType::Blame => (self.blame_view.can_scroll_left(), self.blame_view.can_scroll_right()),
                PanelType::Conflicts => (self.conflict_view.can_scroll_left(), self.conflict_view.can_scroll_right()),
                PanelType::Status => (self.status_view.can_scroll_left(), self.status_view.can_scroll_right()),
                PanelType::Diff => (self.diff_view.can_scroll_left(), self.diff_view.can_scroll_right()),
            };
            Self::render_footer(buf, footer, &theme, mode, view_mode, message.as_deref(), &input_buffer, focused_panel, branch_create_from.as_deref(), self.confirm_target.as_deref(), can_scroll_left, can_scroll_right);

            // Logo at bottom right
            let logo = "g v0.1.0";
            let logo_x = buf.area.width.saturating_sub(logo.len() as u16 + 1);
            let logo_y = buf.area.height.saturating_sub(1);
            let logo_style = Style::new().fg(Color::Rgb(100, 100, 100));  // Dim gray
            buf.set_string(logo_x, logo_y, logo, logo_style);
        })?;

        Ok(())
    }

    fn render_header(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        repo_name: &str,
        branch: Option<&str>,
        commit_hash: Option<&str>,
        ahead: usize,
        behind: usize,
        is_clean: bool,
    ) {
        let branch_info = match branch {
            Some(name) => format!("branch: {}", name),
            None => format!("HEAD: {}", commit_hash.unwrap_or("unknown")),
        };

        let status_icon = if is_clean { "*" } else { "!" };
        let status_text = if is_clean { "clean" } else { "dirty" };
        let status_color = if is_clean { theme.staged } else { theme.unstaged };

        // Build header in parts to track positions correctly
        let prefix = format!(
            " g - [{}] | {} | +{} -{} | ",
            repo_name, branch_info, ahead, behind
        );
        let status_part = format!("{} {}", status_icon, status_text);

        // Render prefix in default style
        buf.set_string(area.x, area.y, &prefix, Style::new().fg(theme.foreground).bold());

        // Render status part in colored style
        buf.set_string(
            area.x + prefix.len() as u16,
            area.y,
            &status_part,
            Style::new().fg(status_color).bold(),
        );
    }

    fn render_status_panel(buf: &mut Buffer, area: Rect, theme: &Theme, focused: bool, view: &mut StatusView) {
        view.render(area, buf, theme, focused);
    }

    fn render_branches_panel(buf: &mut Buffer, area: Rect, theme: &Theme, focused: bool, view: &mut BranchesView) {
        view.render(area, buf, theme, focused);
    }

    fn render_commits_panel(buf: &mut Buffer, area: Rect, theme: &Theme, focused: bool, view: &mut CommitsView) {
        view.render(area, buf, theme, focused);
    }

    fn render_stash_panel(buf: &mut Buffer, area: Rect, theme: &Theme, focused: bool, view: &mut StashView) {
        view.render(area, buf, theme, focused);
    }

    fn render_diff_panel(buf: &mut Buffer, area: Rect, theme: &Theme, focused: bool, view: &mut DiffView) {
        view.render(area, buf, theme, focused);
    }

    fn render_footer(buf: &mut Buffer, area: Rect, theme: &Theme, mode: Mode, view_mode: ViewMode, message: Option<&str>, input: &str, focused_panel: Panel, branch_create_from: Option<&str>, confirm_target: Option<&str>, can_scroll_left: bool, can_scroll_right: bool) {
        // Message line (top of footer)
        if let Some(msg) = message {
            buf.set_string(area.x + 1, area.y, msg, Style::new().fg(theme.foreground));
        }

        // Key style helper
        let key_style = Style::new().fg(theme.branch_local).bold();
        let desc_style = Style::new().fg(theme.untracked);
        let sep_style = Style::new().fg(theme.border);

        match mode {
            Mode::Normal => {
                // Line 1: Global commands + panel indicator in single-pane mode
                let help_y1 = area.y + 1;

                if view_mode == ViewMode::SinglePane {
                    // Rich tab bar for zoom mode
                    Self::render_zoom_tab_bar(buf, area.x, help_y1, area.width, focused_panel, theme);
                } else {
                    // Build global commands with dynamic h/l scroll indicator
                    let scroll_cmd: Option<(&str, &str)> = match (can_scroll_left, can_scroll_right) {
                        (true, true) => Some(("h/l", "scroll")),
                        (true, false) => Some(("h", "scroll")),
                        (false, true) => Some(("l", "scroll")),
                        (false, false) => None,
                    };

                    let mut global_cmds_vec: Vec<(&str, &str)> = vec![
                        ("q", "quit"),
                        ("arrows", "focus"),
                        ("H/J/K/L", "resize"),
                        ("j/k", "move"),
                    ];

                    if let Some(scroll) = scroll_cmd {
                        global_cmds_vec.push(scroll);
                    }

                    global_cmds_vec.extend_from_slice(&[
                        ("Enter", "select"),
                        ("/", "search"),
                        ("r", "refresh"),
                        ("z", "zoom"),
                    ]);

                    Self::render_command_line(buf, area.x + 1, help_y1, &global_cmds_vec, key_style, desc_style, sep_style, area.width.saturating_sub(2));
                }

                // Line 2: Panel-specific commands
                let help_y2 = area.y + 2;
                let panel_cmds: &[(&str, &str)] = match focused_panel {
                    PanelType::Status => &[
                        ("a", "toggle stage"),
                        ("c", "commit"),
                        ("d", "discard"),
                        ("s", "stash"),
                        ("P", "push"),
                    ],
                    PanelType::Branches => &[
                        ("c", "create from"),
                        ("v", "view local/remote"),
                        ("d", "delete (merged)"),
                        ("D", "force delete"),
                    ],
                    PanelType::Commits => &[
                        ("Enter", "view diff"),
                        ("c", "checkout"),
                        ("R", "revert"),
                        ("v", "view mode"),
                    ],
                    PanelType::Stash => &[
                        ("Enter", "pop"),
                        ("a", "apply"),
                        ("d", "drop"),
                    ],
                    PanelType::Diff => &[
                        ("j/k", "scroll"),
                        ("v", "toggle inline/split"),
                    ],
                    PanelType::Tags => &[
                        ("n", "new tag"),
                        ("d", "delete"),
                    ],
                    PanelType::Remotes => &[
                        ("f", "fetch"),
                    ],
                    PanelType::Worktrees => &[
                    ],
                    PanelType::Submodules => &[
                        ("u", "update"),
                    ],
                    PanelType::Blame => &[
                        ("j/k", "scroll"),
                    ],
                    PanelType::Files => &[
                        ("Space/Enter", "open"),
                        ("v", "show ignored"),
                        ("b", "blame"),
                    ],
                    PanelType::Conflicts => &[
                        ("o", "use ours"),
                        ("t", "use theirs"),
                    ],
                };
                Self::render_command_line(buf, area.x + 1, help_y2, panel_cmds, key_style, desc_style, sep_style, area.width.saturating_sub(2));
            }
            Mode::Search | Mode::Input(_) => {
                let prompt: String = match mode {
                    Mode::Search => "/".to_string(),
                    Mode::Input(InputContext::CommitMessage) => "Commit: ".to_string(),
                    Mode::Input(InputContext::BranchName) => {
                        match branch_create_from {
                            Some(from) => format!("New branch from '{}': ", from),
                            None => "Branch: ".to_string(),
                        }
                    },
                    Mode::Input(InputContext::SearchQuery) => "Search: ".to_string(),
                    Mode::Input(InputContext::TagName) => "Tag: ".to_string(),
                    Mode::Input(InputContext::StashMessage) => "Stash: ".to_string(),
                    _ => "> ".to_string(),
                };
                let line = format!("{}{}", prompt, input);
                buf.set_string(area.x + 1, area.y + 1, &line, Style::new().fg(theme.foreground));

                // Show input help
                let input_help = [("Enter", "confirm"), ("Esc", "cancel")];
                Self::render_command_line(buf, area.x + 1, area.y + 2, &input_help, key_style, desc_style, sep_style, area.width.saturating_sub(2));
            }
            Mode::Command => {
                let line = format!(":{}", input);
                buf.set_string(area.x + 1, area.y + 1, &line, Style::new().fg(theme.foreground));
            }
            Mode::Confirm(action) => {
                let action_desc = match action {
                    ConfirmAction::BranchDelete => format!("Delete branch '{}'?", confirm_target.as_deref().unwrap_or("?")),
                    ConfirmAction::BranchForceDelete => format!("Force delete branch '{}'?", confirm_target.as_deref().unwrap_or("?")),
                    ConfirmAction::Discard => format!("Discard changes in '{}'?", confirm_target.as_deref().unwrap_or("?")),
                    ConfirmAction::Push => "Push to remote?".to_string(),
                    ConfirmAction::StashDrop => format!("Drop stash@{{{}}}?", confirm_target.as_deref().unwrap_or("?")),
                    ConfirmAction::CommitRevert => format!("Revert commit {}?", confirm_target.as_deref().map(|s| &s[..7.min(s.len())]).unwrap_or("?")),
                };
                let warn_style = Style::new().fg(theme.diff_remove).bold();
                buf.set_string(area.x + 1, area.y + 1, &action_desc, warn_style);

                let confirm_help = [("y", "yes"), ("n/Esc", "cancel")];
                Self::render_command_line(buf, area.x + 1, area.y + 2, &confirm_help, key_style, desc_style, sep_style, area.width.saturating_sub(2));
            }
        }
    }

    fn render_command_line(buf: &mut Buffer, x: u16, y: u16, cmds: &[(&str, &str)], key_style: Style, desc_style: Style, sep_style: Style, max_width: u16) {
        let mut current_x = x;
        for (i, (key, desc)) in cmds.iter().enumerate() {
            if current_x >= x + max_width {
                break;
            }

            // Add separator
            if i > 0 {
                buf.set_string(current_x, y, " | ", sep_style);
                current_x += 3;
            }

            // Key (use char count for proper width calculation)
            buf.set_string(current_x, y, key, key_style);
            current_x += key.chars().count() as u16;

            // Colon (no space)
            buf.set_string(current_x, y, ":", sep_style);
            current_x += 1;

            // Description
            buf.set_string(current_x, y, desc, desc_style);
            current_x += desc.chars().count() as u16;
        }
    }

    fn render_zoom_tab_bar(buf: &mut Buffer, x: u16, y: u16, width: u16, focused: PanelType, theme: &Theme) {
        // Simple tab bar with fixed-width tabs and blue highlight
        use crate::tui::Color;

        let panels: &[(PanelType, &str)] = &[
            (PanelType::Status, "Status"),
            (PanelType::Branches, "Branch"),
            (PanelType::Commits, "Commit"),
            (PanelType::Stash, "Stash"),
            (PanelType::Diff, "Diff"),
            (PanelType::Tags, "Tags"),
            (PanelType::Remotes, "Remote"),
            (PanelType::Worktrees, "Wktree"),
            (PanelType::Submodules, "Submod"),
            (PanelType::Blame, "Blame"),
            (PanelType::Files, "Files"),
            (PanelType::Conflicts, "Conflct"),
        ];

        // Fixed tab width (including padding)
        let tab_width: u16 = 8;
        let mut current_x = x + 1;

        // Blue background color for active tab
        let active_bg = Color::Rgb(30, 80, 140);
        let active_style = Style::new().fg(Color::White).bg(active_bg);
        let inactive_style = Style::new().fg(theme.untracked);

        for (panel_type, name) in panels.iter() {
            if current_x + tab_width > x + width - 10 {
                break;
            }

            // Pad name to fixed width
            let padded = format!("{:^width$}", name, width = tab_width as usize);

            if *panel_type == focused {
                buf.set_string(current_x, y, &padded, active_style);
            } else {
                buf.set_string(current_x, y, &padded, inactive_style);
            }

            current_x += tab_width;
        }

        // z:exit at the end
        let exit_str = " z:exit";
        let exit_x = x + width - exit_str.len() as u16 - 1;
        buf.set_string(exit_x, y, exit_str, Style::new().fg(theme.branch_local));
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Search => self.handle_search_key(key),
            Mode::Command => self.handle_command_key(key),
            Mode::Input(ctx) => self.handle_input_key(key, ctx),
            Mode::Confirm(action) => self.handle_confirm_key(key, action),
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent, action: ConfirmAction) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Execute the confirmed action
                self.execute_confirmed_action(action)?;
                self.mode = Mode::Normal;
                self.confirm_target = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Escape => {
                self.message = Some("Cancelled".to_string());
                self.mode = Mode::Normal;
                self.confirm_target = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_confirmed_action(&mut self, action: ConfirmAction) -> Result<()> {
        match action {
            ConfirmAction::BranchDelete => {
                if let Some(ref name) = self.confirm_target {
                    match self.repo.delete_branch(name, false) {
                        Ok(()) => {
                            self.message = Some(format!("Deleted branch '{}'", name));
                            self.refresh_branches()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Delete failed: {}", e));
                        }
                    }
                }
            }
            ConfirmAction::BranchForceDelete => {
                if let Some(ref name) = self.confirm_target {
                    match self.repo.delete_branch(name, true) {
                        Ok(()) => {
                            self.message = Some(format!("Force deleted branch '{}'", name));
                            self.refresh_branches()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Force delete failed: {}", e));
                        }
                    }
                }
            }
            ConfirmAction::Discard => {
                if let Some(ref path) = self.confirm_target {
                    match self.repo.discard_file(path) {
                        Ok(()) => {
                            self.message = Some(format!("Discarded changes in '{}'", path));
                            self.refresh_status()?;
                            self.refresh_diff()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Discard failed: {}", e));
                        }
                    }
                }
            }
            ConfirmAction::Push => {
                // Push is not yet implemented
                self.message = Some("Push not implemented yet".to_string());
            }
            ConfirmAction::StashDrop => {
                if let Some(ref index_str) = self.confirm_target {
                    if let Ok(index) = index_str.parse::<usize>() {
                        match self.repo.stash_drop(index) {
                            Ok(()) => {
                                self.message = Some(format!("Dropped stash@{{{}}}", index));
                                self.refresh_stash()?;
                            }
                            Err(e) => {
                                self.message = Some(format!("Stash drop failed: {}", e));
                            }
                        }
                    }
                }
            }
            ConfirmAction::CommitRevert => {
                if let Some(ref commit_id) = self.confirm_target {
                    match self.repo.revert_commit(commit_id) {
                        Ok(()) => {
                            self.message = Some("Reverted commit".to_string());
                            self.refresh_all()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Revert failed: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let (width, height) = self.terminal.size()?;
        let main_top = 1u16; // After header
        let main_height = height.saturating_sub(4); // header + footer

        debug!("mouse: {:?} at ({}, {}), drag_state: {}", mouse.kind, mouse.column, mouse.row, self.drag_state.is_some());

        match self.view_mode {
            ViewMode::MultiPane => {
                // If dragging, only handle drag and up events
                if self.drag_state.is_some() {
                    debug!("  IN DRAG MODE, event: {:?}", mouse.kind);
                    match mouse.kind {
                        MouseEventKind::Up(MouseButton::Left) => {
                            debug!("  DRAG END");
                            self.save_layout_config();
                            self.drag_state = None;
                            self.terminal.force_full_redraw();
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if let Some(ref drag_state) = self.drag_state.clone() {
                                self.resize_panels(&drag_state, mouse.column, mouse.row, width, main_top, main_height);
                                // Force full redraw to prevent border afterimages
                                self.terminal.force_full_redraw();
                            }
                        }
                        _ => {
                            debug!("  IGNORED (in drag mode)");
                        }
                    }
                } else {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            // Check if clicking on a border first
                            if let Some(drag_state) = self.detect_border_at(mouse.column, mouse.row, width, main_top, main_height) {
                                debug!("  DRAG START: {:?}", drag_state.drag_type);
                                self.drag_state = Some(drag_state);
                            } else if let Some(panel) = self.panel_at_position(mouse.column, mouse.row, width, main_top, main_height) {
                                debug!("  FOCUS CHANGE: {:?} -> {:?}", self.focused_panel, panel);
                                if self.focused_panel != panel {
                                    self.focused_panel = panel;
                                    self.terminal.force_full_redraw();
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if let Some(panel) = self.panel_at_position(mouse.column, mouse.row, width, main_top, main_height) {
                                self.scroll_panel_up(panel)?;
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if let Some(panel) = self.panel_at_position(mouse.column, mouse.row, width, main_top, main_height) {
                                self.scroll_panel_down(panel)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            ViewMode::SinglePane => {
                // In single pane mode, scroll affects the current panel
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        self.scroll_panel_up(self.focused_panel)?;
                    }
                    MouseEventKind::ScrollDown => {
                        self.scroll_panel_down(self.focused_panel)?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Detect if the mouse is on a border (column or panel) or intersection
    fn detect_border_at(&self, x: u16, y: u16, width: u16, main_top: u16, main_height: u16) -> Option<DragState> {
        if y < main_top || y >= main_top + main_height {
            return None;
        }

        let mouse_x_pct = x as f32 / width as f32;
        let mouse_y_pct = (y - main_top) as f32 / main_height as f32;

        const COL_THRESHOLD: f32 = 0.015;
        const PANEL_THRESHOLD: f32 = 0.025;

        // First, check for intersections (T or cross junctions)
        // An intersection occurs where a column border meets panel borders
        let mut col_x = 0.0f32;
        for (col_idx, column) in self.config.layout.columns.iter().enumerate() {
            col_x += column.width;

            // Check if mouse is near this column's right edge (but not the last column)
            if col_idx < self.config.layout.columns.len() - 1 {
                if (mouse_x_pct - col_x).abs() < COL_THRESHOLD {
                    // We're on a column border - check if also on a panel border
                    // Check left column's panel borders
                    let mut left_panel_y = 0.0f32;
                    for (left_panel_idx, panel) in column.panels.iter().enumerate() {
                        left_panel_y += panel.height;
                        if left_panel_idx < column.panels.len() - 1 {
                            if (mouse_y_pct - left_panel_y).abs() < PANEL_THRESHOLD {
                                // Found intersection with left column panel border
                                // Check if right column also has a panel border nearby
                                let right_col = &self.config.layout.columns[col_idx + 1];
                                let mut right_panel_y = 0.0f32;
                                for (right_panel_idx, rpanel) in right_col.panels.iter().enumerate() {
                                    right_panel_y += rpanel.height;
                                    if right_panel_idx < right_col.panels.len() - 1 {
                                        if (mouse_y_pct - right_panel_y).abs() < PANEL_THRESHOLD {
                                            // Cross intersection: both columns have panel borders here
                                            return Some(DragState {
                                                drag_type: DragType::Intersection {
                                                    col_idx,
                                                    left_panel_idx,
                                                    right_panel_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                // T intersection: only left column has panel border
                                return Some(DragState {
                                    drag_type: DragType::Intersection {
                                        col_idx,
                                        left_panel_idx,
                                        right_panel_idx: usize::MAX, // No right panel border
                                    },
                                });
                            }
                        }
                    }

                    // Check right column's panel borders (T from right side)
                    let right_col = &self.config.layout.columns[col_idx + 1];
                    let mut right_panel_y = 0.0f32;
                    for (right_panel_idx, rpanel) in right_col.panels.iter().enumerate() {
                        right_panel_y += rpanel.height;
                        if right_panel_idx < right_col.panels.len() - 1 {
                            if (mouse_y_pct - right_panel_y).abs() < PANEL_THRESHOLD {
                                // T intersection: only right column has panel border
                                return Some(DragState {
                                    drag_type: DragType::Intersection {
                                        col_idx,
                                        left_panel_idx: usize::MAX, // No left panel border
                                        right_panel_idx,
                                    },
                                });
                            }
                        }
                    }

                    // Just a column border, no panel intersection
                    return Some(DragState {
                        drag_type: DragType::ColumnBorder { col_idx },
                    });
                }
            }
        }

        // Check for panel borders (horizontal) within each column
        col_x = 0.0;
        for (col_idx, column) in self.config.layout.columns.iter().enumerate() {
            let col_right = col_x + column.width;

            // Check if mouse is within this column
            if mouse_x_pct >= col_x && mouse_x_pct < col_right {
                let mut panel_y = 0.0f32;
                for (panel_idx, panel) in column.panels.iter().enumerate() {
                    panel_y += panel.height;

                    // Check if mouse is near this panel's bottom edge (but not the last panel)
                    if panel_idx < column.panels.len() - 1 {
                        if (mouse_y_pct - panel_y).abs() < PANEL_THRESHOLD {
                            return Some(DragState {
                                drag_type: DragType::PanelBorder { col_idx, panel_idx },
                            });
                        }
                    }
                }
                break;
            }

            col_x = col_right;
        }

        None
    }

    /// Resize based on drag
    fn resize_panels(&mut self, drag_state: &DragState, x: u16, _y: u16, width: u16, main_top: u16, main_height: u16) {
        match &drag_state.drag_type {
            DragType::ColumnBorder { col_idx } => {
                let col_idx = *col_idx;
                if col_idx >= self.config.layout.columns.len() - 1 {
                    return;
                }

                let mouse_x_pct = x as f32 / width as f32;

                // Calculate where this column border should be
                let col_start: f32 = self.config.layout.columns[..col_idx]
                    .iter()
                    .map(|c| c.width)
                    .sum();

                let combined_width = self.config.layout.columns[col_idx].width
                    + self.config.layout.columns[col_idx + 1].width;

                let new_width = (mouse_x_pct - col_start).clamp(0.1, combined_width - 0.1);
                let next_width = combined_width - new_width;

                self.config.layout.columns[col_idx].width = new_width;
                self.config.layout.columns[col_idx + 1].width = next_width;
            }
            DragType::PanelBorder { col_idx, panel_idx } => {
                let col_idx = *col_idx;
                let panel_idx = *panel_idx;

                if col_idx >= self.config.layout.columns.len() {
                    return;
                }
                let column = &mut self.config.layout.columns[col_idx];
                if panel_idx >= column.panels.len() - 1 {
                    return;
                }

                let mouse_y_pct = (_y.saturating_sub(main_top)) as f32 / main_height as f32;

                // Calculate where this panel border should be
                let panel_start: f32 = column.panels[..panel_idx]
                    .iter()
                    .map(|p| p.height)
                    .sum();

                let combined_height = column.panels[panel_idx].height
                    + column.panels[panel_idx + 1].height;

                let new_height = (mouse_y_pct - panel_start).clamp(0.1, combined_height - 0.1);
                let next_height = combined_height - new_height;

                column.panels[panel_idx].height = new_height;
                column.panels[panel_idx + 1].height = next_height;
            }
            DragType::Intersection { col_idx, left_panel_idx, right_panel_idx } => {
                let col_idx = *col_idx;
                let left_panel_idx = *left_panel_idx;
                let right_panel_idx = *right_panel_idx;

                if col_idx >= self.config.layout.columns.len() - 1 {
                    return;
                }

                let mouse_x_pct = x as f32 / width as f32;
                let mouse_y_pct = (_y.saturating_sub(main_top)) as f32 / main_height as f32;

                // Resize column widths (same as ColumnBorder)
                let col_start: f32 = self.config.layout.columns[..col_idx]
                    .iter()
                    .map(|c| c.width)
                    .sum();

                let combined_width = self.config.layout.columns[col_idx].width
                    + self.config.layout.columns[col_idx + 1].width;

                let new_width = (mouse_x_pct - col_start).clamp(0.1, combined_width - 0.1);
                let next_width = combined_width - new_width;

                self.config.layout.columns[col_idx].width = new_width;
                self.config.layout.columns[col_idx + 1].width = next_width;

                // Resize left column's panel heights (if applicable)
                if left_panel_idx != usize::MAX {
                    let column = &mut self.config.layout.columns[col_idx];
                    if left_panel_idx < column.panels.len() - 1 {
                        let panel_start: f32 = column.panels[..left_panel_idx]
                            .iter()
                            .map(|p| p.height)
                            .sum();

                        let combined_height = column.panels[left_panel_idx].height
                            + column.panels[left_panel_idx + 1].height;

                        let new_height = (mouse_y_pct - panel_start).clamp(0.1, combined_height - 0.1);
                        let next_height = combined_height - new_height;

                        column.panels[left_panel_idx].height = new_height;
                        column.panels[left_panel_idx + 1].height = next_height;
                    }
                }

                // Resize right column's panel heights (if applicable)
                if right_panel_idx != usize::MAX {
                    let column = &mut self.config.layout.columns[col_idx + 1];
                    if right_panel_idx < column.panels.len() - 1 {
                        let panel_start: f32 = column.panels[..right_panel_idx]
                            .iter()
                            .map(|p| p.height)
                            .sum();

                        let combined_height = column.panels[right_panel_idx].height
                            + column.panels[right_panel_idx + 1].height;

                        let new_height = (mouse_y_pct - panel_start).clamp(0.1, combined_height - 0.1);
                        let next_height = combined_height - new_height;

                        column.panels[right_panel_idx].height = new_height;
                        column.panels[right_panel_idx + 1].height = next_height;
                    }
                }
            }
        }
    }

    /// Save the current layout config to file
    fn save_layout_config(&self) {
        let config_path = Config::config_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Generate TOML content for columns
        let mut content = String::new();

        // Read existing config and preserve non-layout settings
        if let Ok(existing) = std::fs::read_to_string(&config_path) {
            let mut in_columns_section = false;
            for line in existing.lines() {
                // Skip old column/panel definitions
                if line.starts_with("[[columns]]") {
                    in_columns_section = true;
                    continue;
                }
                if in_columns_section {
                    // Skip until we hit a non-indented, non-empty line that's not part of columns
                    if line.starts_with("[[") && !line.starts_with("[[columns]]") {
                        in_columns_section = false;
                    } else {
                        continue;
                    }
                }
                content.push_str(line);
                content.push('\n');
            }
        }

        // Add column configurations
        content.push_str("\n# Layout (auto-generated)\n");
        for column in &self.config.layout.columns {
            content.push_str("[[columns]]\n");
            content.push_str(&format!("width = {:.3}\n", column.width));
            content.push_str("panels = [\n");
            for panel in &column.panels {
                content.push_str(&format!(
                    "  {{ type = \"{}\", height = {:.3} }},\n",
                    panel_type_to_string(panel.panel),
                    panel.height
                ));
            }
            content.push_str("]\n\n");
        }

        let _ = std::fs::write(&config_path, content);
    }

    fn next_panel(&self) -> PanelType {
        let all_panels = self.available_panels();
        let current_idx = all_panels.iter().position(|p| *p == self.focused_panel).unwrap_or(0);
        let next_idx = (current_idx + 1) % all_panels.len();
        all_panels[next_idx]
    }

    fn prev_panel(&self) -> PanelType {
        let all_panels = self.available_panels();
        let current_idx = all_panels.iter().position(|p| *p == self.focused_panel).unwrap_or(0);
        let prev_idx = if current_idx == 0 { all_panels.len() - 1 } else { current_idx - 1 };
        all_panels[prev_idx]
    }

    fn available_panels(&self) -> Vec<PanelType> {
        match self.view_mode {
            ViewMode::SinglePane => {
                // All panels available in zoom mode
                vec![
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
                ]
            }
            ViewMode::MultiPane => self.config.layout.all_panels(),
        }
    }

    fn panel_at_position(&self, x: u16, y: u16, width: u16, main_top: u16, main_height: u16) -> Option<PanelType> {
        if y < main_top || y >= main_top + main_height {
            return None;
        }

        let mouse_x_pct = x as f32 / width as f32;
        let mouse_y_pct = (y - main_top) as f32 / main_height as f32;

        // Find which column and panel the mouse is in
        let mut col_x = 0.0f32;
        for column in &self.config.layout.columns {
            let col_right = col_x + column.width;

            if mouse_x_pct >= col_x && mouse_x_pct < col_right {
                let mut panel_y = 0.0f32;
                for panel in &column.panels {
                    let panel_bottom = panel_y + panel.height;

                    if mouse_y_pct >= panel_y && mouse_y_pct < panel_bottom {
                        return Some(panel.panel);
                    }

                    panel_y = panel_bottom;
                }
                break;
            }

            col_x = col_right;
        }

        None
    }

    fn scroll_panel_up(&mut self, panel: PanelType) -> Result<()> {
        match panel {
            PanelType::Status => {
                self.status_view.move_up();
                self.refresh_diff()?;
            }
            PanelType::Branches => self.branches_view.move_up(),
            PanelType::Commits => self.commits_view.move_up(),
            PanelType::Stash => self.stash_view.move_up(),
            PanelType::Diff => self.diff_view.scroll_up(),
            PanelType::Tags => self.tags_view.move_up(),
            PanelType::Remotes => self.remotes_view.move_up(),
            PanelType::Worktrees => self.worktree_view.move_up(),
            PanelType::Submodules => self.submodules_view.move_up(),
            PanelType::Blame => self.blame_view.move_up(),
            PanelType::Files => self.filetree_view.move_up(),
            PanelType::Conflicts => self.conflict_view.move_up(),
        }
        Ok(())
    }

    fn scroll_panel_down(&mut self, panel: PanelType) -> Result<()> {
        match panel {
            PanelType::Status => {
                self.status_view.move_down();
                self.refresh_diff()?;
            }
            PanelType::Branches => self.branches_view.move_down(),
            PanelType::Commits => self.commits_view.move_down(),
            PanelType::Stash => self.stash_view.move_down(),
            PanelType::Diff => self.diff_view.scroll_down(),
            PanelType::Tags => self.tags_view.move_down(),
            PanelType::Remotes => self.remotes_view.move_down(),
            PanelType::Worktrees => self.worktree_view.move_down(),
            PanelType::Submodules => self.submodules_view.move_down(),
            PanelType::Blame => self.blame_view.move_down(),
            PanelType::Files => self.filetree_view.move_down(),
            PanelType::Conflicts => self.conflict_view.move_down(),
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Quit
            KeyCode::Char('q') => {
                self.should_quit = true;
            }

            KeyCode::Tab => {
                self.focused_panel = self.next_panel();
                self.terminal.force_full_redraw();
            }

            KeyCode::BackTab => {
                self.focused_panel = self.prev_panel();
                self.terminal.force_full_redraw();
            }

            // Menu toggle
            KeyCode::Char('m') => {
                self.menu_view.toggle();
            }

            // Vim navigation (j/k for item movement within panel, g/G for top/bottom)
            KeyCode::Char('j') => self.item_down()?,
            KeyCode::Char('k') => self.item_up()?,
            KeyCode::Char('g') => self.item_top(),
            KeyCode::Char('G') => self.item_bottom(),

            // h/l for horizontal scrolling
            KeyCode::Char('h') => self.scroll_left(),
            KeyCode::Char('l') => self.scroll_right(),

            // Shift+hjkl for resizing panes/columns
            KeyCode::Char('K') => self.resize_focused_panel_height(-0.01),
            KeyCode::Char('J') => self.resize_focused_panel_height(0.01),
            KeyCode::Char('H') => self.resize_focused_column_width(-0.01),
            KeyCode::Char('L') => self.resize_focused_column_width(0.01),

            // Arrow keys for pane navigation (move to adjacent pane)
            KeyCode::Up => self.focus_pane_up(),
            KeyCode::Down => self.focus_pane_down(),
            KeyCode::Left => self.focus_pane_left(),
            KeyCode::Right => self.focus_pane_right(),

            // Actions
            KeyCode::Enter | KeyCode::Char(' ') => self.handle_enter()?,

            // Search
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.input_buffer.clear();
                self.input_cursor = 0;
            }

            // Command mode
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.input_buffer.clear();
                self.input_cursor = 0;
            }

            // Panel-specific actions
            KeyCode::Char('a') if self.focused_panel == PanelType::Status => {
                // Toggle: if there are staged files, unstage all; otherwise stage all
                if self.status_view.staged.is_empty() {
                    self.repo.stage_all()?;
                    self.refresh_status()?;
                    self.refresh_diff()?;
                    self.message = Some("Staged all files".to_string());
                } else {
                    self.repo.unstage_all()?;
                    self.refresh_status()?;
                    self.refresh_diff()?;
                    self.message = Some("Unstaged all files".to_string());
                }
            }

            KeyCode::Char('A') if self.focused_panel == PanelType::Status => {
                // Force unstage all (keep for explicit unstage)
                self.repo.unstage_all()?;
                self.refresh_status()?;
                self.refresh_diff()?;
                self.message = Some("Unstaged all files".to_string());
            }

            KeyCode::Char('c') if self.focused_panel == PanelType::Status => {
                if self.status_view.staged_count() > 0 {
                    self.mode = Mode::Input(InputContext::CommitMessage);
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                } else {
                    self.message = Some("No changes staged".to_string());
                }
            }

            // Stash save from Status panel
            KeyCode::Char('s') if self.focused_panel == PanelType::Status => {
                if !self.status_view.is_empty() {
                    self.mode = Mode::Input(InputContext::StashMessage);
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                } else {
                    self.message = Some("No changes to stash".to_string());
                }
            }

            // Discard changes (Status panel)
            KeyCode::Char('d') if self.focused_panel == PanelType::Status => {
                if let Some(entry) = self.status_view.selected_entry() {
                    self.confirm_target = Some(entry.path.clone());
                    self.mode = Mode::Confirm(ConfirmAction::Discard);
                }
            }

            // Push to remote (Status panel)
            KeyCode::Char('P') if self.focused_panel == PanelType::Status => {
                self.confirm_target = None;
                self.mode = Mode::Confirm(ConfirmAction::Push);
            }

            KeyCode::Char('c') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    self.branch_create_from = Some(branch.name.clone());
                    self.mode = Mode::Input(InputContext::BranchName);
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                }
            }

            KeyCode::Char('v') if self.focused_panel == PanelType::Branches => {
                self.branches_view.toggle_remote();
                self.refresh_branches()?;
            }

            // Branch delete (safe - only merged branches)
            KeyCode::Char('d') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    if branch.is_head {
                        self.message = Some("Cannot delete current branch".to_string());
                    } else {
                        self.confirm_target = Some(branch.name.clone());
                        self.mode = Mode::Confirm(ConfirmAction::BranchDelete);
                    }
                }
            }

            // Branch force delete (even unmerged branches)
            KeyCode::Char('D') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    if branch.is_head {
                        self.message = Some("Cannot delete current branch".to_string());
                    } else {
                        self.confirm_target = Some(branch.name.clone());
                        self.mode = Mode::Confirm(ConfirmAction::BranchForceDelete);
                    }
                }
            }

            // Commits panel actions
            KeyCode::Char('c') if self.focused_panel == PanelType::Commits => {
                // Checkout (detached HEAD)
                if let Some(commit) = self.commits_view.selected_commit() {
                    let short_id = commit.short_id.clone();
                    match self.repo.checkout_commit(&commit.id) {
                        Ok(()) => {
                            self.message = Some(format!("Checked out: {}", short_id));
                            self.refresh_all()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Checkout failed: {}", e));
                        }
                    }
                }
            }

            KeyCode::Char('R') if self.focused_panel == PanelType::Commits => {
                // Revert commit
                if let Some(commit) = self.commits_view.selected_commit() {
                    self.confirm_target = Some(commit.id.clone());
                    self.mode = Mode::Confirm(ConfirmAction::CommitRevert);
                }
            }

            KeyCode::Char('v') if self.focused_panel == PanelType::Commits => {
                self.commits_view.toggle_view_mode();
                // Load graph commits if switching to graph mode
                if self.commits_view.view_mode == CommitsViewMode::Graph {
                    self.refresh_graph_commits()?;
                }
                let mode_name = match self.commits_view.view_mode {
                    CommitsViewMode::Compact => "compact",
                    CommitsViewMode::Detailed => "detailed",
                    CommitsViewMode::Graph => "graph",
                };
                self.message = Some(format!("Commits view: {}", mode_name));
            }

            KeyCode::Char('v') if self.focused_panel == PanelType::Files => {
                self.filetree_view.toggle_show_ignored();
                self.refresh_filetree()?;
                let mode_name = if self.filetree_view.show_ignored {
                    "showing ignored files"
                } else {
                    "hiding ignored files"
                };
                self.message = Some(format!("Files view: {}", mode_name));
            }

            // Stash panel actions
            KeyCode::Char('a') if self.focused_panel == PanelType::Stash => {
                if let Some(stash) = self.stash_view.selected_stash() {
                    let index = stash.index;
                    match self.repo.stash_apply(index) {
                        Ok(()) => {
                            self.message = Some(format!("Applied stash@{{{}}}", index));
                            self.refresh_all()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Stash apply failed: {}", e));
                        }
                    }
                }
            }

            KeyCode::Char('d') if self.focused_panel == PanelType::Stash => {
                if let Some(stash) = self.stash_view.selected_stash() {
                    self.confirm_target = Some(stash.index.to_string());
                    self.mode = Mode::Confirm(ConfirmAction::StashDrop);
                }
            }

            // Diff view toggle (inline/split)
            KeyCode::Char('v') if self.focused_panel == PanelType::Diff => {
                self.diff_view.toggle_mode();
                let mode_name = match self.diff_view.mode {
                    DiffMode::Inline => "inline",
                    DiffMode::SideBySide => "side-by-side",
                };
                self.message = Some(format!("Diff mode: {}", mode_name));
            }

            KeyCode::Char('r') => {
                self.refresh_all()?;
                self.message = Some("Refreshed".to_string());
            }

            // Toggle view mode (zoom)
            KeyCode::Char('z') => {
                self.view_mode = match self.view_mode {
                    ViewMode::MultiPane => ViewMode::SinglePane,
                    ViewMode::SinglePane => ViewMode::MultiPane,
                };
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.commits_view.search(&self.input_buffer);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                self.execute_command()?;
                self.mode = Mode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_input_key(&mut self, key: KeyEvent, ctx: InputContext) -> Result<()> {
        match key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                self.submit_input(ctx)?;
                self.mode = Mode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn item_up(&mut self) -> Result<()> {
        match self.focused_panel {
            PanelType::Status => {
                self.status_view.move_up();
                self.refresh_diff()?;
            }
            PanelType::Branches => self.branches_view.move_up(),
            PanelType::Commits => self.commits_view.move_up(),
            PanelType::Stash => self.stash_view.move_up(),
            PanelType::Diff => self.diff_view.scroll_up(),
            PanelType::Tags => self.tags_view.move_up(),
            PanelType::Remotes => self.remotes_view.move_up(),
            PanelType::Worktrees => self.worktree_view.move_up(),
            PanelType::Submodules => self.submodules_view.move_up(),
            PanelType::Blame => self.blame_view.move_up(),
            PanelType::Files => self.filetree_view.move_up(),
            PanelType::Conflicts => self.conflict_view.move_up(),
        }
        Ok(())
    }

    fn item_down(&mut self) -> Result<()> {
        match self.focused_panel {
            PanelType::Status => {
                self.status_view.move_down();
                self.refresh_diff()?;
            }
            PanelType::Branches => self.branches_view.move_down(),
            PanelType::Commits => self.commits_view.move_down(),
            PanelType::Stash => self.stash_view.move_down(),
            PanelType::Diff => self.diff_view.scroll_down(),
            PanelType::Tags => self.tags_view.move_down(),
            PanelType::Remotes => self.remotes_view.move_down(),
            PanelType::Worktrees => self.worktree_view.move_down(),
            PanelType::Submodules => self.submodules_view.move_down(),
            PanelType::Blame => self.blame_view.move_down(),
            PanelType::Files => self.filetree_view.move_down(),
            PanelType::Conflicts => self.conflict_view.move_down(),
        }
        Ok(())
    }

    fn item_top(&mut self) {
        match self.focused_panel {
            PanelType::Status => self.status_view.move_to_top(),
            PanelType::Branches => self.branches_view.move_to_top(),
            PanelType::Commits => self.commits_view.move_to_top(),
            PanelType::Stash => self.stash_view.move_to_top(),
            PanelType::Diff => self.diff_view.scroll_to_top(),
            PanelType::Tags => self.tags_view.move_to_top(),
            PanelType::Remotes => self.remotes_view.move_to_top(),
            PanelType::Worktrees => self.worktree_view.move_to_top(),
            PanelType::Submodules => self.submodules_view.move_to_top(),
            PanelType::Blame => self.blame_view.move_to_top(),
            PanelType::Files => self.filetree_view.move_to_top(),
            PanelType::Conflicts => self.conflict_view.move_to_top(),
        }
    }

    fn item_bottom(&mut self) {
        match self.focused_panel {
            PanelType::Status => self.status_view.move_to_bottom(),
            PanelType::Branches => self.branches_view.move_to_bottom(),
            PanelType::Commits => self.commits_view.move_to_bottom(),
            PanelType::Stash => self.stash_view.move_to_bottom(),
            PanelType::Diff => self.diff_view.scroll_to_bottom(),
            PanelType::Tags => self.tags_view.move_to_bottom(),
            PanelType::Remotes => self.remotes_view.move_to_bottom(),
            PanelType::Worktrees => self.worktree_view.move_to_bottom(),
            PanelType::Submodules => self.submodules_view.move_to_bottom(),
            PanelType::Blame => self.blame_view.move_to_bottom(),
            PanelType::Files => self.filetree_view.move_to_bottom(),
            PanelType::Conflicts => self.conflict_view.move_to_bottom(),
        }
    }

    fn scroll_left(&mut self) {
        match self.focused_panel {
            PanelType::Status => self.status_view.scroll_left(),
            PanelType::Branches => self.branches_view.scroll_left(),
            PanelType::Commits => self.commits_view.scroll_left(),
            PanelType::Stash => self.stash_view.scroll_left(),
            PanelType::Diff => self.diff_view.scroll_left(),
            PanelType::Tags => self.tags_view.scroll_left(),
            PanelType::Remotes => self.remotes_view.scroll_left(),
            PanelType::Worktrees => self.worktree_view.scroll_left(),
            PanelType::Submodules => self.submodules_view.scroll_left(),
            PanelType::Blame => self.blame_view.scroll_left(),
            PanelType::Files => self.filetree_view.scroll_left(),
            PanelType::Conflicts => self.conflict_view.scroll_left(),
        }
    }

    fn scroll_right(&mut self) {
        match self.focused_panel {
            PanelType::Status => self.status_view.scroll_right(),
            PanelType::Branches => self.branches_view.scroll_right(),
            PanelType::Commits => self.commits_view.scroll_right(),
            PanelType::Stash => self.stash_view.scroll_right(),
            PanelType::Diff => self.diff_view.scroll_right(),
            PanelType::Tags => self.tags_view.scroll_right(),
            PanelType::Remotes => self.remotes_view.scroll_right(),
            PanelType::Worktrees => self.worktree_view.scroll_right(),
            PanelType::Submodules => self.submodules_view.scroll_right(),
            PanelType::Blame => self.blame_view.scroll_right(),
            PanelType::Files => self.filetree_view.scroll_right(),
            PanelType::Conflicts => self.conflict_view.scroll_right(),
        }
    }

    fn focus_pane_up(&mut self) {
        match self.view_mode {
            ViewMode::SinglePane => {
                // In zoom mode, up/down also cycles panels
                self.focused_panel = self.prev_panel();
                self.terminal.force_full_redraw();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_above(self.focused_panel) {
                    self.focused_panel = panel;
                    self.terminal.force_full_redraw();
                }
            }
        }
    }

    fn focus_pane_down(&mut self) {
        match self.view_mode {
            ViewMode::SinglePane => {
                // In zoom mode, up/down also cycles panels
                self.focused_panel = self.next_panel();
                self.terminal.force_full_redraw();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_below(self.focused_panel) {
                    self.focused_panel = panel;
                    self.terminal.force_full_redraw();
                }
            }
        }
    }

    fn focus_pane_left(&mut self) {
        match self.view_mode {
            ViewMode::SinglePane => {
                self.focused_panel = self.prev_panel();
                self.terminal.force_full_redraw();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_left(self.focused_panel) {
                    self.focused_panel = panel;
                    self.terminal.force_full_redraw();
                }
            }
        }
    }

    fn focus_pane_right(&mut self) {
        match self.view_mode {
            ViewMode::SinglePane => {
                self.focused_panel = self.next_panel();
                self.terminal.force_full_redraw();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_right(self.focused_panel) {
                    self.focused_panel = panel;
                    self.terminal.force_full_redraw();
                }
            }
        }
    }

    /// Resize the height of the focused panel (delta is percentage change)
    fn resize_focused_panel_height(&mut self, delta: f32) {
        if let Some((col_idx, panel_idx)) = self.config.layout.find_panel(self.focused_panel) {
            let column = &mut self.config.layout.columns[col_idx];

            // Need at least 2 panels in the column to resize
            if column.panels.len() < 2 {
                return;
            }

            // Determine which panels to adjust
            // If not the last panel, adjust current and next
            // If last panel, adjust previous and current
            let (idx1, idx2) = if panel_idx < column.panels.len() - 1 {
                (panel_idx, panel_idx + 1)
            } else {
                (panel_idx - 1, panel_idx)
            };

            let combined = column.panels[idx1].height + column.panels[idx2].height;

            // Apply delta to first panel (with clamping)
            let new_height1 = (column.panels[idx1].height + delta).clamp(0.1, combined - 0.1);
            let new_height2 = combined - new_height1;

            column.panels[idx1].height = new_height1;
            column.panels[idx2].height = new_height2;

            self.save_layout_config();
            self.terminal.force_full_redraw();
        }
    }

    /// Resize the width of the column containing the focused panel (delta is percentage change)
    fn resize_focused_column_width(&mut self, delta: f32) {
        if let Some((col_idx, _)) = self.config.layout.find_panel(self.focused_panel) {
            let num_columns = self.config.layout.columns.len();

            // Need at least 2 columns to resize
            if num_columns < 2 {
                return;
            }

            // Determine which columns to adjust
            // If not the last column, adjust current and next
            // If last column, adjust previous and current
            let (idx1, idx2) = if col_idx < num_columns - 1 {
                (col_idx, col_idx + 1)
            } else {
                (col_idx - 1, col_idx)
            };

            let combined = self.config.layout.columns[idx1].width
                + self.config.layout.columns[idx2].width;

            // Apply delta to first column (with clamping)
            let new_width1 = (self.config.layout.columns[idx1].width + delta).clamp(0.1, combined - 0.1);
            let new_width2 = combined - new_width1;

            self.config.layout.columns[idx1].width = new_width1;
            self.config.layout.columns[idx2].width = new_width2;

            self.save_layout_config();
            self.terminal.force_full_redraw();
        }
    }

    fn handle_enter(&mut self) -> Result<()> {
        match self.focused_panel {
            PanelType::Status => {
                if let Some(entry) = self.status_view.selected_entry().cloned() {
                    match self.status_view.section {
                        Section::Staged => {
                            self.repo.unstage_file(&entry.path)?;
                            self.message = Some(format!("Unstaged: {}", entry.path));
                        }
                        _ => {
                            self.repo.stage_file(&entry.path)?;
                            self.message = Some(format!("Staged: {}", entry.path));
                        }
                    }
                    self.refresh_status()?;
                    self.refresh_diff()?;
                }
            }
            PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    if !branch.is_head {
                        let name = branch.name.clone();
                        self.repo.switch_branch(&name)?;
                        self.message = Some(format!("Switched to: {}", name));
                        self.refresh_all()?;
                    }
                }
            }
            PanelType::Commits => {
                // Show commit in diff view
                self.focused_panel = PanelType::Diff;
            }
            PanelType::Stash => {
                // Pop selected stash (Enter = pop)
                if let Some(stash) = self.stash_view.selected_stash() {
                    let index = stash.index;
                    match self.repo.stash_pop(index) {
                        Ok(()) => {
                            self.message = Some(format!("Popped stash@{{{}}}", index));
                            self.refresh_all()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Stash pop failed: {}", e));
                        }
                    }
                }
            }
            PanelType::Diff => {
                // Stage/unstage hunk (TODO)
            }
            PanelType::Tags => {
                // Checkout tag (TODO)
            }
            PanelType::Remotes => {
                // Fetch remote (TODO)
            }
            PanelType::Worktrees => {
                // Switch to worktree (TODO)
            }
            PanelType::Submodules => {
                // Update submodule
                if let Some(sub) = self.submodules_view.selected_submodule() {
                    let path = sub.path.clone();
                    match self.repo.submodule_update(&path) {
                        Ok(()) => {
                            self.message = Some(format!("Updated submodule: {}", path));
                            self.refresh_submodules()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Submodule update failed: {}", e));
                        }
                    }
                }
            }
            PanelType::Blame => {
                // Nothing to do for blame
            }
            PanelType::Files => {
                // Expand/collapse or stage file
                if let Some(entry) = self.filetree_view.selected_entry() {
                    if entry.is_dir {
                        let path = entry.path.clone();
                        if let Some(load_path) = self.filetree_view.toggle_expand() {
                            // Lazy load children for this directory
                            let children = self.repo.file_tree_dir(&load_path, self.filetree_view.show_ignored)?;
                            self.filetree_view.load_children(&path, children);
                        }
                    } else {
                        // Stage or unstage file based on status
                        let path = entry.path.clone();
                        self.repo.stage_file(&path)?;
                        self.message = Some(format!("Staged: {}", path));
                        self.refresh_status()?;
                        self.refresh_filetree()?;
                    }
                }
            }
            PanelType::Conflicts => {
                // Open conflict resolution (show in diff)
                self.focused_panel = PanelType::Diff;
            }
        }
        Ok(())
    }

    fn execute_command(&mut self) -> Result<()> {
        let input = self.input_buffer.clone();
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["q"] | ["quit"] => {
                self.should_quit = true;
            }
            ["w"] | ["write"] => {
                if self.status_view.staged_count() > 0 {
                    self.mode = Mode::Input(InputContext::CommitMessage);
                } else {
                    self.message = Some("No changes staged".to_string());
                }
            }
            ["branch", name] => {
                let name = name.to_string();
                self.repo.create_branch(&name, None)?;
                self.refresh_branches()?;
                self.message = Some(format!("Created branch: {}", name));
            }
            ["checkout", name] => {
                let name = name.to_string();
                self.repo.switch_branch(&name)?;
                self.refresh_all()?;
                self.message = Some(format!("Switched to: {}", name));
            }
            ["stash"] => {
                self.repo.stash_save(None)?;
                self.refresh_status()?;
                self.message = Some("Changes stashed".to_string());
            }
            ["stash", "pop"] => {
                self.repo.stash_pop(0)?;
                self.refresh_status()?;
                self.message = Some("Stash popped".to_string());
            }
            ["tag", name] => {
                let name = name.to_string();
                self.repo.create_tag(&name, None)?;
                self.message = Some(format!("Created tag: {}", name));
            }
            ["fetch"] => {
                let remotes = self.repo.remotes()?;
                if let Some(remote) = remotes.first() {
                    self.repo.fetch(remote)?;
                    self.refresh_branches()?;
                    self.message = Some(format!("Fetched from: {}", remote));
                }
            }
            _ => {
                self.message = Some(format!("Unknown command: {}", input));
            }
        }

        Ok(())
    }

    fn submit_input(&mut self, ctx: InputContext) -> Result<()> {
        match ctx {
            InputContext::CommitMessage => {
                if !self.input_buffer.is_empty() {
                    let oid = self.repo.commit(&self.input_buffer)?;
                    self.message = Some(format!("Created commit: {}", &oid[..7]));
                    self.refresh_all()?;
                }
            }
            InputContext::BranchName => {
                if !self.input_buffer.is_empty() {
                    let from = self.branch_create_from.take();
                    self.repo.create_branch(&self.input_buffer, from.as_deref())?;
                    self.refresh_branches()?;
                    match from {
                        Some(ref f) => self.message = Some(format!("Created branch '{}' from '{}'", self.input_buffer, f)),
                        None => self.message = Some(format!("Created branch: {}", self.input_buffer)),
                    }
                }
            }
            InputContext::SearchQuery => {
                self.commits_view.search(&self.input_buffer);
            }
            InputContext::TagName => {
                if !self.input_buffer.is_empty() {
                    self.repo.create_tag(&self.input_buffer, None)?;
                    self.message = Some(format!("Created tag: {}", self.input_buffer));
                }
            }
            InputContext::StashMessage => {
                let msg = if self.input_buffer.is_empty() {
                    None
                } else {
                    Some(self.input_buffer.as_str())
                };
                self.repo.stash_save(msg)?;
                self.refresh_status()?;
                self.message = Some("Changes stashed".to_string());
            }
        }
        Ok(())
    }
}

fn panel_type_to_string(panel: PanelType) -> &'static str {
    match panel {
        PanelType::Status => "status",
        PanelType::Branches => "branches",
        PanelType::Commits => "commits",
        PanelType::Stash => "stash",
        PanelType::Diff => "diff",
        PanelType::Tags => "tags",
        PanelType::Remotes => "remotes",
        PanelType::Worktrees => "worktrees",
        PanelType::Submodules => "submodules",
        PanelType::Blame => "blame",
        PanelType::Files => "files",
        PanelType::Conflicts => "conflicts",
    }
}
