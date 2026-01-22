use std::path::{Path, PathBuf};
use git2::{Repository as Git2Repository, Signature};
use crate::error::{Error, Result};
use super::branch::{BranchInfo, BranchType};
use super::commit::CommitInfo;
use super::status::{StatusEntry, FileStatus};
use super::diff::{DiffInfo, FileDiff, Hunk, DiffLine, LineType};
use super::stash::StashEntry;
use super::tag::TagInfo;
use super::worktree::WorktreeInfo;
use super::submodule::SubmoduleInfo;
use super::blame::{BlameInfo, BlameLine};
use super::filetree::{FileTreeEntry, FileTreeStatus};
use super::conflict::{ConflictEntry, ConflictType};
use super::loggraph::GraphCommit;

pub struct Repository {
    repo: Git2Repository,
    path: PathBuf,
}

impl Repository {
    pub fn discover() -> Result<Self> {
        let repo = Git2Repository::discover(".")?;
        let path = repo.workdir()
            .unwrap_or_else(|| repo.path())
            .to_path_buf();
        Ok(Self { repo, path })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Git2Repository::open(path.as_ref())?;
        let path = repo.workdir()
            .unwrap_or_else(|| repo.path())
            .to_path_buf();
        Ok(Self { repo, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn head_name(&self) -> Result<Option<String>> {
        let head = self.repo.head()?;
        if head.is_branch() {
            Ok(head.shorthand().map(|s| s.to_string()))
        } else {
            // Detached HEAD
            Ok(None)
        }
    }

    pub fn head_commit_short(&self) -> Result<String> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        let id = commit.id().to_string();
        Ok(id[..7.min(id.len())].to_string())
    }

    pub fn is_clean(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(statuses.is_empty())
    }

    pub fn ahead_behind(&self) -> Result<(usize, usize)> {
        let head = match self.repo.head() {
            Ok(h) => h,
            Err(_) => return Ok((0, 0)),
        };

        if !head.is_branch() {
            return Ok((0, 0));
        }

        let branch_name = match head.shorthand() {
            Some(n) => n,
            None => return Ok((0, 0)),
        };

        let local = self.repo.find_branch(branch_name, git2::BranchType::Local)?;
        let upstream = match local.upstream() {
            Ok(u) => u,
            Err(_) => return Ok((0, 0)),
        };

        let local_oid = local.get().target().unwrap();
        let upstream_oid = upstream.get().target().unwrap();

        let (ahead, behind) = self.repo.graph_ahead_behind(local_oid, upstream_oid)?;
        Ok((ahead, behind))
    }

    // Branch operations
    pub fn branches(&self, include_remote: bool) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();

        // Local branches
        for branch_result in self.repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();
            let is_head = branch.is_head();

            let commit = branch.get().peel_to_commit()?;
            let last_commit = CommitInfo::from_commit(&commit);

            // Get upstream tracking info
            let (ahead, behind) = match branch.upstream() {
                Ok(upstream) => {
                    let local_oid = branch.get().target().unwrap();
                    let upstream_oid = upstream.get().target().unwrap();
                    self.repo.graph_ahead_behind(local_oid, upstream_oid).unwrap_or((0, 0))
                }
                Err(_) => (0, 0),
            };

            branches.push(BranchInfo {
                name,
                branch_type: BranchType::Local,
                is_head,
                last_commit,
                ahead,
                behind,
            });
        }

        // Remote branches
        if include_remote {
            for branch_result in self.repo.branches(Some(git2::BranchType::Remote))? {
                let (branch, _) = branch_result?;
                let name = branch.name()?.unwrap_or("").to_string();

                let commit = branch.get().peel_to_commit()?;
                let last_commit = CommitInfo::from_commit(&commit);

                branches.push(BranchInfo {
                    name,
                    branch_type: BranchType::Remote,
                    is_head: false,
                    last_commit,
                    ahead: 0,
                    behind: 0,
                });
            }
        }

        Ok(branches)
    }

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

