use std::time::Duration;
use crate::error::Result;
use crate::config::{Config, Theme};
use crate::git::Repository;
use crate::tui::{Terminal, Buffer, Rect, Style, Color};
use crate::input::{EventReader, Event, KeyEvent, KeyCode, Modifiers};
use crate::views::{StatusView, BranchesView, CommitsView, DiffView, DiffMode, Section};
use crate::widgets::{Block, Borders, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Status,
    Branches,
    Commits,
    Diff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Command,
    Input(InputContext),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    CommitMessage,
    BranchName,
    SearchQuery,
    TagName,
    StashMessage,
}

pub struct App {
    pub repo: Repository,
    pub config: Config,
    pub terminal: Terminal,
    pub event_reader: EventReader,

    pub focused_panel: Panel,
    pub mode: Mode,

    pub status_view: StatusView,
    pub branches_view: BranchesView,
    pub commits_view: CommitsView,
    pub diff_view: DiffView,

    pub input_buffer: String,
    pub input_cursor: usize,
    pub message: Option<String>,

    pub should_quit: bool,
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
            focused_panel: Panel::Status,
            mode: Mode::Normal,
            status_view: StatusView::new(),
            branches_view: BranchesView::new(),
            commits_view: CommitsView::new(),
            diff_view: DiffView::new(),
            input_buffer: String::new(),
            input_cursor: 0,
            message: None,
            should_quit: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.terminal.init()?;
        self.refresh_all()?;

