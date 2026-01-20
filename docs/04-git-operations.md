# Git Operations Design

## Overview

All Git operations are performed through the `git2` crate, which provides Rust bindings to libgit2.

## Repository Wrapper

```rust
use git2::{Repository, Error as Git2Error};

pub struct GitRepository {
    repo: Repository,
    path: PathBuf,
}

impl GitRepository {
    pub fn open_current_dir() -> Result<Self> {
        let repo = Repository::discover(".")?;
        let path = repo.path().parent()
            .unwrap_or(repo.path())
            .to_path_buf();
        Ok(Self { repo, path })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::open(path)?;
        Ok(Self { repo, path: path.to_path_buf() })
    }
}
```

## Branch Operations

### List Branches

```rust
impl GitRepository {
    pub fn branches(&self, filter: BranchFilter) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();

        let branch_type = match filter {
            BranchFilter::Local => Some(git2::BranchType::Local),
            BranchFilter::Remote => Some(git2::BranchType::Remote),
            BranchFilter::All => None,
        };

        for branch_result in self.repo.branches(branch_type)? {
            let (branch, branch_type) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();
            let is_head = branch.is_head();

            let commit = branch.get().peel_to_commit()?;
            let last_commit = CommitInfo::from_commit(&commit);

            branches.push(BranchInfo {
                name,
                branch_type: branch_type.into(),
                is_head,
                last_commit,
            });
        }

        Ok(branches)
    }

    pub fn current_branch(&self) -> Result<Option<String>> {
        let head = self.repo.head()?;
        if head.is_branch() {
            Ok(head.shorthand().map(|s| s.to_string()))
        } else {
            // Detached HEAD
            Ok(None)
        }
    }
}

pub struct BranchInfo {
    pub name: String,
    pub branch_type: BranchType,
    pub is_head: bool,
    pub last_commit: CommitInfo,
}

pub enum BranchType {
    Local,
    Remote,
}

pub enum BranchFilter {
    Local,
    Remote,
    All,
}
```

### Create/Delete Branch

```rust
impl GitRepository {
    pub fn create_branch(&self, name: &str, target: Option<&str>) -> Result<()> {
        let commit = match target {
            Some(ref_name) => {
                let obj = self.repo.revparse_single(ref_name)?;
                obj.peel_to_commit()?
            }
            None => {
                self.repo.head()?.peel_to_commit()?
            }
        };

        self.repo.branch(name, &commit, false)?;
        Ok(())
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let mut branch = self.repo.find_branch(name, git2::BranchType::Local)?;

        if !force && branch.is_head() {
            return Err(Error::Git("Cannot delete current branch".into()));
        }

        branch.delete()?;
        Ok(())
    }

    pub fn switch_branch(&self, name: &str) -> Result<()> {
        let obj = self.repo.revparse_single(&format!("refs/heads/{}", name))?;
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&format!("refs/heads/{}", name))?;
        Ok(())
    }
}
```

## Commit Operations

### Commit History

```rust
impl GitRepository {
    pub fn commits(&self, max_count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::with_capacity(max_count);

        for (i, oid) in revwalk.enumerate() {
            if i >= max_count {
                break;
            }

            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitInfo::from_commit(&commit));
        }

        Ok(commits)
    }

    pub fn search_commits(&self, query: &str, max_count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let query_lower = query.to_lowercase();
        let mut commits = Vec::new();

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            let message = commit.message().unwrap_or("");
            let author = commit.author().name().unwrap_or("");

            if message.to_lowercase().contains(&query_lower)
                || author.to_lowercase().contains(&query_lower)
            {
                commits.push(CommitInfo::from_commit(&commit));
                if commits.len() >= max_count {
                    break;
                }
            }
        }

        Ok(commits)
    }
}

pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub time: i64,
}

impl CommitInfo {
    fn from_commit(commit: &git2::Commit) -> Self {
        let id = commit.id().to_string();
        let short_id = id[..7].to_string();

        Self {
            id,
            short_id,
            message: commit.message().unwrap_or("").lines().next().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            email: commit.author().email().unwrap_or("").to_string(),
            time: commit.time().seconds(),
        }
    }
}
```

### Create Commit

```rust
impl GitRepository {
    pub fn commit(&self, message: &str) -> Result<String> {
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;

        let signature = self.repo.signature()?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;

        let commit_oid = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(commit_oid.to_string())
    }
}
```

## Status/Staging Operations

