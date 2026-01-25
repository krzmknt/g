use crate::config::{Config, DefaultBranchesMode, DefaultCommitsMode, DefaultDiffMode, Theme};
use crate::error::Result;
use crate::git::{IssueInfo, PullRequestInfo, ReleaseInfo, Repository, WorkflowRun};
use crate::input::{
    Event, EventReader, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crate::tui::{Buffer, Color, Rect, Style, Terminal};
use crate::views::{
    ActionsView, BlameView, BranchesView, CommitsView, CommitsViewMode, ConflictView, DiffMode,
    DiffView, FileTreeView, IssuesView, MenuView, PanelType, PullRequestsView, ReleasesView,
    RemotesView, Section, StashView, StatusView, SubmodulesView, TagsView, WorktreeView,
};
use crate::widgets::{Block, Borders, Widget};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::git::{
    BranchInfo, CommitInfo, ConflictEntry, FileTreeEntry, GraphLine, RemoteInfo, StatusEntry,
    SubmoduleInfo, TagInfo, WorktreeInfo,
};

/// Result from async fetch operations
pub enum AsyncLoadResult {
    // GitHub API results
    PullRequests(std::result::Result<Vec<PullRequestInfo>, String>),
    Issues(std::result::Result<Vec<IssueInfo>, String>),
    Actions(std::result::Result<Vec<WorkflowRun>, String>),
    Releases(std::result::Result<Vec<ReleaseInfo>, String>),
    /// PR commits for a specific PR (pr_number, commit_ids)
    PrCommits(u32, std::result::Result<Vec<String>, String>),
    // Git data results
    GitStatus(std::result::Result<Vec<StatusEntry>, String>),
    GitBranches(std::result::Result<Vec<BranchInfo>, String>),
    GitCommits(std::result::Result<Vec<CommitInfo>, String>),
    GitGraphCommits(std::result::Result<Vec<GraphLine>, String>),
    GitTags(std::result::Result<Vec<TagInfo>, String>),
    GitRemotes(std::result::Result<Vec<RemoteInfo>, String>),
    GitWorktrees(std::result::Result<Vec<WorktreeInfo>, String>),
    GitSubmodules(std::result::Result<Vec<SubmoduleInfo>, String>),
    GitConflicts(std::result::Result<Vec<ConflictEntry>, String>),
    GitFileTree(std::result::Result<Vec<FileTreeEntry>, String>),
    // Remote operation results (fetch/pull/push)
    RemoteOperationComplete(std::result::Result<String, String>),
}

// Re-export PanelType as Panel for backwards compatibility within app
pub use crate::views::PanelType as Panel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Visual, // Visual line selection mode in Preview pane
    Search,
    Command,
    Input(InputContext),
    Confirm(ConfirmAction),
    Select(SelectAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAction {
    ResetOrRevert,  // Choose between reset and revert
    ResetMode,      // Choose reset mode: --soft, --mixed, --hard
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
    RemoteBranchDelete,
    BranchPush,
    DeleteMergedBranches,
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
    pub pull_requests_view: PullRequestsView,
    pub issues_view: IssuesView,
    pub actions_view: ActionsView,
    pub releases_view: ReleasesView,
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

    // Merged branches to delete (local, remote)
    pub merged_branches_to_delete: Option<(Vec<String>, Vec<String>)>,

    // Selection dialog state (current selected index)
    pub select_index: usize,

    // Async loading channel for GitHub API calls
    async_sender: Sender<AsyncLoadResult>,
    async_receiver: Receiver<AsyncLoadResult>,

    // Repository path for spawning async tasks
    repo_path: PathBuf,

    // Last spinner tick time
    last_spinner_tick: Instant,

    // Last auto-refresh time (for periodic background refresh of all data)
    last_auto_refresh: Option<Instant>,

    // Flags to track if data is being refreshed in background
    refreshing_status: bool,
    refreshing_branches: bool,
    refreshing_commits: bool,
    refreshing_graph_commits: bool,
    refreshing_tags: bool,
    refreshing_remotes: bool,
    refreshing_worktrees: bool,
    refreshing_submodules: bool,
    refreshing_conflicts: bool,
    refreshing_filetree: bool,
    /// PR number currently being loaded for commits (None if not loading)
    refreshing_pr_commits: Option<u32>,
    /// Remote operation in progress (fetch/pull/push) - shows spinner in footer
    remote_operation: Option<RemoteOperation>,
    /// Spinner frame for remote operations (0-7)
    remote_spinner_frame: usize,
}

/// Remote operation type for spinner display
#[derive(Debug, Clone)]
pub enum RemoteOperation {
    Fetch(String),  // branch name
    Pull(String),   // branch name
    Push(String),   // branch name
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
    Intersection {
        col_idx: usize,
        left_panel_idx: usize,
        right_panel_idx: usize,
    },
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Repository::discover()?;
        let config = Config::load().unwrap_or_default();
        let terminal = Terminal::new()?;
        let event_reader = EventReader::new();

        // Create views with default modes from config
        let mut diff_view = DiffView::new();
        match config.view_defaults.diff_mode {
            DefaultDiffMode::Inline => diff_view.set_mode(DiffMode::Inline),
            DefaultDiffMode::Split => diff_view.set_mode(DiffMode::SideBySide),
        }

        let mut commits_view = CommitsView::new();
        match config.view_defaults.commits_mode {
            DefaultCommitsMode::Compact => commits_view.set_view_mode(CommitsViewMode::Compact),
            DefaultCommitsMode::Detailed => commits_view.set_view_mode(CommitsViewMode::Detailed),
            DefaultCommitsMode::Graph => commits_view.set_view_mode(CommitsViewMode::Graph),
        }

        let branches_view = BranchesView::new();

        // Create channel for async loading
        let (async_sender, async_receiver) = mpsc::channel();

        // Get repo path for spawning async tasks (working directory)
        let repo_path = repo.path().to_path_buf();

        Ok(Self {
            repo,
            config,
            terminal,
            event_reader,
            focused_panel: PanelType::Status,
            mode: Mode::Normal,
            view_mode: ViewMode::MultiPane,
            status_view: StatusView::new(),
            branches_view,
            commits_view,
            stash_view: StashView::new(),
            diff_view,
            tags_view: TagsView::new(),
            remotes_view: RemotesView::new(),
            worktree_view: WorktreeView::new(),
            submodules_view: SubmodulesView::new(),
            blame_view: BlameView::new(),
            filetree_view: FileTreeView::new(),
            conflict_view: ConflictView::new(),
            pull_requests_view: PullRequestsView::new(),
            issues_view: IssuesView::new(),
            actions_view: ActionsView::new(),
            releases_view: ReleasesView::new(),
            menu_view: MenuView::new(),
            input_buffer: String::new(),
            input_cursor: 0,
            message: None,
            should_quit: false,
            drag_state: None,
            branch_create_from: None,
            confirm_target: None,
            merged_branches_to_delete: None,
            select_index: 0,
            async_sender,
            async_receiver,
            repo_path,
            last_spinner_tick: Instant::now(),
            last_auto_refresh: None,
            refreshing_status: false,
            refreshing_branches: false,
            refreshing_commits: false,
            refreshing_graph_commits: false,
            refreshing_tags: false,
            refreshing_remotes: false,
            refreshing_worktrees: false,
            refreshing_submodules: false,
            refreshing_conflicts: false,
            refreshing_filetree: false,
            refreshing_pr_commits: None,
            remote_operation: None,
            remote_spinner_frame: 0,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.terminal.init()?;
        self.refresh_all()?;

        let mut frame_count = 0u64;
        loop {
            debug!(
                "FRAME {}: focused_panel={:?}, drag_state={}",
                frame_count,
                self.focused_panel,
                self.drag_state.is_some()
            );
            frame_count += 1;

            // Process any completed async load results
            self.process_async_results();

            // Check for timeouts and tick spinners
            self.update_loading_states();

            // Periodic background refresh for all data
            self.check_auto_refresh();

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

    /// Process any completed async load results from background threads
    fn process_async_results(&mut self) {
        // Non-blocking receive of all pending results
        while let Ok(result) = self.async_receiver.try_recv() {
            match result {
                AsyncLoadResult::PullRequests(Ok(prs)) => {
                    self.pull_requests_view.set_loaded(prs);
                }
                AsyncLoadResult::PullRequests(Err(e)) => {
                    self.pull_requests_view.set_error(e);
                }
                AsyncLoadResult::Issues(Ok(issues)) => {
                    self.issues_view.set_loaded(issues);
                }
                AsyncLoadResult::Issues(Err(e)) => {
                    self.issues_view.set_error(e);
                }
                AsyncLoadResult::Actions(Ok(runs)) => {
                    self.actions_view.set_loaded(runs);
                }
                AsyncLoadResult::Actions(Err(e)) => {
                    self.actions_view.set_error(e);
                }
                AsyncLoadResult::Releases(Ok(releases)) => {
                    self.releases_view.set_loaded(releases);
                }
                AsyncLoadResult::Releases(Err(e)) => {
                    self.releases_view.set_error(e);
                }
                AsyncLoadResult::PrCommits(pr_number, Ok(commit_ids)) => {
                    // Only apply if we're still waiting for this PR's commits
                    if self.refreshing_pr_commits == Some(pr_number) {
                        debug!("Async fetch_pr_commits: got {} commits for PR #{}", commit_ids.len(), pr_number);
                        self.commits_view.set_highlight_commits(commit_ids);
                        self.refreshing_pr_commits = None;
                    }
                }
                AsyncLoadResult::PrCommits(pr_number, Err(e)) => {
                    if self.refreshing_pr_commits == Some(pr_number) {
                        debug!("Async fetch_pr_commits error for PR #{}: {}", pr_number, e);
                        self.refreshing_pr_commits = None;
                    }
                }
                AsyncLoadResult::GitStatus(Ok(status)) => {
                    self.status_view.update_preserve_scroll(status);
                    self.refreshing_status = false;
                }
                AsyncLoadResult::GitStatus(Err(_)) => {
                    self.refreshing_status = false;
                }
                AsyncLoadResult::GitBranches(Ok(branches)) => {
                    self.branches_view.update_preserve_scroll(branches);
                    self.refreshing_branches = false;
                }
                AsyncLoadResult::GitBranches(Err(_)) => {
                    self.refreshing_branches = false;
                }
                AsyncLoadResult::GitCommits(Ok(commits)) => {
                    self.commits_view.update_preserve_scroll(commits);
                    self.refreshing_commits = false;
                }
                AsyncLoadResult::GitCommits(Err(_)) => {
                    self.refreshing_commits = false;
                }
                AsyncLoadResult::GitGraphCommits(Ok(commits)) => {
                    self.commits_view.update_graph(commits);
                    self.refreshing_graph_commits = false;
                }
                AsyncLoadResult::GitGraphCommits(Err(_)) => {
                    self.refreshing_graph_commits = false;
                }
                AsyncLoadResult::GitTags(Ok(tags)) => {
                    self.tags_view.update(tags);
                    self.refreshing_tags = false;
                }
                AsyncLoadResult::GitTags(Err(_)) => {
                    self.refreshing_tags = false;
                }
                AsyncLoadResult::GitRemotes(Ok(remotes)) => {
                    self.remotes_view.update(remotes);
                    self.refreshing_remotes = false;
                }
                AsyncLoadResult::GitRemotes(Err(_)) => {
                    self.refreshing_remotes = false;
                }
                AsyncLoadResult::GitWorktrees(Ok(worktrees)) => {
                    self.worktree_view.update(worktrees);
                    self.refreshing_worktrees = false;
                }
                AsyncLoadResult::GitWorktrees(Err(_)) => {
                    self.refreshing_worktrees = false;
                }
                AsyncLoadResult::GitSubmodules(Ok(submodules)) => {
                    self.submodules_view.update(submodules);
                    self.refreshing_submodules = false;
                }
                AsyncLoadResult::GitSubmodules(Err(_)) => {
                    self.refreshing_submodules = false;
                }
                AsyncLoadResult::GitConflicts(Ok(conflicts)) => {
                    self.conflict_view.update(conflicts);
                    self.refreshing_conflicts = false;
                }
                AsyncLoadResult::GitConflicts(Err(_)) => {
                    self.refreshing_conflicts = false;
                }
                AsyncLoadResult::GitFileTree(Ok(tree)) => {
                    self.filetree_view.update(tree);
                    self.refreshing_filetree = false;
                }
                AsyncLoadResult::GitFileTree(Err(_)) => {
                    self.refreshing_filetree = false;
                }
                AsyncLoadResult::RemoteOperationComplete(Ok(msg)) => {
                    self.remote_operation = None;
                    self.message = Some(msg);
                    // Refresh all data after remote operation
                    let _ = self.refresh_all();
                }
                AsyncLoadResult::RemoteOperationComplete(Err(e)) => {
                    self.remote_operation = None;
                    self.message = Some(format!("Error: {}", e));
                }
            }
        }
    }

    /// Update loading states: check timeouts and tick spinners
    fn update_loading_states(&mut self) {
        // Check timeouts
        self.pull_requests_view.check_timeout();
        self.issues_view.check_timeout();
        self.actions_view.check_timeout();
        self.releases_view.check_timeout();

        // Tick spinners every 100ms
        if self.last_spinner_tick.elapsed() >= Duration::from_millis(100) {
            self.last_spinner_tick = Instant::now();
            if self.pull_requests_view.is_loading() {
                self.pull_requests_view.tick_spinner();
            }
            if self.issues_view.is_loading() {
                self.issues_view.tick_spinner();
            }
            if self.actions_view.is_loading() {
                self.actions_view.tick_spinner();
            }
            if self.releases_view.is_loading() {
                self.releases_view.tick_spinner();
            }
            // Tick remote operation spinner
            if self.remote_operation.is_some() {
                self.remote_spinner_frame = (self.remote_spinner_frame + 1) % 8;
            }
        }
    }

    /// Check if it's time to do a periodic background refresh for all data
    fn check_auto_refresh(&mut self) {
        let interval = self.config.auto_refresh;
        if interval == 0 {
            return; // Disabled
        }

        let should_refresh = match self.last_auto_refresh {
            None => {
                // First time, set the timestamp
                self.last_auto_refresh = Some(Instant::now());
                false
            }
            Some(last) => last.elapsed() >= Duration::from_secs(interval as u64),
        };

        if should_refresh {
            self.last_auto_refresh = Some(Instant::now());
            self.start_background_refresh_all();
        }
    }

    /// Start async loading of pull requests
    fn start_loading_pull_requests(&mut self) {
        self.start_loading_pull_requests_impl(false);
    }

    fn start_loading_pull_requests_impl(&mut self, background: bool) {
        if self.pull_requests_view.is_refreshing() {
            return;
        }
        if background {
            self.pull_requests_view.start_background_refresh();
        } else {
            self.pull_requests_view.start_loading();
        }
        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();
        thread::spawn(move || {
            let result = fetch_pull_requests(&repo_path);
            let _ = sender.send(AsyncLoadResult::PullRequests(result));
        });
    }

    /// Start async loading of issues
    fn start_loading_issues(&mut self) {
        self.start_loading_issues_impl(false);
    }

    fn start_loading_issues_impl(&mut self, background: bool) {
        if self.issues_view.is_refreshing() {
            return;
        }
        if background {
            self.issues_view.start_background_refresh();
        } else {
            self.issues_view.start_loading();
        }
        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();
        thread::spawn(move || {
            let result = fetch_issues(&repo_path);
            let _ = sender.send(AsyncLoadResult::Issues(result));
        });
    }

    /// Start async loading of actions/workflow runs
    fn start_loading_actions(&mut self) {
        self.start_loading_actions_impl(false);
    }

    fn start_loading_actions_impl(&mut self, background: bool) {
        if self.actions_view.is_refreshing() {
            return;
        }
        if background {
            self.actions_view.start_background_refresh();
        } else {
            self.actions_view.start_loading();
        }
        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();
        thread::spawn(move || {
            let result = fetch_workflow_runs(&repo_path);
            let _ = sender.send(AsyncLoadResult::Actions(result));
        });
    }

    /// Start async loading of releases
    fn start_loading_releases(&mut self) {
        self.start_loading_releases_impl(false);
    }

    fn start_loading_releases_impl(&mut self, background: bool) {
        if self.releases_view.is_refreshing() {
            return;
        }
        if background {
            self.releases_view.start_background_refresh();
        } else {
            self.releases_view.start_loading();
        }
        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();
        thread::spawn(move || {
            let result = fetch_releases(&repo_path);
            let _ = sender.send(AsyncLoadResult::Releases(result));
        });
    }

    /// Start background refresh for all data (no spinner, keeps showing old data)
    fn start_background_refresh_all(&mut self) {
        // Refresh GitHub views
        self.start_loading_pull_requests_impl(true);
        self.start_loading_issues_impl(true);
        self.start_loading_actions_impl(true);
        self.start_loading_releases_impl(true);

        // Refresh git status
        if !self.refreshing_status {
            self.refreshing_status = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.status().map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitStatus(result));
            });
        }

        // Refresh branches
        if !self.refreshing_branches {
            self.refreshing_branches = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            let show_remote = self.branches_view.show_remote;
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.branches(show_remote).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitBranches(result));
            });
        }

        // Refresh commits
        if !self.refreshing_commits {
            self.refreshing_commits = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            let max_commits = self.config.max_commits;
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.commits(max_commits).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitCommits(result));
            });
        }

        // Refresh graph commits
        if !self.refreshing_graph_commits {
            self.refreshing_graph_commits = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            let max_commits = self.config.max_commits;
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.log_graph(max_commits).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitGraphCommits(result));
            });
        }

        // Refresh tags
        if !self.refreshing_tags {
            self.refreshing_tags = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.tags().map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitTags(result));
            });
        }

        // Refresh remotes
        if !self.refreshing_remotes {
            self.refreshing_remotes = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.remote_info().map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitRemotes(result));
            });
        }

        // Refresh worktrees
        if !self.refreshing_worktrees {
            self.refreshing_worktrees = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.worktrees().map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitWorktrees(result));
            });
        }

        // Refresh submodules
        if !self.refreshing_submodules {
            self.refreshing_submodules = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.submodules().map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitSubmodules(result));
            });
        }

        // Refresh conflicts
        if !self.refreshing_conflicts {
            self.refreshing_conflicts = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.conflicts().map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitConflicts(result));
            });
        }

        // Refresh file tree
        if !self.refreshing_filetree {
            self.refreshing_filetree = true;
            let sender = self.async_sender.clone();
            let repo_path = self.repo_path.clone();
            let show_ignored = self.filetree_view.show_ignored;
            thread::spawn(move || {
                let result = match Repository::open(&repo_path) {
                    Ok(repo) => repo.file_tree(show_ignored).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = sender.send(AsyncLoadResult::GitFileTree(result));
            });
        }
    }

    fn refresh_all(&mut self) -> Result<()> {
        self.refresh_status()?;
        self.refresh_branches()?;
        self.refresh_commits()?;
        self.refresh_stash()?;
        self.refresh_diff()?;
        self.refresh_tags()?;
        self.refresh_remotes()?;
        // Start async loading for GitHub API views
        self.start_loading_pull_requests();
        self.start_loading_issues();
        self.start_loading_actions();
        self.start_loading_releases();
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
        // Set current branch for proper coloring
        self.commits_view
            .set_current_branch(self.repo.head_name().ok().flatten());
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

    fn refresh_file_preview(&mut self) -> Result<()> {
        if let Some(entry) = self.filetree_view.selected_entry() {
            if !entry.is_dir {
                let path = entry.path.clone();
                match self.repo.read_file_content(&path) {
                    Ok(content) => {
                        self.diff_view.set_file_content(path, content);
                    }
                    Err(_) => {
                        self.diff_view.clear();
                    }
                }
            } else {
                self.diff_view.clear();
            }
        } else {
            self.diff_view.clear();
        }
        // Force full redraw to ensure preview pane is properly updated
        self.terminal.force_full_redraw();
        Ok(())
    }

    fn refresh_pr_preview(&mut self) {
        if let Some(pr) = self.pull_requests_view.selected_pr() {
            debug!("refresh_pr_preview: PR #{} branch={}", pr.number, pr.head_ref_name);
            self.diff_view.set_pr_preview(pr);
            // Highlight related commits by branch name
            let branch = pr.head_ref_name.clone();
            self.commits_view.set_highlight_branch(Some(branch.clone()));
            self.actions_view.set_highlight_branch(Some(branch));

            // Fetch commits for this specific PR asynchronously
            let pr_number = pr.number;
            self.start_async_pr_commits_load(pr_number);
        } else {
            self.diff_view.clear_pr_preview();
            self.commits_view.clear_highlight_commits();
            self.commits_view.set_highlight_branch(None);
            self.actions_view.set_highlight_branch(None);
            self.refreshing_pr_commits = None;
        }
        self.terminal.force_full_redraw();
    }

    fn refresh_commit_preview(&mut self) {
        if let Some(commit) = self.commits_view.selected_commit() {
            self.diff_view.set_commit_preview(commit);
        } else {
            self.diff_view.clear_commit_preview();
        }
    }

    /// Start async loading of PR commits
    fn start_async_pr_commits_load(&mut self, pr_number: u32) {
        // Clear previous highlights while loading
        self.commits_view.clear_highlight_commits();
        self.refreshing_pr_commits = Some(pr_number);

        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();

        thread::spawn(move || {
            let result = fetch_pr_commits(&repo_path, pr_number);
            let _ = sender.send(AsyncLoadResult::PrCommits(pr_number, result));
        });
    }

    fn clear_pr_highlights(&mut self) {
        self.diff_view.clear_pr_preview();
        self.commits_view.clear_highlight_commits();
        self.commits_view.set_highlight_branch(None);
        self.actions_view.set_highlight_branch(None);
    }

    /// Start async fetch operation with spinner
    fn start_async_fetch(&mut self, remote: String, branch: String) {
        if self.remote_operation.is_some() {
            self.message = Some("Remote operation already in progress".to_string());
            return;
        }
        self.remote_operation = Some(RemoteOperation::Fetch(branch.clone()));
        self.remote_spinner_frame = 0;

        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();

        thread::spawn(move || {
            let result = crate::git::Repository::open(&repo_path)
                .and_then(|repo| repo.fetch_branch(&remote, &branch))
                .map(|()| format!("Fetched {}/{}", remote, branch))
                .map_err(|e| e.to_string());
            let _ = sender.send(AsyncLoadResult::RemoteOperationComplete(result));
        });
    }

    /// Start async pull operation with spinner
    fn start_async_pull(&mut self, remote: String, branch: String) {
        if self.remote_operation.is_some() {
            self.message = Some("Remote operation already in progress".to_string());
            return;
        }
        self.remote_operation = Some(RemoteOperation::Pull(branch.clone()));
        self.remote_spinner_frame = 0;

        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();

        thread::spawn(move || {
            let result = crate::git::Repository::open(&repo_path)
                .and_then(|repo| repo.pull_branch(&remote, &branch))
                .map(|merge_result| {
                    match merge_result {
                        crate::git::MergeResult::UpToDate => format!("Branch {} is up to date", branch),
                        crate::git::MergeResult::FastForward => format!("Fast-forwarded branch: {}", branch),
                        crate::git::MergeResult::Merged => format!("Merged branch: {}", branch),
                        crate::git::MergeResult::Conflict => format!("Merge conflicts in branch: {}", branch),
                    }
                })
                .map_err(|e| e.to_string());
            let _ = sender.send(AsyncLoadResult::RemoteOperationComplete(result));
        });
    }

    /// Start async push operation with spinner
    fn start_async_push(&mut self, remote: String, branch: Option<String>) {
        if self.remote_operation.is_some() {
            self.message = Some("Remote operation already in progress".to_string());
            return;
        }
        let display_name = branch.clone().unwrap_or_else(|| "HEAD".to_string());
        self.remote_operation = Some(RemoteOperation::Push(display_name.clone()));
        self.remote_spinner_frame = 0;

        let sender = self.async_sender.clone();
        let repo_path = self.repo_path.clone();

        thread::spawn(move || {
            let result = crate::git::Repository::open(&repo_path)
                .and_then(|repo| {
                    if let Some(ref b) = branch {
                        repo.push_branch(&remote, b)
                    } else {
                        repo.push(&remote)
                    }
                })
                .map(|()| {
                    if let Some(b) = branch {
                        format!("Pushed {} to {}", b, remote)
                    } else {
                        format!("Pushed to {}", remote)
                    }
                })
                .map_err(|e| e.to_string());
            let _ = sender.send(AsyncLoadResult::RemoteOperationComplete(result));
        });
    }

    fn draw(&mut self) -> Result<()> {
        let theme = self.config.current_theme().clone();
        let focused_panel = self.focused_panel;
        let mode = self.mode;
        let view_mode = self.view_mode;
        let message = self.message.clone();
        let input_buffer = self.input_buffer.clone();
        let branch_create_from = self.branch_create_from.clone();

        self.terminal.draw(|buf| {
            let area = buf.area;

            // Layout (no header, just main + footer)
            let (main, footer) = area.split_horizontal(area.height.saturating_sub(3));

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
                                PanelType::Status => Self::render_status_panel(
                                    buf,
                                    panel_area,
                                    &theme,
                                    is_focused,
                                    &mut self.status_view,
                                ),
                                PanelType::Branches => Self::render_branches_panel(
                                    buf,
                                    panel_area,
                                    &theme,
                                    is_focused,
                                    &mut self.branches_view,
                                ),
                                PanelType::Commits => Self::render_commits_panel(
                                    buf,
                                    panel_area,
                                    &theme,
                                    is_focused,
                                    &mut self.commits_view,
                                ),
                                PanelType::Stash => Self::render_stash_panel(
                                    buf,
                                    panel_area,
                                    &theme,
                                    is_focused,
                                    &mut self.stash_view,
                                ),
                                PanelType::Diff => Self::render_diff_panel(
                                    buf,
                                    panel_area,
                                    &theme,
                                    is_focused,
                                    &mut self.diff_view,
                                ),
                                PanelType::Tags => {
                                    self.tags_view.render(panel_area, buf, &theme, is_focused)
                                }
                                PanelType::Remotes => self
                                    .remotes_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::Worktrees => self
                                    .worktree_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::Submodules => self
                                    .submodules_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::Blame => {
                                    self.blame_view.render(panel_area, buf, &theme, is_focused)
                                }
                                PanelType::Files => self
                                    .filetree_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::Conflicts => self
                                    .conflict_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::PullRequests => self
                                    .pull_requests_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::Issues => {
                                    self.issues_view.render(panel_area, buf, &theme, is_focused)
                                }
                                PanelType::Actions => self
                                    .actions_view
                                    .render(panel_area, buf, &theme, is_focused),
                                PanelType::Releases => self
                                    .releases_view
                                    .render(panel_area, buf, &theme, is_focused),
                            }

                            panel_y += panel_h;
                        }

                        col_x += col_width;
                    }
                }
                ViewMode::SinglePane => {
                    // Full screen single panel
                    match focused_panel {
                        PanelType::Status => Self::render_status_panel(
                            buf,
                            main,
                            &theme,
                            true,
                            &mut self.status_view,
                        ),
                        PanelType::Branches => Self::render_branches_panel(
                            buf,
                            main,
                            &theme,
                            true,
                            &mut self.branches_view,
                        ),
                        PanelType::Commits => Self::render_commits_panel(
                            buf,
                            main,
                            &theme,
                            true,
                            &mut self.commits_view,
                        ),
                        PanelType::Stash => {
                            Self::render_stash_panel(buf, main, &theme, true, &mut self.stash_view)
                        }
                        PanelType::Diff => {
                            Self::render_diff_panel(buf, main, &theme, true, &mut self.diff_view)
                        }
                        PanelType::Tags => self.tags_view.render(main, buf, &theme, true),
                        PanelType::Remotes => self.remotes_view.render(main, buf, &theme, true),
                        PanelType::Worktrees => self.worktree_view.render(main, buf, &theme, true),
                        PanelType::Submodules => {
                            self.submodules_view.render(main, buf, &theme, true)
                        }
                        PanelType::Blame => self.blame_view.render(main, buf, &theme, true),
                        PanelType::Files => self.filetree_view.render(main, buf, &theme, true),
                        PanelType::Conflicts => self.conflict_view.render(main, buf, &theme, true),
                        PanelType::PullRequests => {
                            self.pull_requests_view.render(main, buf, &theme, true)
                        }
                        PanelType::Issues => self.issues_view.render(main, buf, &theme, true),
                        PanelType::Actions => self.actions_view.render(main, buf, &theme, true),
                        PanelType::Releases => self.releases_view.render(main, buf, &theme, true),
                    }
                }
            }

            // Footer - pass scroll state for focused panel
            let (can_scroll_left, can_scroll_right) = match focused_panel {
                PanelType::Files => (
                    self.filetree_view.can_scroll_left(),
                    self.filetree_view.can_scroll_right(),
                ),
                PanelType::Commits => (
                    self.commits_view.can_scroll_left(),
                    self.commits_view.can_scroll_right(),
                ),
                PanelType::Branches => (
                    self.branches_view.can_scroll_left(),
                    self.branches_view.can_scroll_right(),
                ),
                PanelType::Stash => (
                    self.stash_view.can_scroll_left(),
                    self.stash_view.can_scroll_right(),
                ),
                PanelType::Tags => (
                    self.tags_view.can_scroll_left(),
                    self.tags_view.can_scroll_right(),
                ),
                PanelType::Remotes => (
                    self.remotes_view.can_scroll_left(),
                    self.remotes_view.can_scroll_right(),
                ),
                PanelType::Worktrees => (
                    self.worktree_view.can_scroll_left(),
                    self.worktree_view.can_scroll_right(),
                ),
                PanelType::Submodules => (
                    self.submodules_view.can_scroll_left(),
                    self.submodules_view.can_scroll_right(),
                ),
                PanelType::Blame => (
                    self.blame_view.can_scroll_left(),
                    self.blame_view.can_scroll_right(),
                ),
                PanelType::Conflicts => (
                    self.conflict_view.can_scroll_left(),
                    self.conflict_view.can_scroll_right(),
                ),
                PanelType::Status => (
                    self.status_view.can_scroll_left(),
                    self.status_view.can_scroll_right(),
                ),
                PanelType::Diff => (
                    self.diff_view.can_scroll_left(),
                    self.diff_view.can_scroll_right(),
                ),
                PanelType::PullRequests => (
                    self.pull_requests_view.can_scroll_left(),
                    self.pull_requests_view.can_scroll_right(),
                ),
                PanelType::Issues => (
                    self.issues_view.can_scroll_left(),
                    self.issues_view.can_scroll_right(),
                ),
                PanelType::Actions => (
                    self.actions_view.can_scroll_left(),
                    self.actions_view.can_scroll_right(),
                ),
                PanelType::Releases => (
                    self.releases_view.can_scroll_left(),
                    self.releases_view.can_scroll_right(),
                ),
            };
            Self::render_footer(
                buf,
                footer,
                &theme,
                mode,
                view_mode,
                message.as_deref(),
                &input_buffer,
                focused_panel,
                branch_create_from.as_deref(),
                self.confirm_target.as_deref(),
                can_scroll_left,
                can_scroll_right,
                self.select_index,
            );

            // Remote operation spinner in footer (right-aligned, above logo)
            if let Some(ref op) = self.remote_operation {
                const SPINNER_CHARS: [char; 8] = ['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
                let spinner = SPINNER_CHARS[self.remote_spinner_frame];
                let op_text = match op {
                    RemoteOperation::Fetch(branch) => format!("{} Fetching {}...", spinner, branch),
                    RemoteOperation::Pull(branch) => format!("{} Pulling {}...", spinner, branch),
                    RemoteOperation::Push(branch) => format!("{} Pushing {}...", spinner, branch),
                };
                let spinner_x = footer.x + 1;
                let spinner_y = footer.y;  // Top line of footer (message line)
                buf.set_string(
                    spinner_x,
                    spinner_y,
                    &op_text,
                    Style::new().fg(theme.branch_local).bold(),
                );
            }

            // Logo at bottom right
            let logo = "g v0.1.0";
            let logo_x = buf.area.width.saturating_sub(logo.len() as u16 + 1);
            let logo_y = buf.area.height.saturating_sub(1);
            let logo_style = Style::new().fg(Color::Rgb(100, 100, 100)); // Dim gray
            buf.set_string(logo_x, logo_y, logo, logo_style);
        })?;

        Ok(())
    }

    fn render_status_panel(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        focused: bool,
        view: &mut StatusView,
    ) {
        view.render(area, buf, theme, focused);
    }

    fn render_branches_panel(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        focused: bool,
        view: &mut BranchesView,
    ) {
        view.render(area, buf, theme, focused);
    }

    fn render_commits_panel(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        focused: bool,
        view: &mut CommitsView,
    ) {
        view.render(area, buf, theme, focused);
    }

    fn render_stash_panel(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        focused: bool,
        view: &mut StashView,
    ) {
        view.render(area, buf, theme, focused);
    }

    fn render_diff_panel(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        focused: bool,
        view: &mut DiffView,
    ) {
        view.render(area, buf, theme, focused);
    }

    fn render_footer(
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        mode: Mode,
        view_mode: ViewMode,
        message: Option<&str>,
        input: &str,
        focused_panel: Panel,
        branch_create_from: Option<&str>,
        confirm_target: Option<&str>,
        can_scroll_left: bool,
        can_scroll_right: bool,
        select_index: usize,
    ) {
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
                    Self::render_zoom_tab_bar(
                        buf,
                        area.x,
                        help_y1,
                        area.width,
                        focused_panel,
                        theme,
                    );
                } else {
                    // Build global commands with dynamic h/l scroll indicator
                    let scroll_cmd: Option<(&str, &str)> = match (can_scroll_left, can_scroll_right)
                    {
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

                    Self::render_command_line(
                        buf,
                        area.x + 1,
                        help_y1,
                        &global_cmds_vec,
                        key_style,
                        desc_style,
                        sep_style,
                        area.width.saturating_sub(2),
                    );
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
                        ("f", "fetch"),
                        ("p", "pull"),
                        ("P", "push"),
                        ("d", "delete"),
                        ("D", "force delete"),
                        ("M", "delete merged"),
                        ("R", "reset/revert"),
                    ],
                    PanelType::Commits => &[
                        ("Enter", "view diff"),
                        ("c", "checkout"),
                        ("R", "reset/revert"),
                        ("v", "view mode"),
                    ],
                    PanelType::Stash => &[("Enter", "pop"), ("a", "apply"), ("d", "drop")],
                    PanelType::Diff => &[("j/k", "scroll"), ("v", "toggle inline/split")],
                    PanelType::Tags => &[("n", "new tag"), ("d", "delete"), ("R", "reset/revert")],
                    PanelType::Remotes => &[("f", "fetch")],
                    PanelType::Worktrees => &[],
                    PanelType::Submodules => &[("u", "update")],
                    PanelType::Blame => &[("j/k", "scroll")],
                    PanelType::Files => &[
                        ("Space/Enter", "open"),
                        ("v", "show ignored"),
                        ("b", "blame"),
                    ],
                    PanelType::Conflicts => &[("o", "use ours"), ("t", "use theirs")],
                    PanelType::PullRequests => &[],
                    PanelType::Issues => &[],
                    PanelType::Actions => &[],
                    PanelType::Releases => &[],
                };
                Self::render_command_line(
                    buf,
                    area.x + 1,
                    help_y2,
                    panel_cmds,
                    key_style,
                    desc_style,
                    sep_style,
                    area.width.saturating_sub(2),
                );
            }
            Mode::Search | Mode::Input(_) => {
                let prompt: String = match mode {
                    Mode::Search => "/".to_string(),
                    Mode::Input(InputContext::CommitMessage) => "Commit: ".to_string(),
                    Mode::Input(InputContext::BranchName) => match branch_create_from {
                        Some(from) => format!("New branch from '{}': ", from),
                        None => "Branch: ".to_string(),
                    },
                    Mode::Input(InputContext::SearchQuery) => "Search: ".to_string(),
                    Mode::Input(InputContext::TagName) => "Tag: ".to_string(),
                    Mode::Input(InputContext::StashMessage) => "Stash: ".to_string(),
                    _ => "> ".to_string(),
                };
                let line = format!("{}{}", prompt, input);
                buf.set_string(
                    area.x + 1,
                    area.y + 1,
                    &line,
                    Style::new().fg(theme.foreground),
                );

                // Show input help
                let input_help = [("Enter", "confirm"), ("Esc", "cancel")];
                Self::render_command_line(
                    buf,
                    area.x + 1,
                    area.y + 2,
                    &input_help,
                    key_style,
                    desc_style,
                    sep_style,
                    area.width.saturating_sub(2),
                );
            }
            Mode::Command => {
                let line = format!(":{}", input);
                buf.set_string(
                    area.x + 1,
                    area.y + 1,
                    &line,
                    Style::new().fg(theme.foreground),
                );
            }
            Mode::Confirm(action) => {
                let action_desc = match action {
                    ConfirmAction::BranchDelete => format!(
                        "Delete branch '{}'?",
                        confirm_target.as_deref().unwrap_or("?")
                    ),
                    ConfirmAction::BranchForceDelete => format!(
                        "Force delete branch '{}'?",
                        confirm_target.as_deref().unwrap_or("?")
                    ),
                    ConfirmAction::RemoteBranchDelete => format!(
                        "Delete remote branch '{}'?",
                        confirm_target.as_deref().unwrap_or("?")
                    ),
                    ConfirmAction::Discard => format!(
                        "Discard changes in '{}'?",
                        confirm_target.as_deref().unwrap_or("?")
                    ),
                    ConfirmAction::Push => "Push to remote?".to_string(),
                    ConfirmAction::BranchPush => format!(
                        "Push branch '{}' to remote?",
                        confirm_target.as_deref().unwrap_or("?")
                    ),
                    ConfirmAction::StashDrop => format!(
                        "Drop stash@{{{}}}?",
                        confirm_target.as_deref().unwrap_or("?")
                    ),
                    ConfirmAction::CommitRevert => format!(
                        "Revert commit {}?",
                        confirm_target
                            .as_deref()
                            .map(|s| &s[..7.min(s.len())])
                            .unwrap_or("?")
                    ),
                    ConfirmAction::DeleteMergedBranches => format!(
                        "Delete merged branches? ({})",
                        confirm_target.as_deref().unwrap_or("0 local, 0 remote")
                    ),
                };
                let warn_style = Style::new().fg(theme.diff_remove).bold();
                buf.set_string(area.x + 1, area.y + 1, &action_desc, warn_style);

                let confirm_help = [("y", "yes"), ("n/Esc", "cancel")];
                Self::render_command_line(
                    buf,
                    area.x + 1,
                    area.y + 2,
                    &confirm_help,
                    key_style,
                    desc_style,
                    sep_style,
                    area.width.saturating_sub(2),
                );
            }
            Mode::Visual => {
                // Visual mode help line
                let visual_style = Style::new().fg(theme.branch_current).bold();
                buf.set_string(area.x + 1, area.y + 1, "-- VISUAL LINE --", visual_style);

                let visual_help = [
                    ("j/k", "select lines"),
                    ("g/G", "top/bottom"),
                    ("y", "yank"),
                    ("Esc", "exit"),
                ];
                Self::render_command_line(
                    buf,
                    area.x + 1,
                    area.y + 2,
                    &visual_help,
                    key_style,
                    desc_style,
                    sep_style,
                    area.width.saturating_sub(2),
                );
            }
            Mode::Select(action) => {
                let commit_id_short = confirm_target
                    .as_deref()
                    .map(|s| &s[..7.min(s.len())])
                    .unwrap_or("?");

                let (title, options): (&str, Vec<(&str, &str, &str)>) = match action {
                    SelectAction::ResetOrRevert => (
                        &format!("Reset/Revert commit {}", commit_id_short),
                        vec![
                            ("1", "reset", "Move HEAD backward, discard commits"),
                            ("2", "revert", "Create a new commit that reverses changes"),
                        ],
                    ),
                    SelectAction::ResetMode => (
                        &format!("Reset mode for {}", commit_id_short),
                        vec![
                            ("1", "soft", "repo:reset, index:keep, working:keep"),
                            ("2", "mixed", "repo:reset, index:reset, working:keep"),
                            ("3", "hard", "repo:reset, index:reset, working:reset"),
                        ],
                    ),
                };

                // Render title
                let title_style = Style::new().fg(theme.branch_current).bold();
                buf.set_string(area.x + 1, area.y + 1, title, title_style);

                // Render options horizontally on a single line
                let mut x_pos = area.x + 1;
                for (i, (key, label, _)) in options.iter().enumerate() {
                    let is_selected = i == select_index;
                    let prefix = if is_selected { ">" } else { " " };
                    let option_text = format!("{}{}:{} ", prefix, key, label);

                    let style = if is_selected {
                        Style::new().fg(theme.selection_text).bg(theme.selection)
                    } else {
                        Style::new().fg(theme.foreground)
                    };

                    buf.set_string(x_pos, area.y + 2, &option_text, style);
                    x_pos += option_text.len() as u16 + 1;
                }

                // Show description of selected option
                let selected_desc = options.get(select_index).map(|(_, _, d)| *d).unwrap_or("");
                let desc_style = Style::new().fg(theme.untracked);
                buf.set_string(x_pos + 1, area.y + 2, &format!("- {}", selected_desc), desc_style);
            }
        }
    }

    fn render_command_line(
        buf: &mut Buffer,
        x: u16,
        y: u16,
        cmds: &[(&str, &str)],
        key_style: Style,
        desc_style: Style,
        sep_style: Style,
        max_width: u16,
    ) {
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

    fn render_zoom_tab_bar(
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        focused: PanelType,
        theme: &Theme,
    ) {
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
            Mode::Visual => self.handle_visual_key(key),
            Mode::Search => self.handle_search_key(key),
            Mode::Command => self.handle_command_key(key),
            Mode::Input(ctx) => self.handle_input_key(key, ctx),
            Mode::Confirm(action) => self.handle_confirm_key(key, action),
            Mode::Select(action) => self.handle_select_key(key, action),
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent, action: ConfirmAction) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Execute the confirmed action
                self.execute_confirmed_action(action)?;
                self.mode = Mode::Normal;
                self.confirm_target = None;
                self.merged_branches_to_delete = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Escape => {
                self.message = Some("Cancelled".to_string());
                self.mode = Mode::Normal;
                self.confirm_target = None;
                self.merged_branches_to_delete = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_select_key(&mut self, key: KeyEvent, action: SelectAction) -> Result<()> {
        let option_count = match action {
            SelectAction::ResetOrRevert => 2,
            SelectAction::ResetMode => 3,
        };

        match key.code {
            // Navigation (j/l = next, k/h = prev)
            KeyCode::Char('j') | KeyCode::Char('l') | KeyCode::Down | KeyCode::Right => {
                if self.select_index + 1 < option_count {
                    self.select_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Char('h') | KeyCode::Up | KeyCode::Left => {
                if self.select_index > 0 {
                    self.select_index -= 1;
                }
            }
            // Direct selection by number
            KeyCode::Char('1') => {
                self.select_index = 0;
                self.execute_select_action(action)?;
            }
            KeyCode::Char('2') => {
                if option_count >= 2 {
                    self.select_index = 1;
                    self.execute_select_action(action)?;
                }
            }
            KeyCode::Char('3') => {
                if option_count >= 3 {
                    self.select_index = 2;
                    self.execute_select_action(action)?;
                }
            }
            // Enter confirms current selection
            KeyCode::Enter => {
                self.execute_select_action(action)?;
            }
            // Cancel
            KeyCode::Escape => {
                self.message = Some("Cancelled".to_string());
                self.mode = Mode::Normal;
                self.confirm_target = None;
                self.select_index = 0;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_select_action(&mut self, action: SelectAction) -> Result<()> {
        match action {
            SelectAction::ResetOrRevert => {
                match self.select_index {
                    0 => {
                        // Reset - go to reset mode selection
                        self.select_index = 1; // Default to --mixed
                        self.mode = Mode::Select(SelectAction::ResetMode);
                    }
                    1 => {
                        // Revert
                        self.mode = Mode::Confirm(ConfirmAction::CommitRevert);
                    }
                    _ => {}
                }
            }
            SelectAction::ResetMode => {
                if let Some(ref commit_id) = self.confirm_target {
                    let reset_type = match self.select_index {
                        0 => "soft",
                        1 => "mixed",
                        2 => "hard",
                        _ => "mixed",
                    };
                    match self.repo.reset_to_commit(commit_id, reset_type) {
                        Ok(()) => {
                            self.message = Some(format!(
                                "Reset --{} to {}",
                                reset_type,
                                &commit_id[..7.min(commit_id.len())]
                            ));
                            self.refresh_all()?;
                        }
                        Err(e) => {
                            self.message = Some(format!("Reset failed: {}", e));
                        }
                    }
                }
                self.mode = Mode::Normal;
                self.confirm_target = None;
                self.select_index = 0;
            }
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
            ConfirmAction::RemoteBranchDelete => {
                if let Some(ref name) = self.confirm_target {
                    // Parse remote/branch from name like "origin/feature"
                    if let Some(slash_pos) = name.find('/') {
                        let remote_name = &name[..slash_pos];
                        let branch_name = &name[slash_pos + 1..];
                        match self.repo.delete_remote_branch(remote_name, branch_name) {
                            Ok(()) => {
                                self.message = Some(format!("Deleted remote branch '{}'", name));
                                self.refresh_branches()?;
                            }
                            Err(e) => {
                                self.message = Some(format!("Delete failed: {}", e));
                            }
                        }
                    } else {
                        self.message = Some("Invalid remote branch name".to_string());
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
                let remotes = self.repo.remotes()?;
                if let Some(remote) = remotes.first() {
                    self.start_async_push(remote.clone(), None);
                } else {
                    self.message = Some("No remote configured".to_string());
                }
            }
            ConfirmAction::BranchPush => {
                if let Some(ref branch_name) = self.confirm_target {
                    let remotes = self.repo.remotes()?;
                    if let Some(remote) = remotes.first() {
                        self.start_async_push(remote.clone(), Some(branch_name.clone()));
                    } else {
                        self.message = Some("No remote configured".to_string());
                    }
                }
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
            ConfirmAction::DeleteMergedBranches => {
                if let Some((local_branches, remote_branches)) = self.merged_branches_to_delete.take() {
                    let mut deleted_local = 0;
                    let mut deleted_remote = 0;
                    let mut errors = Vec::new();

                    // Delete local branches
                    for branch in &local_branches {
                        match self.repo.delete_branch(branch, false) {
                            Ok(()) => deleted_local += 1,
                            Err(e) => errors.push(format!("{}: {}", branch, e)),
                        }
                    }

                    // Delete remote branches
                    for branch in &remote_branches {
                        // Parse remote/branch from name like "origin/feature"
                        if let Some(slash_pos) = branch.find('/') {
                            let remote_name = &branch[..slash_pos];
                            let branch_name = &branch[slash_pos + 1..];
                            match self.repo.delete_remote_branch(remote_name, branch_name) {
                                Ok(()) => deleted_remote += 1,
                                Err(e) => errors.push(format!("{}: {}", branch, e)),
                            }
                        }
                    }

                    if errors.is_empty() {
                        self.message = Some(format!(
                            "Deleted {} local and {} remote merged branches",
                            deleted_local, deleted_remote
                        ));
                    } else if errors.len() == 1 {
                        self.message = Some(format!(
                            "Deleted {} local, {} remote. Error: {}",
                            deleted_local,
                            deleted_remote,
                            errors[0]
                        ));
                    } else {
                        self.message = Some(format!(
                            "Deleted {} local, {} remote. {} errors: {}",
                            deleted_local,
                            deleted_remote,
                            errors.len(),
                            errors.join("; ")
                        ));
                    }
                    self.refresh_branches()?;
                }
            }
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let (width, height) = self.terminal.size()?;
        let main_top = 0u16;
        let main_height = height.saturating_sub(3); // footer only

        debug!(
            "mouse: {:?} at ({}, {}), drag_state: {}",
            mouse.kind,
            mouse.column,
            mouse.row,
            self.drag_state.is_some()
        );

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
                                self.resize_panels(
                                    &drag_state,
                                    mouse.column,
                                    mouse.row,
                                    width,
                                    main_top,
                                    main_height,
                                );
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
                            if let Some(drag_state) = self.detect_border_at(
                                mouse.column,
                                mouse.row,
                                width,
                                main_top,
                                main_height,
                            ) {
                                debug!("  DRAG START: {:?}", drag_state.drag_type);
                                self.drag_state = Some(drag_state);
                            } else if let Some((panel, row)) = self.panel_click_info(
                                mouse.column,
                                mouse.row,
                                width,
                                main_top,
                                main_height,
                            ) {
                                debug!("  CLICK: panel={:?}, row={}", panel, row);
                                // Focus the panel if different
                                if self.focused_panel != panel {
                                    self.focused_panel = panel;
                                }
                                // Select the item at the clicked row
                                self.handle_panel_click(panel, row)?;
                                self.terminal.force_full_redraw();
                            } else if let Some(panel) = self.panel_at_position(
                                mouse.column,
                                mouse.row,
                                width,
                                main_top,
                                main_height,
                            ) {
                                // Clicked on panel but not on content (e.g., border/title)
                                debug!("  FOCUS CHANGE: {:?} -> {:?}", self.focused_panel, panel);
                                if self.focused_panel != panel {
                                    self.focused_panel = panel;
                                    self.terminal.force_full_redraw();
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if let Some(panel) = self.panel_at_position(
                                mouse.column,
                                mouse.row,
                                width,
                                main_top,
                                main_height,
                            ) {
                                self.scroll_panel_up(panel)?;
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if let Some(panel) = self.panel_at_position(
                                mouse.column,
                                mouse.row,
                                width,
                                main_top,
                                main_height,
                            ) {
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
    fn detect_border_at(
        &self,
        x: u16,
        y: u16,
        width: u16,
        main_top: u16,
        main_height: u16,
    ) -> Option<DragState> {
        if y < main_top || y >= main_top + main_height {
            return None;
        }

        let mouse_x_pct = x as f32 / width as f32;
        let mouse_y_pct = (y - main_top) as f32 / main_height as f32;

        const COL_THRESHOLD: f32 = 0.008;
        const PANEL_THRESHOLD: f32 = 0.012;

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
                                for (right_panel_idx, rpanel) in right_col.panels.iter().enumerate()
                                {
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
    fn resize_panels(
        &mut self,
        drag_state: &DragState,
        x: u16,
        _y: u16,
        width: u16,
        main_top: u16,
        main_height: u16,
    ) {
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
                let panel_start: f32 = column.panels[..panel_idx].iter().map(|p| p.height).sum();

                let combined_height =
                    column.panels[panel_idx].height + column.panels[panel_idx + 1].height;

                let new_height = (mouse_y_pct - panel_start).clamp(0.1, combined_height - 0.1);
                let next_height = combined_height - new_height;

                column.panels[panel_idx].height = new_height;
                column.panels[panel_idx + 1].height = next_height;
            }
            DragType::Intersection {
                col_idx,
                left_panel_idx,
                right_panel_idx,
            } => {
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

                        let new_height =
                            (mouse_y_pct - panel_start).clamp(0.1, combined_height - 0.1);
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

                        let new_height =
                            (mouse_y_pct - panel_start).clamp(0.1, combined_height - 0.1);
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
        let current_idx = all_panels
            .iter()
            .position(|p| *p == self.focused_panel)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % all_panels.len();
        all_panels[next_idx]
    }

    fn prev_panel(&self) -> PanelType {
        let all_panels = self.available_panels();
        let current_idx = all_panels
            .iter()
            .position(|p| *p == self.focused_panel)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            all_panels.len() - 1
        } else {
            current_idx - 1
        };
        all_panels[prev_idx]
    }

    /// Called when focused panel changes - updates preview pane content
    fn on_panel_focus_changed(&mut self) {
        self.terminal.force_full_redraw();
        match self.focused_panel {
            PanelType::PullRequests => {
                self.refresh_pr_preview();
            }
            PanelType::Status => {
                // Restore diff view for status
                self.clear_pr_highlights();
                let _ = self.refresh_diff();
            }
            PanelType::Files => {
                self.clear_pr_highlights();
                let _ = self.refresh_file_preview();
            }
            PanelType::Commits => {
                self.clear_pr_highlights();
                self.refresh_commit_preview();
            }
            _ => {
                // Clear PR preview and highlights for other panels
                self.clear_pr_highlights();
            }
        }
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

    fn panel_at_position(
        &self,
        x: u16,
        y: u16,
        width: u16,
        main_top: u16,
        main_height: u16,
    ) -> Option<PanelType> {
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

    /// Returns (panel, row_index_in_content) if click is inside panel content area
    /// row_index_in_content is the 0-based row within the scrollable content
    fn panel_click_info(
        &self,
        x: u16,
        y: u16,
        width: u16,
        main_top: u16,
        main_height: u16,
    ) -> Option<(PanelType, usize)> {
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
                        // Calculate pixel coordinates of the panel
                        let panel_pixel_x = (col_x * width as f32) as u16;
                        let panel_pixel_y = main_top + (panel_y * main_height as f32) as u16;
                        let panel_pixel_h = ((panel_bottom - panel_y) * main_height as f32) as u16;

                        // Inner area (accounting for border: 1 char each side)
                        let inner_x = panel_pixel_x + 1;
                        let inner_y = panel_pixel_y + 1;
                        let inner_h = panel_pixel_h.saturating_sub(2);

                        // Check if click is within inner area
                        if x > inner_x && y > inner_y && y < inner_y + inner_h {
                            // Adjust by subtracting less to shift detection area down by ~0.5 line
                            let row_in_content = y.saturating_sub(inner_y) as usize;
                            return Some((panel.panel, row_in_content));
                        }

                        return None;
                    }

                    panel_y = panel_bottom;
                }
                break;
            }

            col_x = col_right;
        }

        None
    }

    fn handle_panel_click(&mut self, panel: PanelType, row: usize) -> Result<()> {
        match panel {
            PanelType::Status => {
                self.status_view.select_at_row(row);
                self.refresh_diff()?;
            }
            PanelType::Branches => self.branches_view.select_at_row(row),
            PanelType::Commits => {
                self.commits_view.select_at_row(row);
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.select_at_row(row),
            PanelType::Diff => self.diff_view.select_at_row(row),
            PanelType::Tags => self.tags_view.select_at_row(row),
            PanelType::Remotes => self.remotes_view.select_at_row(row),
            PanelType::Worktrees => self.worktree_view.select_at_row(row),
            PanelType::Submodules => self.submodules_view.select_at_row(row),
            PanelType::Blame => self.blame_view.select_at_row(row),
            PanelType::Files => {
                self.filetree_view.select_at_row(row);
                self.refresh_file_preview()?;
            }
            PanelType::Conflicts => self.conflict_view.select_at_row(row),
            PanelType::PullRequests => self.pull_requests_view.select_at_row(row),
            PanelType::Issues => self.issues_view.select_at_row(row),
            PanelType::Actions => self.actions_view.select_at_row(row),
            PanelType::Releases => self.releases_view.select_at_row(row),
        }
        Ok(())
    }

    fn scroll_panel_up(&mut self, panel: PanelType) -> Result<()> {
        match panel {
            PanelType::Status => {
                self.status_view.move_up();
                self.refresh_diff()?;
            }
            PanelType::Branches => self.branches_view.move_up(),
            PanelType::Commits => {
                self.commits_view.move_up();
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.move_up(),
            PanelType::Diff => self.diff_view.scroll_up(),
            PanelType::Tags => self.tags_view.move_up(),
            PanelType::Remotes => self.remotes_view.move_up(),
            PanelType::Worktrees => self.worktree_view.move_up(),
            PanelType::Submodules => self.submodules_view.move_up(),
            PanelType::Blame => self.blame_view.move_up(),
            PanelType::Files => {
                self.filetree_view.move_up();
                self.refresh_file_preview()?;
            }
            PanelType::Conflicts => self.conflict_view.move_up(),
            PanelType::PullRequests => {
                self.pull_requests_view.move_up();
                self.refresh_pr_preview();
            }
            PanelType::Issues => self.issues_view.move_up(),
            PanelType::Actions => self.actions_view.move_up(),
            PanelType::Releases => self.releases_view.move_up(),
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
            PanelType::Commits => {
                self.commits_view.move_down();
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.move_down(),
            PanelType::Diff => self.diff_view.scroll_down(),
            PanelType::Tags => self.tags_view.move_down(),
            PanelType::Remotes => self.remotes_view.move_down(),
            PanelType::Worktrees => self.worktree_view.move_down(),
            PanelType::Submodules => self.submodules_view.move_down(),
            PanelType::Blame => self.blame_view.move_down(),
            PanelType::Files => {
                self.filetree_view.move_down();
                self.refresh_file_preview()?;
            }
            PanelType::Conflicts => self.conflict_view.move_down(),
            PanelType::PullRequests => {
                self.pull_requests_view.move_down();
                self.refresh_pr_preview();
            }
            PanelType::Issues => self.issues_view.move_down(),
            PanelType::Actions => self.actions_view.move_down(),
            PanelType::Releases => self.releases_view.move_down(),
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
                self.on_panel_focus_changed();
            }

            KeyCode::BackTab => {
                self.focused_panel = self.prev_panel();
                self.on_panel_focus_changed();
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

            // Search navigation (next/prev result)
            KeyCode::Char('n') => {
                let result_count = match self.focused_panel {
                    PanelType::Status => {
                        self.status_view.next_search_result();
                        self.status_view.search_results.len()
                    }
                    PanelType::Branches => {
                        self.branches_view.next_search_result();
                        self.branches_view.search_results.len()
                    }
                    PanelType::Commits => {
                        self.commits_view.next_search_result();
                        self.refresh_commit_preview();
                        self.commits_view.search_results.len()
                    }
                    PanelType::Stash => {
                        self.stash_view.next_search_result();
                        self.stash_view.search_results.len()
                    }
                    PanelType::Tags => {
                        self.tags_view.next_search_result();
                        self.tags_view.search_results.len()
                    }
                    PanelType::Files => {
                        self.filetree_view.next_search_result();
                        self.filetree_view.search_results.len()
                    }
                    PanelType::Remotes => {
                        self.remotes_view.next_search_result();
                        self.remotes_view.search_results.len()
                    }
                    PanelType::Diff => {
                        self.diff_view.next_search_result();
                        self.diff_view.search_matches.len()
                    }
                    PanelType::PullRequests => {
                        self.pull_requests_view.next_search_result();
                        self.pull_requests_view.search_results.len()
                    }
                    PanelType::Issues => {
                        self.issues_view.next_search_result();
                        self.issues_view.search_results.len()
                    }
                    PanelType::Actions => {
                        self.actions_view.next_search_result();
                        self.actions_view.search_results.len()
                    }
                    PanelType::Releases => {
                        self.releases_view.next_search_result();
                        self.releases_view.search_results.len()
                    }
                    _ => 0,
                };
                if result_count == 0 {
                    self.message = Some("No search results".to_string());
                }
            }

            KeyCode::Char('N') => {
                let result_count = match self.focused_panel {
                    PanelType::Status => {
                        self.status_view.prev_search_result();
                        self.status_view.search_results.len()
                    }
                    PanelType::Branches => {
                        self.branches_view.prev_search_result();
                        self.branches_view.search_results.len()
                    }
                    PanelType::Commits => {
                        self.commits_view.prev_search_result();
                        self.refresh_commit_preview();
                        self.commits_view.search_results.len()
                    }
                    PanelType::Stash => {
                        self.stash_view.prev_search_result();
                        self.stash_view.search_results.len()
                    }
                    PanelType::Tags => {
                        self.tags_view.prev_search_result();
                        self.tags_view.search_results.len()
                    }
                    PanelType::Files => {
                        self.filetree_view.prev_search_result();
                        self.filetree_view.search_results.len()
                    }
                    PanelType::Remotes => {
                        self.remotes_view.prev_search_result();
                        self.remotes_view.search_results.len()
                    }
                    PanelType::Diff => {
                        self.diff_view.prev_search_result();
                        self.diff_view.search_matches.len()
                    }
                    PanelType::PullRequests => {
                        self.pull_requests_view.prev_search_result();
                        self.pull_requests_view.search_results.len()
                    }
                    PanelType::Issues => {
                        self.issues_view.prev_search_result();
                        self.issues_view.search_results.len()
                    }
                    PanelType::Actions => {
                        self.actions_view.prev_search_result();
                        self.actions_view.search_results.len()
                    }
                    PanelType::Releases => {
                        self.releases_view.prev_search_result();
                        self.releases_view.search_results.len()
                    }
                    _ => 0,
                };
                if result_count == 0 {
                    self.message = Some("No search results".to_string());
                }
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

            // Push to remote (Branches panel)
            KeyCode::Char('P') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    self.confirm_target = Some(branch.name.clone());
                    self.mode = Mode::Confirm(ConfirmAction::BranchPush);
                }
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
                    } else if branch.branch_type == crate::git::BranchType::Remote {
                        // Remote branch - use remote delete
                        self.confirm_target = Some(branch.name.clone());
                        self.mode = Mode::Confirm(ConfirmAction::RemoteBranchDelete);
                    } else {
                        // Local branch
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

            // Reset/Revert to branch commit
            KeyCode::Char('R') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    self.confirm_target = Some(branch.last_commit.id.clone());
                    self.select_index = 0;
                    self.mode = Mode::Select(SelectAction::ResetOrRevert);
                }
            }

            // Delete all merged branches
            KeyCode::Char('M') if self.focused_panel == PanelType::Branches => {
                match self.repo.merged_branches() {
                    Ok((local, remote)) => {
                        if local.is_empty() && remote.is_empty() {
                            self.message = Some("No merged branches to delete".to_string());
                        } else {
                            self.confirm_target = Some(format!("{} local, {} remote", local.len(), remote.len()));
                            self.merged_branches_to_delete = Some((local, remote));
                            self.mode = Mode::Confirm(ConfirmAction::DeleteMergedBranches);
                        }
                    }
                    Err(e) => {
                        self.message = Some(format!("Error finding merged branches: {}", e));
                    }
                }
            }

            // Branch fetch
            KeyCode::Char('f') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    let branch_name = branch.name.clone();
                    let remotes = self.repo.remotes()?;
                    if let Some(remote) = remotes.first() {
                        self.start_async_fetch(remote.clone(), branch_name);
                    } else {
                        self.message = Some("No remote configured".to_string());
                    }
                }
            }

            // Branch pull (local) / fetch (remote)
            KeyCode::Char('p') if self.focused_panel == PanelType::Branches => {
                if let Some(branch) = self.branches_view.selected_branch() {
                    let branch_name = branch.name.clone();

                    if branch.branch_type == crate::git::BranchType::Remote {
                        // For remote branches, fetch instead of pull
                        // Parse remote/branch from name like "origin/main"
                        if let Some(slash_pos) = branch_name.find('/') {
                            let remote_name = branch_name[..slash_pos].to_string();
                            let ref_name = branch_name[slash_pos + 1..].to_string();
                            self.start_async_fetch(remote_name, ref_name);
                        } else {
                            self.message = Some("Invalid remote branch name".to_string());
                        }
                    } else {
                        // For local branches, pull
                        let remotes = self.repo.remotes()?;
                        if let Some(remote) = remotes.first() {
                            self.start_async_pull(remote.clone(), branch_name);
                        } else {
                            self.message = Some("No remote configured".to_string());
                        }
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
                // Reset/Revert commit
                if let Some(commit) = self.commits_view.selected_commit() {
                    self.confirm_target = Some(commit.id.clone());
                    self.select_index = 0;
                    self.mode = Mode::Select(SelectAction::ResetOrRevert);
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

            // Tags panel - Reset/Revert to tag commit
            KeyCode::Char('R') if self.focused_panel == PanelType::Tags => {
                if let Some(tag) = self.tags_view.selected_tag() {
                    self.confirm_target = Some(tag.target.clone());
                    self.select_index = 0;
                    self.mode = Mode::Select(SelectAction::ResetOrRevert);
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

            // Enter visual (line selection) mode in Diff/Preview pane
            KeyCode::Char('V') if self.focused_panel == PanelType::Diff => {
                self.diff_view.enter_visual_mode();
                self.mode = Mode::Visual;
            }

            KeyCode::Char('r') => {
                self.refresh_all()?;
                self.message = Some("Refreshed".to_string());
            }

            // Retry/Load for GitHub panes (PRs, Issues, Actions, Releases)
            KeyCode::Char('R') if self.focused_panel == PanelType::PullRequests => {
                if self.pull_requests_view.can_retry() {
                    self.start_loading_pull_requests();
                    self.message = Some("Loading pull requests...".to_string());
                }
            }
            KeyCode::Char('R') if self.focused_panel == PanelType::Issues => {
                if self.issues_view.can_retry() {
                    self.start_loading_issues();
                    self.message = Some("Loading issues...".to_string());
                }
            }
            KeyCode::Char('R') if self.focused_panel == PanelType::Actions => {
                if self.actions_view.can_retry() {
                    self.start_loading_actions();
                    self.message = Some("Loading actions...".to_string());
                }
            }
            KeyCode::Char('R') if self.focused_panel == PanelType::Releases => {
                if self.releases_view.can_retry() {
                    self.start_loading_releases();
                    self.message = Some("Loading releases...".to_string());
                }
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
                    let match_count = match self.focused_panel {
                        PanelType::Status => {
                            self.status_view.search(&self.input_buffer);
                            self.status_view.search_results.len()
                        }
                        PanelType::Branches => {
                            self.branches_view.search(&self.input_buffer);
                            self.branches_view.search_results.len()
                        }
                        PanelType::Commits => {
                            self.commits_view.search(&self.input_buffer);
                            self.commits_view.search_results.len()
                        }
                        PanelType::Stash => {
                            self.stash_view.search(&self.input_buffer);
                            self.stash_view.search_results.len()
                        }
                        PanelType::Tags => {
                            self.tags_view.search(&self.input_buffer);
                            self.tags_view.search_results.len()
                        }
                        PanelType::Files => {
                            self.filetree_view.search(&self.input_buffer);
                            self.filetree_view.search_results.len()
                        }
                        PanelType::Remotes => {
                            self.remotes_view.search(&self.input_buffer);
                            self.remotes_view.search_results.len()
                        }
                        PanelType::Diff => {
                            self.diff_view.search(&self.input_buffer);
                            self.diff_view.search_matches.len()
                        }
                        PanelType::PullRequests => {
                            self.pull_requests_view.search(&self.input_buffer);
                            self.pull_requests_view.search_results.len()
                        }
                        PanelType::Issues => {
                            self.issues_view.search(&self.input_buffer);
                            self.issues_view.search_results.len()
                        }
                        PanelType::Actions => {
                            self.actions_view.search(&self.input_buffer);
                            self.actions_view.search_results.len()
                        }
                        PanelType::Releases => {
                            self.releases_view.search(&self.input_buffer);
                            self.releases_view.search_results.len()
                        }
                        _ => 0,
                    };
                    if match_count == 0 {
                        self.message = Some(format!("Pattern not found: {}", self.input_buffer));
                    } else {
                        self.message = Some(format!("[{} matches] n/N to navigate", match_count));
                    }
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

    fn handle_visual_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Escape | KeyCode::Char('q') => {
                // Exit visual mode
                self.diff_view.exit_visual_mode();
                self.mode = Mode::Normal;
                self.message = None;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                // Move cursor down (extend selection)
                let max_lines = self.diff_view.get_total_lines();
                self.diff_view.visual_move_down(max_lines);
                // Adjust scroll if needed
                let (_, height) = self.terminal.size().unwrap_or((80, 24));
                let visible_height = height.saturating_sub(4) as usize; // Approximate
                self.diff_view.ensure_cursor_visible(visible_height);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Move cursor up (extend selection)
                self.diff_view.visual_move_up();
            }
            KeyCode::Char('g') => {
                // Move to top
                self.diff_view.cursor_line = 0;
                self.diff_view.scroll = 0;
            }
            KeyCode::Char('G') => {
                // Move to bottom
                let max_lines = self.diff_view.get_total_lines();
                if max_lines > 0 {
                    self.diff_view.cursor_line = max_lines - 1;
                    let (_, height) = self.terminal.size().unwrap_or((80, 24));
                    let visible_height = height.saturating_sub(4) as usize;
                    self.diff_view.ensure_cursor_visible(visible_height);
                }
            }
            KeyCode::Char('y') => {
                // Yank (copy) selected lines to clipboard
                if let Some(text) = self.diff_view.get_selected_text() {
                    // Copy to system clipboard
                    if self.copy_to_clipboard(&text) {
                        let (start, end) = self.diff_view.get_selection_range();
                        let line_count = end - start + 1;
                        self.message = Some(format!("{} line(s) yanked", line_count));
                    } else {
                        self.message = Some("Failed to copy to clipboard".to_string());
                    }
                    // Exit visual mode after yank
                    self.diff_view.exit_visual_mode();
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn copy_to_clipboard(&self, text: &str) -> bool {
        // Use pbcopy on macOS, xclip on Linux
        #[cfg(target_os = "macos")]
        {
            use std::io::Write;
            use std::process::{Command, Stdio};

            if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                return child.wait().map(|s| s.success()).unwrap_or(false);
            }
            false
        }
        #[cfg(target_os = "linux")]
        {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Try xclip first, then xsel
            for cmd in &["xclip", "xsel"] {
                let args = if *cmd == "xclip" {
                    vec!["-selection", "clipboard"]
                } else {
                    vec!["--clipboard", "--input"]
                };

                if let Ok(mut child) = Command::new(cmd).args(&args).stdin(Stdio::piped()).spawn() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    if child.wait().map(|s| s.success()).unwrap_or(false) {
                        return true;
                    }
                }
            }
            false
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            false
        }
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
            PanelType::Commits => {
                self.commits_view.move_up();
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.move_up(),
            PanelType::Diff => self.diff_view.cursor_up(),
            PanelType::Tags => self.tags_view.move_up(),
            PanelType::Remotes => self.remotes_view.move_up(),
            PanelType::Worktrees => self.worktree_view.move_up(),
            PanelType::Submodules => self.submodules_view.move_up(),
            PanelType::Blame => self.blame_view.move_up(),
            PanelType::Files => {
                self.filetree_view.move_up();
                self.refresh_file_preview()?;
            }
            PanelType::Conflicts => self.conflict_view.move_up(),
            PanelType::PullRequests => {
                self.pull_requests_view.move_up();
                self.refresh_pr_preview();
            }
            PanelType::Issues => self.issues_view.move_up(),
            PanelType::Actions => self.actions_view.move_up(),
            PanelType::Releases => self.releases_view.move_up(),
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
            PanelType::Commits => {
                self.commits_view.move_down();
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.move_down(),
            PanelType::Diff => {
                let (_, height) = self.terminal.size().unwrap_or((80, 24));
                let visible_height = height.saturating_sub(6) as usize; // Approximate visible lines
                self.diff_view.cursor_down(visible_height);
            }
            PanelType::Tags => self.tags_view.move_down(),
            PanelType::Remotes => self.remotes_view.move_down(),
            PanelType::Worktrees => self.worktree_view.move_down(),
            PanelType::Submodules => self.submodules_view.move_down(),
            PanelType::Blame => self.blame_view.move_down(),
            PanelType::Files => {
                self.filetree_view.move_down();
                self.refresh_file_preview()?;
            }
            PanelType::Conflicts => self.conflict_view.move_down(),
            PanelType::PullRequests => {
                self.pull_requests_view.move_down();
                self.refresh_pr_preview();
            }
            PanelType::Issues => self.issues_view.move_down(),
            PanelType::Actions => self.actions_view.move_down(),
            PanelType::Releases => self.releases_view.move_down(),
        }
        Ok(())
    }

    fn item_top(&mut self) {
        match self.focused_panel {
            PanelType::Status => self.status_view.move_to_top(),
            PanelType::Branches => self.branches_view.move_to_top(),
            PanelType::Commits => {
                self.commits_view.move_to_top();
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.move_to_top(),
            PanelType::Diff => self.diff_view.cursor_to_top(),
            PanelType::Tags => self.tags_view.move_to_top(),
            PanelType::Remotes => self.remotes_view.move_to_top(),
            PanelType::Worktrees => self.worktree_view.move_to_top(),
            PanelType::Submodules => self.submodules_view.move_to_top(),
            PanelType::Blame => self.blame_view.move_to_top(),
            PanelType::Files => self.filetree_view.move_to_top(),
            PanelType::Conflicts => self.conflict_view.move_to_top(),
            PanelType::PullRequests => {
                self.pull_requests_view.move_to_top();
                self.refresh_pr_preview();
            }
            PanelType::Issues => self.issues_view.move_to_top(),
            PanelType::Actions => self.actions_view.move_to_top(),
            PanelType::Releases => self.releases_view.move_to_top(),
        }
    }

    fn item_bottom(&mut self) {
        match self.focused_panel {
            PanelType::Status => self.status_view.move_to_bottom(),
            PanelType::Branches => self.branches_view.move_to_bottom(),
            PanelType::Commits => {
                self.commits_view.move_to_bottom();
                self.refresh_commit_preview();
            }
            PanelType::Stash => self.stash_view.move_to_bottom(),
            PanelType::Diff => {
                let (_, height) = self.terminal.size().unwrap_or((80, 24));
                let visible_height = height.saturating_sub(6) as usize;
                self.diff_view.cursor_to_bottom(visible_height);
            }
            PanelType::Tags => self.tags_view.move_to_bottom(),
            PanelType::Remotes => self.remotes_view.move_to_bottom(),
            PanelType::Worktrees => self.worktree_view.move_to_bottom(),
            PanelType::Submodules => self.submodules_view.move_to_bottom(),
            PanelType::Blame => self.blame_view.move_to_bottom(),
            PanelType::Files => self.filetree_view.move_to_bottom(),
            PanelType::Conflicts => self.conflict_view.move_to_bottom(),
            PanelType::PullRequests => {
                self.pull_requests_view.move_to_bottom();
                self.refresh_pr_preview();
            }
            PanelType::Issues => self.issues_view.move_to_bottom(),
            PanelType::Actions => self.actions_view.move_to_bottom(),
            PanelType::Releases => self.releases_view.move_to_bottom(),
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
            PanelType::PullRequests => self.pull_requests_view.scroll_left(),
            PanelType::Issues => self.issues_view.scroll_left(),
            PanelType::Actions => self.actions_view.scroll_left(),
            PanelType::Releases => self.releases_view.scroll_left(),
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
            PanelType::PullRequests => self.pull_requests_view.scroll_right(),
            PanelType::Issues => self.issues_view.scroll_right(),
            PanelType::Actions => self.actions_view.scroll_right(),
            PanelType::Releases => self.releases_view.scroll_right(),
        }
    }

    fn focus_pane_up(&mut self) {
        let old_panel = self.focused_panel;
        match self.view_mode {
            ViewMode::SinglePane => {
                // In zoom mode, up/down also cycles panels
                self.focused_panel = self.prev_panel();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_above(self.focused_panel) {
                    self.focused_panel = panel;
                }
            }
        }
        if self.focused_panel != old_panel {
            self.on_panel_focus_changed();
        }
    }

    fn focus_pane_down(&mut self) {
        let old_panel = self.focused_panel;
        match self.view_mode {
            ViewMode::SinglePane => {
                // In zoom mode, up/down also cycles panels
                self.focused_panel = self.next_panel();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_below(self.focused_panel) {
                    self.focused_panel = panel;
                }
            }
        }
        if self.focused_panel != old_panel {
            self.on_panel_focus_changed();
        }
    }

    fn focus_pane_left(&mut self) {
        let old_panel = self.focused_panel;
        match self.view_mode {
            ViewMode::SinglePane => {
                self.focused_panel = self.prev_panel();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_left(self.focused_panel) {
                    self.focused_panel = panel;
                }
            }
        }
        if self.focused_panel != old_panel {
            self.on_panel_focus_changed();
        }
    }

    fn focus_pane_right(&mut self) {
        let old_panel = self.focused_panel;
        match self.view_mode {
            ViewMode::SinglePane => {
                self.focused_panel = self.next_panel();
            }
            ViewMode::MultiPane => {
                if let Some(panel) = self.config.layout.panel_right(self.focused_panel) {
                    self.focused_panel = panel;
                }
            }
        }
        if self.focused_panel != old_panel {
            self.on_panel_focus_changed();
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

            let combined =
                self.config.layout.columns[idx1].width + self.config.layout.columns[idx2].width;

            // Apply delta to first column (with clamping)
            let new_width1 =
                (self.config.layout.columns[idx1].width + delta).clamp(0.1, combined - 0.1);
            let new_width2 = combined - new_width1;

            self.config.layout.columns[idx1].width = new_width1;
            self.config.layout.columns[idx2].width = new_width2;

            self.save_layout_config();
            self.terminal.force_full_redraw();
        }
    }

    /// Update the files pane filter based on marked commits
    fn update_files_filter_from_marked_commits(&mut self) -> Result<()> {
        let marked = self.commits_view.get_marked_commits();
        if marked.is_empty() {
            self.filetree_view.clear_filter();
            self.message = None;
        } else {
            let commit_ids: Vec<String> = marked.iter().cloned().collect();
            match self.repo.files_changed_in_commits(&commit_ids) {
                Ok(files) => {
                    let count = files.len();
                    self.filetree_view.set_filter(files);
                    self.message = Some(format!("Filtered: {} files from {} commits", count, commit_ids.len()));
                }
                Err(e) => {
                    self.message = Some(format!("Error getting files: {}", e));
                }
            }
        }
        Ok(())
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
                        let branch_type = branch.branch_type;
                        self.repo.switch_branch(&name, branch_type)?;
                        // Show the local branch name (strip remote prefix if remote branch)
                        let display_name = match branch_type {
                            crate::git::BranchType::Remote => {
                                if let Some(pos) = name.find('/') {
                                    &name[pos + 1..]
                                } else {
                                    &name
                                }
                            }
                            crate::git::BranchType::Local => &name,
                        };
                        self.message = Some(format!("Switched to: {}", display_name));
                        self.refresh_all()?;
                    }
                }
            }
            PanelType::Commits => {
                // Toggle mark on selected commit
                self.commits_view.toggle_mark();
                // Update files pane filter based on marked commits
                self.update_files_filter_from_marked_commits()?;
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
                // Expand/collapse directories, or open file in preview
                if let Some(entry) = self.filetree_view.selected_entry() {
                    if entry.is_dir {
                        let path = entry.path.clone();
                        if let Some(load_path) = self.filetree_view.toggle_expand() {
                            // Lazy load children for this directory
                            let children = self
                                .repo
                                .file_tree_dir(&load_path, self.filetree_view.show_ignored)?;
                            self.filetree_view.load_children(&path, children);
                        }
                    } else {
                        // Open file in preview and move focus to Diff/Preview pane
                        self.refresh_file_preview()?;
                        self.focused_panel = PanelType::Diff;
                        self.terminal.force_full_redraw();
                    }
                }
            }
            PanelType::Conflicts => {
                // Open conflict resolution (show in diff)
                self.focused_panel = PanelType::Diff;
            }
            PanelType::PullRequests => {
                // Could open PR in browser in future
            }
            PanelType::Issues => {
                // Could open issue in browser in future
            }
            PanelType::Actions => {
                // Could open action run in browser in future
            }
            PanelType::Releases => {
                // Could open release in browser in future
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
                // Command-line checkout assumes local branch
                self.repo.switch_branch(&name, crate::git::BranchType::Local)?;
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
                    self.repo
                        .create_branch(&self.input_buffer, from.as_deref())?;
                    self.refresh_branches()?;
                    match from {
                        Some(ref f) => {
                            self.message = Some(format!(
                                "Created branch '{}' from '{}'",
                                self.input_buffer, f
                            ))
                        }
                        None => {
                            self.message = Some(format!("Created branch: {}", self.input_buffer))
                        }
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
        PanelType::PullRequests => "pullrequests",
        PanelType::Issues => "issues",
        PanelType::Actions => "actions",
        PanelType::Releases => "releases",
    }
}

/// Fetch pull requests from GitHub (runs in background thread)
fn fetch_pull_requests(repo_path: &PathBuf) -> std::result::Result<Vec<PullRequestInfo>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "pr",
            "list",
            "--json",
            "number,title,author,state,createdAt,baseRefName,headRefName,additions,deletions,isDraft,body,url",
            "--limit",
            "100",
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// Fetch commits for a specific PR (on-demand when PR is selected)
fn fetch_pr_commits(
    repo_path: &PathBuf,
    pr_number: u32,
) -> std::result::Result<Vec<String>, String> {
    let output = std::process::Command::new("gh")
        .args(["pr", "view", &pr_number.to_string(), "--json", "commits"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);

    // Parse the JSON to extract commit OIDs
    #[derive(serde::Deserialize)]
    struct PrCommitsResponse {
        commits: Vec<CommitOid>,
    }
    #[derive(serde::Deserialize)]
    struct CommitOid {
        oid: String,
    }

    let response: PrCommitsResponse = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    Ok(response.commits.into_iter().map(|c| c.oid).collect())
}

/// Fetch issues from GitHub (runs in background thread)
fn fetch_issues(repo_path: &PathBuf) -> std::result::Result<Vec<IssueInfo>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "issue",
            "list",
            "--json",
            "number,title,author,state,createdAt,labels",
            "--limit",
            "100",
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// Fetch workflow runs from GitHub (runs in background thread)
fn fetch_workflow_runs(repo_path: &PathBuf) -> std::result::Result<Vec<WorkflowRun>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "run",
            "list",
            "--json",
            "databaseId,name,displayTitle,status,conclusion,headBranch,createdAt",
            "--limit",
            "100",
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// Fetch releases from GitHub (runs in background thread)
fn fetch_releases(repo_path: &PathBuf) -> std::result::Result<Vec<ReleaseInfo>, String> {
    let output = std::process::Command::new("gh")
        .args([
            "release",
            "list",
            "--json",
            "tagName,name,isDraft,isPrerelease,createdAt",
            "--limit",
            "100",
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}