        loop {
            self.draw()?;

            let event = self.event_reader.read_event(Duration::from_millis(100))?;

            if let Event::Key(key) = event {
                self.handle_key(key)?;
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
        self.refresh_diff()?;
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
        let message = self.message.clone();
        let input_buffer = self.input_buffer.clone();

        // Get repo info for header
        let repo_name = self.repo.name();
        let branch_name = self.repo.head_name().ok().flatten();
        let commit_hash = self.repo.head_commit_short().ok();
        let (ahead, behind) = self.repo.ahead_behind().unwrap_or((0, 0));
        let is_clean = self.repo.is_clean().unwrap_or(true);

        self.terminal.draw(|buf| {
            let area = buf.area;

            // Clear with background color
            buf.set_style(area, Style::new().bg(theme.background));

            // Layout
            let (header, rest) = area.split_horizontal(1);
            let (main, footer) = rest.split_horizontal(rest.height.saturating_sub(3));

            // Header
            Self::render_header(buf, header, &theme, &repo_name, branch_name.as_deref(), commit_hash.as_deref(), ahead, behind, is_clean);

            // Calculate panel sizes
            let left_width = (main.width as f32 * 0.45).min(60.0) as u16;
            let (left, right) = main.split_vertical(left_width);

            // Left side: 3 panels
            let panel_height = left.height / 3;
            let (status_area, rest_left) = left.split_horizontal(panel_height);
            let (branches_area, commits_area) = rest_left.split_horizontal(panel_height);

            // Render panels
            Self::render_status_panel(buf, status_area, &theme, focused_panel == Panel::Status, &mut self.status_view);
            Self::render_branches_panel(buf, branches_area, &theme, focused_panel == Panel::Branches, &mut self.branches_view);
            Self::render_commits_panel(buf, commits_area, &theme, focused_panel == Panel::Commits, &mut self.commits_view);
            Self::render_diff_panel(buf, right, &theme, focused_panel == Panel::Diff, &mut self.diff_view);

            // Footer
            Self::render_footer(buf, footer, &theme, mode, message.as_deref(), &input_buffer, focused_panel);
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

    fn render_diff_panel(buf: &mut Buffer, area: Rect, theme: &Theme, focused: bool, view: &mut DiffView) {
        view.render(area, buf, theme, focused);
    }

    fn render_footer(buf: &mut Buffer, area: Rect, theme: &Theme, mode: Mode, message: Option<&str>, input: &str, focused_panel: Panel) {
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
                // Line 1: Global commands
                let help_y1 = area.y + 1;
                let global_cmds = [
                    ("q", "quit"),
                    ("Tab/←→", "panel"),
                    ("j/k", "move"),
                    ("Enter", "select"),
                    ("/", "search"),
                    ("r", "refresh"),
                    ("T", "theme"),
                ];
                Self::render_command_line(buf, area.x + 1, help_y1, &global_cmds, key_style, desc_style, sep_style, area.width.saturating_sub(2));

                // Line 2: Panel-specific commands
                let help_y2 = area.y + 2;
                let panel_cmds: &[(&str, &str)] = match focused_panel {
                    Panel::Status => &[
                        ("a", "stage all"),
                        ("A", "unstage all"),
                        ("c", "commit"),
                        ("d", "discard"),
                        ("s", "stash"),
                        ("P", "push"),
                    ],
                    Panel::Branches => &[
                        ("n", "new branch"),
                        ("t", "toggle local/remote"),
                        ("d", "delete (merged)"),
                        ("D", "force delete"),
                    ],
                    Panel::Commits => &[
                        ("Enter", "view diff"),
                        ("c", "checkout"),
                        ("R", "revert"),
                    ],
                    Panel::Diff => &[
                        ("j/k", "scroll"),
                        ("v", "toggle inline/split"),
                    ],
                };
                Self::render_command_line(buf, area.x + 1, help_y2, panel_cmds, key_style, desc_style, sep_style, area.width.saturating_sub(2));
            }
            Mode::Search | Mode::Input(_) => {
                let prompt = match mode {
                    Mode::Search => "/",
                    Mode::Input(InputContext::CommitMessage) => "Commit: ",
                    Mode::Input(InputContext::BranchName) => "Branch: ",
                    Mode::Input(InputContext::SearchQuery) => "Search: ",
                    Mode::Input(InputContext::TagName) => "Tag: ",
                    Mode::Input(InputContext::StashMessage) => "Stash: ",
                    _ => "> ",
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

            // Key
            buf.set_string(current_x, y, key, key_style);
            current_x += key.len() as u16;

            // Space
            buf.set_string(current_x, y, ":", sep_style);
            current_x += 1;

            // Description
            buf.set_string(current_x, y, desc, desc_style);
            current_x += desc.len() as u16;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Search => self.handle_search_key(key),
            Mode::Command => self.handle_command_key(key),
            Mode::Input(ctx) => self.handle_input_key(key, ctx),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Quit
            KeyCode::Char('q') => {
                self.should_quit = true;
            }

            // Panel navigation
            KeyCode::Char('1') => self.focused_panel = Panel::Status,
            KeyCode::Char('2') => self.focused_panel = Panel::Branches,
            KeyCode::Char('3') => self.focused_panel = Panel::Commits,
            KeyCode::Char('4') => self.focused_panel = Panel::Diff,

            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    Panel::Status => Panel::Branches,
                    Panel::Branches => Panel::Commits,
                    Panel::Commits => Panel::Diff,
                    Panel::Diff => Panel::Status,
                };
            }

            KeyCode::BackTab => {
                self.focused_panel = match self.focused_panel {
                    Panel::Status => Panel::Diff,
                    Panel::Branches => Panel::Status,
                    Panel::Commits => Panel::Branches,
                    Panel::Diff => Panel::Commits,
                };
            }

            // Vim navigation
            KeyCode::Char('j') | KeyCode::Down => self.move_down()?,
            KeyCode::Char('k') | KeyCode::Up => self.move_up()?,
            KeyCode::Char('h') | KeyCode::Left => self.move_left()?,
            KeyCode::Char('l') | KeyCode::Right => self.move_right()?,

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
            KeyCode::Char('a') if self.focused_panel == Panel::Status => {
                self.repo.stage_all()?;
                self.refresh_status()?;
                self.refresh_diff()?;
                self.message = Some("Staged all files".to_string());
            }

            KeyCode::Char('A') if self.focused_panel == Panel::Status => {
                self.repo.unstage_all()?;
                self.refresh_status()?;
                self.refresh_diff()?;
                self.message = Some("Unstaged all files".to_string());
            }

            KeyCode::Char('c') if self.focused_panel == Panel::Status => {
                if self.status_view.staged_count() > 0 {
                    self.mode = Mode::Input(InputContext::CommitMessage);
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                } else {
                    self.message = Some("No changes staged".to_string());
                }
            }

            KeyCode::Char('n') if self.focused_panel == Panel::Branches => {
                self.mode = Mode::Input(InputContext::BranchName);
                self.input_buffer.clear();
                self.input_cursor = 0;
            }

            KeyCode::Char('t') if self.focused_panel == Panel::Branches => {
                self.branches_view.toggle_remote();
                self.refresh_branches()?;
            }

            // Branch delete (safe - only merged branches)
            KeyCode::Char('d') if self.focused_panel == Panel::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    if branch.is_head {
                        self.message = Some("Cannot delete current branch".to_string());
                    } else {
                        let name = branch.name.clone();
                        match self.repo.delete_branch(&name, false) {
                            Ok(()) => {
                                self.message = Some(format!("Deleted branch: {}", name));
                                self.refresh_branches()?;
                            }
                            Err(e) => {
                                self.message = Some(format!("{}", e));
                            }
                        }
                    }
                }
            }

            // Branch force delete (even unmerged branches)
            KeyCode::Char('D') if self.focused_panel == Panel::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    if branch.is_head {
                        self.message = Some("Cannot delete current branch".to_string());
                    } else {
                        let name = branch.name.clone();
                        match self.repo.delete_branch(&name, true) {
                            Ok(()) => {
                                self.message = Some(format!("Force deleted branch: {}", name));
                                self.refresh_branches()?;
                            }
                            Err(e) => {
                                self.message = Some(format!("Failed to delete: {}", e));
                            }
                        }
                    }
                }
            }