```rust
impl GitRepository {
    pub fn status(&self) -> Result<Vec<StatusEntry>> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        opts.recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            entries.push(StatusEntry {
                path,
                staged: Self::get_staged_status(status),
                unstaged: Self::get_unstaged_status(status),
            });
        }

        Ok(entries)
    }

    fn get_staged_status(status: git2::Status) -> FileStatus {
        if status.contains(git2::Status::INDEX_NEW) {
            FileStatus::Added
        } else if status.contains(git2::Status::INDEX_MODIFIED) {
            FileStatus::Modified
        } else if status.contains(git2::Status::INDEX_DELETED) {
            FileStatus::Deleted
        } else if status.contains(git2::Status::INDEX_RENAMED) {
            FileStatus::Renamed
        } else {
            FileStatus::Unmodified
        }
    }

    fn get_unstaged_status(status: git2::Status) -> FileStatus {
        if status.contains(git2::Status::WT_NEW) {
            FileStatus::Untracked
        } else if status.contains(git2::Status::WT_MODIFIED) {
            FileStatus::Modified
        } else if status.contains(git2::Status::WT_DELETED) {
            FileStatus::Deleted
        } else if status.contains(git2::Status::WT_RENAMED) {
            FileStatus::Renamed
        } else {
            FileStatus::Unmodified
        }
    }

    pub fn stage_file(&self, path: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    pub fn unstage_file(&self, path: &str) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.reset_default(Some(&head.into_object()), &[Path::new(path)])?;
        Ok(())
    }

    pub fn stage_all(&self) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }
}

pub struct StatusEntry {
    pub path: String,
    pub staged: FileStatus,
    pub unstaged: FileStatus,
}

pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}
```

## Diff Operations

```rust
impl GitRepository {
    pub fn diff_staged(&self) -> Result<DiffInfo> {
        let head_tree = self.repo.head()?.peel_to_tree()?;
        let diff = self.repo.diff_tree_to_index(Some(&head_tree), None, None)?;
        Self::parse_diff(&diff)
    }

    pub fn diff_unstaged(&self) -> Result<DiffInfo> {
        let diff = self.repo.diff_index_to_workdir(None, None)?;
        Self::parse_diff(&diff)
    }

    pub fn diff_file(&self, path: &str, staged: bool) -> Result<DiffInfo> {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path);

        let diff = if staged {
            let head_tree = self.repo.head()?.peel_to_tree()?;
            self.repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        } else {
            self.repo.diff_index_to_workdir(None, Some(&mut opts))?
        };

        Self::parse_diff(&diff)
    }

    fn parse_diff(diff: &git2::Diff) -> Result<DiffInfo> {
        let mut files = Vec::new();
        let mut current_file: Option<FileDiff> = None;

        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            // Parse diff output into structured data
            // ... implementation
            true
        })?;

        Ok(DiffInfo { files })
    }
}

pub struct DiffInfo {
    pub files: Vec<FileDiff>,
}

pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<Hunk>,
}

pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

pub struct DiffLine {
    pub line_type: LineType,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

pub enum LineType {
    Context,
    Addition,
    Deletion,
}
```

## Stash Operations

```rust
impl GitRepository {
    pub fn stash_list(&self) -> Result<Vec<StashEntry>> {
        let mut stashes = Vec::new();

        self.repo.stash_foreach(|index, message, oid| {
            stashes.push(StashEntry {
                index,
                message: message.to_string(),
                id: oid.to_string(),
            });
            true
        })?;

        Ok(stashes)
    }

    pub fn stash_save(&self, message: Option<&str>) -> Result<()> {
        let signature = self.repo.signature()?;
        self.repo.stash_save(&signature, message.unwrap_or(""), None)?;
        Ok(())
    }

    pub fn stash_pop(&self, index: usize) -> Result<()> {
        self.repo.stash_pop(index, None)?;
        Ok(())
    }

    pub fn stash_drop(&self, index: usize) -> Result<()> {
        self.repo.stash_drop(index)?;
        Ok(())
    }
}

pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub id: String,
}
```

## Remote Operations

```rust
impl GitRepository {
    pub fn remotes(&self) -> Result<Vec<String>> {
        let remotes = self.repo.remotes()?;
        Ok(remotes.iter().filter_map(|r| r.map(|s| s.to_string())).collect())
    }

    pub fn push(&self, remote: &str, branch: &str, callbacks: RemoteCallbacks) -> Result<()> {
        let mut remote = self.repo.find_remote(remote)?;
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks.into());

        remote.push(&[&refspec], Some(&mut push_opts))?;
        Ok(())
    }

    pub fn pull(&self, remote: &str, branch: &str, callbacks: RemoteCallbacks) -> Result<()> {
        let mut remote = self.repo.find_remote(remote)?;

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks.into());

        remote.fetch(&[branch], Some(&mut fetch_opts), None)?;

        // Fast-forward merge
        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = fetch_head.peel_to_commit()?;

        let mut reference = self.repo.head()?;
        reference.set_target(fetch_commit.id(), "pull: fast-forward")?;

        self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    }
}

pub struct RemoteCallbacks {
    pub credentials: Option<Box<dyn Fn() -> Result<git2::Cred>>>,
    pub progress: Option<Box<dyn Fn(usize, usize)>>,
}
```