        if branch.is_head() {
            return Err(Error::Git(git2::Error::from_str("Cannot delete current branch")));
        }

        // Check if branch is merged into HEAD
        if !force {
            let branch_commit = branch.get().peel_to_commit()?;
            let head_commit = self.repo.head()?.peel_to_commit()?;

            // Check if branch commit is ancestor of HEAD (i.e., merged)
            let is_merged = self.repo.merge_base(branch_commit.id(), head_commit.id())
                .map(|base| base == branch_commit.id())
                .unwrap_or(false);

            if !is_merged {
                return Err(Error::Git(git2::Error::from_str(
                    "Branch not fully merged. Use D to force delete."
                )));
            }
        }

        branch.delete()?;
        Ok(())
    }

    pub fn switch_branch(&self, name: &str) -> Result<()> {
        let refname = format!("refs/heads/{}", name);
        let obj = self.repo.revparse_single(&refname)?;

        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&refname)?;
        Ok(())
    }

    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<()> {
        let mut branch = self.repo.find_branch(old_name, git2::BranchType::Local)?;
        branch.rename(new_name, false)?;
        Ok(())
    }

    pub fn checkout_commit(&self, commit_id: &str) -> Result<()> {
        let obj = self.repo.revparse_single(commit_id)?;
        let commit = obj.peel_to_commit()?;

        self.repo.checkout_tree(&commit.into_object(), None)?;
        self.repo.set_head_detached(obj.id())?;
        Ok(())
    }

    pub fn revert_commit(&self, commit_id: &str) -> Result<()> {
        let obj = self.repo.revparse_single(commit_id)?;
        let commit = obj.peel_to_commit()?;

        // Revert creates a new commit that undoes the changes
        self.repo.revert(&commit, None)?;
        Ok(())
    }

    // Commit operations
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

            let message = commit.message().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("").to_string();

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

    // Status operations
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
                staged: FileStatus::from_index_status(status),
                unstaged: FileStatus::from_workdir_status(status),
            });
        }

        Ok(entries)
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

    pub fn unstage_all(&self) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.reset(&head.into_object(), git2::ResetType::Mixed, None)?;
        Ok(())
    }

    pub fn discard_file(&self, path: &str) -> Result<()> {
        let mut opts = git2::build::CheckoutBuilder::new();
        opts.path(path);
        opts.force();
        self.repo.checkout_head(Some(&mut opts))?;
        Ok(())
    }

    // Diff operations
    pub fn diff_staged(&self) -> Result<DiffInfo> {
        let head_tree = self.repo.head()?.peel_to_tree()?;
        let diff = self.repo.diff_tree_to_index(Some(&head_tree), None, None)?;
        Self::parse_diff(&diff)
    }

    pub fn diff_unstaged(&self) -> Result<DiffInfo> {
        let diff = self.repo.diff_index_to_workdir(None, None)?;
        Self::parse_diff(&diff)
    }

    fn parse_diff(diff: &git2::Diff) -> Result<DiffInfo> {
        let mut files = Vec::new();
        let mut current_file: Option<FileDiff> = None;
        let mut current_hunk: Option<Hunk> = None;
        let mut last_hunk_start: Option<(u32, u32)> = None;

        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            let file_path = delta.new_file().path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            // Check if we're starting a new file
            if current_file.as_ref().map(|f| &f.path) != Some(&file_path) {
                if let Some(mut file) = current_file.take() {
                    if let Some(h) = current_hunk.take() {
                        file.hunks.push(h);
                    }
                    files.push(file);
                }
                current_file = Some(FileDiff {
                    path: file_path.clone(),
                    hunks: Vec::new(),
                });
                last_hunk_start = None;
            }

            // Check if we're starting a new hunk (only when hunk start position changes)
            if let Some(h) = hunk {
                let hunk_key = (h.old_start(), h.new_start());
                if last_hunk_start != Some(hunk_key) {
                    last_hunk_start = Some(hunk_key);
                    if let Some(file) = current_file.as_mut() {
                        if let Some(prev_hunk) = current_hunk.take() {
                            file.hunks.push(prev_hunk);
                        }
                        let header = format!(
                            "@@ -{},{} +{},{} @@",
                            h.old_start(), h.old_lines(),
                            h.new_start(), h.new_lines()
                        );
                        current_hunk = Some(Hunk {
                            header,
                            old_start: h.old_start(),
                            old_lines: h.old_lines(),
                            new_start: h.new_start(),
                            new_lines: h.new_lines(),
                            lines: Vec::new(),
                        });
                    }
                }
            }

            // Add line to current hunk
            if let Some(hunk) = current_hunk.as_mut() {
                let content = String::from_utf8_lossy(line.content()).to_string();
                let line_type = match line.origin() {
                    '+' => LineType::Addition,
                    '-' => LineType::Deletion,
                    _ => LineType::Context,
                };
                hunk.lines.push(DiffLine {
                    line_type,
                    content,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                });
            }

            true
        })?;

        // Don't forget the last file/hunk
        if let Some(mut file) = current_file {
            if let Some(h) = current_hunk {
                file.hunks.push(h);
            }
            files.push(file);
        }

        Ok(DiffInfo { files })
    }

    // Stash operations
    pub fn stash_list(&mut self) -> Result<Vec<StashEntry>> {
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

    pub fn stash_save(&mut self, message: Option<&str>) -> Result<()> {
        let signature = self.repo.signature()?;
        self.repo.stash_save(&signature, message.unwrap_or("WIP"), None)?;
        Ok(())
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_pop(index, None)?;
        Ok(())
    }

    pub fn stash_apply(&mut self, index: usize) -> Result<()> {
        self.repo.stash_apply(index, None)?;
        Ok(())
    }

    pub fn stash_drop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_drop(index)?;
        Ok(())
    }

    // Tag operations
    pub fn tags(&self) -> Result<Vec<TagInfo>> {
        let mut tags = Vec::new();

        let tag_names = self.repo.tag_names(None)?;
        for name in tag_names.iter().flatten() {
            let refname = format!("refs/tags/{}", name);
            if let Ok(reference) = self.repo.find_reference(&refname) {
                let target = reference.target().map(|oid| oid.to_string()[..7].to_string());

                // Try to get annotation
                let message = if let Ok(obj) = reference.peel(git2::ObjectType::Tag) {
                    if let Ok(tag) = obj.into_tag() {
                        tag.message().map(|m| m.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let is_annotated = message.is_some();
                tags.push(TagInfo {
                    name: name.to_string(),
                    message,
                    target: target.unwrap_or_default(),
                    is_annotated,
                });
            }
        }

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

    // Merge operations
    pub fn merge(&self, branch_name: &str) -> Result<MergeResult> {
        let branch = self.repo.find_branch(branch_name, git2::BranchType::Local)?;
        let annotated = self.repo.reference_to_annotated_commit(branch.get())?;

        let (analysis, _) = self.repo.merge_analysis(&[&annotated])?;

        if analysis.contains(git2::MergeAnalysis::ANALYSIS_UP_TO_DATE) {
            return Ok(MergeResult::UpToDate);
        }

        if analysis.contains(git2::MergeAnalysis::ANALYSIS_FASTFORWARD) {
            let target = self.repo.find_commit(annotated.id())?;
            let mut head_ref = self.repo.head()?;
            head_ref.set_target(target.id(), "merge: fast-forward")?;
            self.repo.checkout_head(None)?;
            return Ok(MergeResult::FastForward);
        }

        // Normal merge
        self.repo.merge(&[&annotated], None, None)?;

        if self.repo.index()?.has_conflicts() {
            Ok(MergeResult::Conflict)
        } else {
            // Commit the merge
            self.commit_merge(branch_name)?;
            Ok(MergeResult::Merged)
        }
    }

    fn commit_merge(&self, branch_name: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;

        let head = self.repo.head()?.peel_to_commit()?;
        let branch = self.repo.find_branch(branch_name, git2::BranchType::Local)?;
        let branch_commit = branch.get().peel_to_commit()?;

        let signature = self.repo.signature()?;
        let message = format!("Merge branch '{}'", branch_name);

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

    // Remote operations
    pub fn remotes(&self) -> Result<Vec<String>> {
        let remotes = self.repo.remotes()?;
        Ok(remotes.iter().filter_map(|r| r.map(|s| s.to_string())).collect())
    }

    pub fn fetch(&self, remote_name: &str) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        remote.fetch(&[] as &[&str], None, None)?;
        Ok(())
    }

    // Remote info
    pub fn remote_info(&self) -> Result<Vec<RemoteInfo>> {
        let mut remotes = Vec::new();
        for name in self.repo.remotes()?.iter().flatten() {
            if let Ok(remote) = self.repo.find_remote(name) {
                remotes.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("").to_string(),
                    push_url: remote.pushurl().map(|s| s.to_string()),
                });
            }
        }
        Ok(remotes)
    }

    // Worktree operations
    pub fn worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let mut worktrees = Vec::new();

        // Main worktree
        let main_path = self.path.to_string_lossy().to_string();
        let main_head = self.head_name()?.unwrap_or_else(|| self.head_commit_short().unwrap_or_default());
        worktrees.push(WorktreeInfo {
            name: "main".to_string(),
            path: main_path,
            head: main_head,
            is_main: true,
            is_locked: false,
        });

        // Additional worktrees
        if let Ok(wts) = self.repo.worktrees() {
            for name in wts.iter().flatten() {
                if let Ok(wt) = self.repo.find_worktree(name) {
                    let path = wt.path().to_string_lossy().to_string();
                    let is_locked = wt.is_locked().is_ok();
                    worktrees.push(WorktreeInfo {
                        name: name.to_string(),
                        path,
                        head: String::new(), // Would need to open the worktree to get head
                        is_main: false,
                        is_locked,
                    });
                }
            }
        }

        Ok(worktrees)
    }

    // Submodule operations
    pub fn submodules(&self) -> Result<Vec<SubmoduleInfo>> {
        let mut submodules = Vec::new();

        for sm in self.repo.submodules()? {
            let name = sm.name().unwrap_or("").to_string();
            let path = sm.path().to_string_lossy().to_string();
            let url = sm.url().unwrap_or("").to_string();
            let head = sm.head_id().map(|oid| oid.to_string()[..7].to_string());

            submodules.push(SubmoduleInfo {
                name,
                path,
                url,
                head,
                is_initialized: sm.open().is_ok(),
            });
        }

        Ok(submodules)
    }

    pub fn submodule_update(&self, name: &str) -> Result<()> {
        let mut sm = self.repo.find_submodule(name)?;
        sm.update(true, None)?;
        Ok(())
    }

    // Blame operations
    pub fn blame_file(&self, path: &str) -> Result<BlameInfo> {
        let blame = self.repo.blame_file(Path::new(path), None)?;
        let mut lines = Vec::new();

        // Read the file content
        let file_path = self.path.join(path);
        let content = std::fs::read_to_string(&file_path).unwrap_or_default();

        for (i, line_content) in content.lines().enumerate() {
            let line_num = i + 1;
            if let Some(hunk) = blame.get_line(line_num) {
                let commit_id = hunk.final_commit_id().to_string();
                let sig = hunk.final_signature();
                let author = sig.name().unwrap_or("").to_string();
                let time = sig.when();
                let date = format_timestamp(time.seconds());

                lines.push(BlameLine {
                    line_number: line_num,
                    commit_id: commit_id[..7.min(commit_id.len())].to_string(),
                    author,
                    date,
                    content: line_content.to_string(),
                });
            }
        }

        Ok(BlameInfo {
            path: path.to_string(),
            lines,
        })
    }

    // File tree operations
    pub fn file_tree(&self) -> Result<Vec<FileTreeEntry>> {
        let statuses = self.status()?;
        let status_map: std::collections::HashMap<String, FileTreeStatus> = statuses
            .iter()
            .map(|e| {
                let status = if e.unstaged == FileStatus::Untracked {
                    FileTreeStatus::Untracked
                } else if e.unstaged == FileStatus::Deleted || e.staged == FileStatus::Deleted {
                    FileTreeStatus::Deleted
                } else if e.staged == FileStatus::Added {
                    FileTreeStatus::Added
                } else if e.unstaged.is_changed() || e.staged.is_changed() {
                    FileTreeStatus::Modified
                } else {
                    FileTreeStatus::Modified
                };
                (e.path.clone(), status)
            })
            .collect();

        self.build_file_tree(&self.path, "", &status_map)
    }

    fn build_file_tree(
        &self,
        base_path: &Path,
        relative_path: &str,
        status_map: &std::collections::HashMap<String, FileTreeStatus>,
    ) -> Result<Vec<FileTreeEntry>> {
        let mut entries = Vec::new();
        let full_path = if relative_path.is_empty() {
            base_path.to_path_buf()
        } else {
            base_path.join(relative_path)
        };

        if let Ok(read_dir) = std::fs::read_dir(&full_path) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files and .git
                if name.starts_with('.') {
                    continue;
                }

                let entry_relative = if relative_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", relative_path, name)
                };

                if entry.path().is_dir() {
                    let children = self.build_file_tree(base_path, &entry_relative, status_map)?;
                    let mut dir_entry = FileTreeEntry::new_dir(name, entry_relative);
                    dir_entry.children = children;
                    entries.push(dir_entry);
                } else {
                    let status = status_map.get(&entry_relative).copied();
                    entries.push(FileTreeEntry::new_file(name, entry_relative, status));
                }
            }
        }

        // Sort: directories first, then files, alphabetically
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }

    // Conflict operations
    pub fn conflicts(&self) -> Result<Vec<ConflictEntry>> {
        let index = self.repo.index()?;
        let mut conflicts = Vec::new();

        // Note: conflict detection is done below

        // Alternative: check for conflict markers in files
        for entry in index.conflicts()? {
            if let Ok(conflict) = entry {
                let path = conflict.our
                    .as_ref()
                    .or(conflict.their.as_ref())
                    .or(conflict.ancestor.as_ref())
                    .and_then(|e| std::str::from_utf8(&e.path).ok())
                    .unwrap_or("")
                    .to_string();

                let conflict_type = match (&conflict.our, &conflict.their) {
                    (Some(_), Some(_)) => ConflictType::BothModified,
                    (Some(_), None) => ConflictType::DeletedByThem,
                    (None, Some(_)) => ConflictType::DeletedByUs,
                    (None, None) => ConflictType::BothModified,
                };

                conflicts.push(ConflictEntry {
                    path,
                    conflict_type,
                    ours: None,
                    theirs: None,
                    ancestor: None,
                });
            }
        }

        Ok(conflicts)
    }

    pub fn resolve_conflict_ours(&self, path: &str) -> Result<()> {
        // Checkout our version
        let mut opts = git2::build::CheckoutBuilder::new();
        opts.path(path);
        opts.force();
        self.repo.checkout_index(None, Some(&mut opts))?;

        // Stage the file
        self.stage_file(path)?;
        Ok(())
    }

    pub fn resolve_conflict_theirs(&self, path: &str) -> Result<()> {
        // For theirs, we need to use MERGE_HEAD
        let merge_head = self.repo.find_reference("MERGE_HEAD")?;
        let commit = merge_head.peel_to_commit()?;
        let tree = commit.tree()?;

        if let Some(entry) = tree.get_path(Path::new(path)).ok() {
            let blob = self.repo.find_blob(entry.id())?;
            std::fs::write(self.path.join(path), blob.content())?;
        }

        self.stage_file(path)?;
        Ok(())
    }

    // Log graph operations
    pub fn log_graph(&self, max_count: usize) -> Result<Vec<GraphCommit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        // Also push all branches for a complete graph
        for branch in self.repo.branches(None)? {
            if let Ok((branch, _)) = branch {
                if let Some(oid) = branch.get().target() {
                    let _ = revwalk.push(oid);
                }
            }
        }

        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;

        // Build ref map for labels
        let mut ref_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        // Add branches
        for branch in self.repo.branches(None)? {
            if let Ok((branch, _)) = branch {
                if let Some(oid) = branch.get().target() {
                    let name = branch.name().ok().flatten().unwrap_or("").to_string();
                    ref_map.entry(oid.to_string()).or_default().push(name);
                }
            }
        }

        // Add tags
        for tag_name in self.repo.tag_names(None)?.iter().flatten() {
            let refname = format!("refs/tags/{}", tag_name);
            if let Ok(reference) = self.repo.find_reference(&refname) {
                if let Some(oid) = reference.target() {
                    ref_map.entry(oid.to_string()).or_default().push(format!("tag: {}", tag_name));
                }
            }
        }

        let mut commits = Vec::new();
        let mut graph_state: Vec<Option<git2::Oid>> = Vec::new();

        for oid in revwalk.take(max_count) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            // Build graph characters
            let graph_chars = self.build_graph_line(&commit, &mut graph_state);

            let parents: Vec<String> = commit.parents()
                .map(|p| p.id().to_string()[..7].to_string())
                .collect();

            let refs = ref_map.get(&oid.to_string()).cloned().unwrap_or_default();

            commits.push(GraphCommit {
                id: oid.to_string(),
                short_id: oid.to_string()[..7].to_string(),
                message: commit.summary().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                date: format_timestamp(commit.time().seconds()),
                parents,
                graph_chars,
                refs,
            });
        }

        Ok(commits)
    }

    fn build_graph_line(&self, commit: &git2::Commit, state: &mut Vec<Option<git2::Oid>>) -> String {
        let oid = commit.id();
        let parent_count = commit.parent_count();

        // Find or add this commit to state
        let col = state.iter().position(|s| *s == Some(oid))
            .unwrap_or_else(|| {
                // Find empty slot or add new
                if let Some(pos) = state.iter().position(|s| s.is_none()) {
                    state[pos] = Some(oid);
                    pos
                } else {
                    state.push(Some(oid));
                    state.len() - 1
                }
            });

        let mut chars = String::new();

        // Build the line
        for (i, slot) in state.iter().enumerate() {
            if i == col {
                chars.push('*');
            } else if slot.is_some() {
                chars.push('|');
            } else {
                chars.push(' ');
            }
            chars.push(' ');
        }

        // Update state for parents
        if parent_count == 0 {
            state[col] = None;
        } else if parent_count == 1 {
            state[col] = Some(commit.parent_id(0).unwrap());
        } else {
            // Merge commit - first parent takes this slot
            state[col] = Some(commit.parent_id(0).unwrap());
            // Additional parents get new slots
            for i in 1..parent_count {
                if let Ok(parent_id) = commit.parent_id(i) {
                    if let Some(pos) = state.iter().position(|s| s.is_none()) {
                        state[pos] = Some(parent_id);
                    } else {
                        state.push(Some(parent_id));
                    }
                }
            }
        }

        // Clean up empty trailing slots
        while state.last() == Some(&None) {
            state.pop();
        }

        chars
    }
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

fn format_timestamp(timestamp: i64) -> String {
    // Simple formatting - in a real app you'd use chrono
    let secs = timestamp;
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let remaining_days = days % 365;
    let months = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;
    format!("{:04}-{:02}-{:02}", years, months, day)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeResult {
    UpToDate,
    FastForward,
    Merged,
    Conflict,
}