            // Commits panel actions
            KeyCode::Char('c') if self.focused_panel == Panel::Commits => {
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

            KeyCode::Char('R') if self.focused_panel == Panel::Commits => {
                // Revert commit
                if let Some(commit) = self.commits_view.selected_commit() {
                    let short_id = commit.short_id.clone();
                    match self.repo.revert_commit(&commit.id) {
                        Ok(()) => {
                            self.message = Some(format!("Reverted: {}", short_id));
                            self.refresh_all()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Revert failed: {}", e));
                        }
                    }
                }
            }

            // Diff view toggle (inline/split)
            KeyCode::Char('v') if self.focused_panel == Panel::Diff => {
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

            // Theme toggle
            KeyCode::Char('T') => {
                self.config.toggle_theme();
                self.message = Some(format!("Theme: {:?}", self.config.theme));
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

    fn move_up(&mut self) -> Result<()> {
        match self.focused_panel {
            Panel::Status => {
                self.status_view.move_up();
                self.refresh_diff()?;
            }
            Panel::Branches => self.branches_view.move_up(),
            Panel::Commits => self.commits_view.move_up(),
            Panel::Diff => self.diff_view.scroll_up(),
        }
        Ok(())
    }

    fn move_down(&mut self) -> Result<()> {
        match self.focused_panel {
            Panel::Status => {
                self.status_view.move_down();
                self.refresh_diff()?;
            }
            Panel::Branches => self.branches_view.move_down(),
            Panel::Commits => self.commits_view.move_down(),
            Panel::Diff => self.diff_view.scroll_down(),
        }
        Ok(())
    }

    fn move_left(&mut self) -> Result<()> {
        // Left key cycles through panes (same as Shift+Tab)
        self.focused_panel = match self.focused_panel {
            Panel::Status => Panel::Diff,
            Panel::Branches => Panel::Status,
            Panel::Commits => Panel::Branches,
            Panel::Diff => Panel::Commits,
        };
        Ok(())
    }

    fn move_right(&mut self) -> Result<()> {
        // Right key cycles through panes (same as Tab)
        self.focused_panel = match self.focused_panel {
            Panel::Status => Panel::Branches,
            Panel::Branches => Panel::Commits,
            Panel::Commits => Panel::Diff,
            Panel::Diff => Panel::Status,
        };
        Ok(())
    }

    fn handle_enter(&mut self) -> Result<()> {
        match self.focused_panel {
            Panel::Status => {
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
            Panel::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    if !branch.is_head {
                        let name = branch.name.clone();
                        self.repo.switch_branch(&name)?;
                        self.message = Some(format!("Switched to: {}", name));
                        self.refresh_all()?;
                    }
                }
            }
            Panel::Commits => {
                // Show commit in diff view
                self.focused_panel = Panel::Diff;
            }
            Panel::Diff => {
                // Stage/unstage hunk (TODO)
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
                    self.repo.create_branch(&self.input_buffer, None)?;
                    self.refresh_branches()?;
                    self.message = Some(format!("Created branch: {}", self.input_buffer));
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