## Merge Operations

```rust
impl GitRepository {
    pub fn merge(&self, branch: &str) -> Result<MergeResult> {
        let branch_ref = self.repo.find_branch(branch, git2::BranchType::Local)?;
        let annotated_commit = self.repo.reference_to_annotated_commit(branch_ref.get())?;

        let (analysis, _) = self.repo.merge_analysis(&[&annotated_commit])?;

        if analysis.contains(git2::MergeAnalysis::ANALYSIS_UP_TO_DATE) {
            return Ok(MergeResult::UpToDate);
        }

        if analysis.contains(git2::MergeAnalysis::ANALYSIS_FASTFORWARD) {
            // Fast-forward
            let target_commit = self.repo.find_commit(annotated_commit.id())?;
            let mut head_ref = self.repo.head()?;
            head_ref.set_target(target_commit.id(), "merge: fast-forward")?;
            self.repo.checkout_head(None)?;
            return Ok(MergeResult::FastForward);
        }

        // Normal merge
        self.repo.merge(&[&annotated_commit], None, None)?;

        if self.repo.index()?.has_conflicts() {
            Ok(MergeResult::Conflict)
        } else {
            // Auto-commit merge
            self.commit_merge(branch)?;
            Ok(MergeResult::Merged)
        }
    }

    fn commit_merge(&self, branch: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;

        let head = self.repo.head()?.peel_to_commit()?;
        let branch_ref = self.repo.find_branch(branch, git2::BranchType::Local)?;
        let branch_commit = branch_ref.get().peel_to_commit()?;

        let signature = self.repo.signature()?;
        let message = format!("Merge branch '{}'", branch);

        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&head, &branch_commit],
        )?;

        self.repo.cleanup_state()?;
        Ok(())
    }
}

pub enum MergeResult {
    UpToDate,
    FastForward,
    Merged,
    Conflict,
}
```

## Tag Operations

```rust
impl GitRepository {
    pub fn tags(&self) -> Result<Vec<TagInfo>> {
        let mut tags = Vec::new();

        self.repo.tag_foreach(|oid, name| {
            let name = String::from_utf8_lossy(name)
                .trim_start_matches("refs/tags/")
                .to_string();

            if let Ok(tag) = self.repo.find_tag(oid) {
                tags.push(TagInfo {
                    name,
                    message: tag.message().map(|s| s.to_string()),
                    target: tag.target_id().to_string(),
                    is_annotated: true,
                });
            } else {
                tags.push(TagInfo {
                    name,
                    message: None,
                    target: oid.to_string(),
                    is_annotated: false,
                });
            }
            true
        })?;

        Ok(tags)
    }

    pub fn create_tag(&self, name: &str, message: Option<&str>) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;

        match message {
            Some(msg) => {
                let signature = self.repo.signature()?;
                self.repo.tag(name, head.as_object(), &signature, msg, false)?;
            }
            None => {
                self.repo.tag_lightweight(name, head.as_object(), false)?;
            }
        }

        Ok(())
    }

    pub fn delete_tag(&self, name: &str) -> Result<()> {
        self.repo.tag_delete(name)?;
        Ok(())
    }
}

pub struct TagInfo {
    pub name: String,
    pub message: Option<String>,
    pub target: String,
    pub is_annotated: bool,
}
```

## Rebase Operations

```rust
impl GitRepository {
    pub fn rebase(&self, upstream: &str) -> Result<RebaseResult> {
        let upstream_commit = self.repo.revparse_single(upstream)?.peel_to_commit()?;
        let upstream_annotated = self.repo.find_annotated_commit(upstream_commit.id())?;

        let mut rebase = self.repo.rebase(None, Some(&upstream_annotated), None, None)?;

        let mut conflicts = Vec::new();

        while let Some(op) = rebase.next() {
            let op = op?;

            if self.repo.index()?.has_conflicts() {
                conflicts.push(op.id().to_string());
                break;
            }

            let signature = self.repo.signature()?;
            rebase.commit(None, &signature, None)?;
        }

        if conflicts.is_empty() {
            rebase.finish(None)?;
            Ok(RebaseResult::Success)
        } else {
            Ok(RebaseResult::Conflict(conflicts))
        }
    }

    pub fn rebase_continue(&self) -> Result<RebaseResult> {
        let mut rebase = self.repo.open_rebase(None)?;
        // ... continue logic
        Ok(RebaseResult::Success)
    }

    pub fn rebase_abort(&self) -> Result<()> {
        let mut rebase = self.repo.open_rebase(None)?;
        rebase.abort()?;
        Ok(())
    }
}

pub enum RebaseResult {
    Success,
    Conflict(Vec<String>),
}
```
