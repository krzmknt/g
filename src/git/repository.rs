use super::blame::{BlameInfo, BlameLine};
use super::branch::{BranchInfo, BranchType, UpstreamInfo};
use super::commit::CommitInfo;
use super::conflict::{ConflictEntry, ConflictType};
use super::diff::{DiffInfo, DiffLine, FileDiff, Hunk, LineType};
use super::filetree::{FileTreeEntry, FileTreeStatus};
use super::loggraph::{GraphCommit, GraphLine};
use super::stash::StashEntry;
use super::status::{FileStatus, StatusEntry};
use super::submodule::SubmoduleInfo;
use super::tag::TagInfo;
use super::worktree::WorktreeInfo;
use crate::error::{Error, Result};
use git2::{Repository as Git2Repository, Signature};
use std::path::{Path, PathBuf};

pub struct Repository {
    repo: Git2Repository,
    path: PathBuf,
}

impl Repository {
    pub fn discover() -> Result<Self> {
        let repo = Git2Repository::discover(".")?;
        let path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        Ok(Self { repo, path })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Git2Repository::open(path.as_ref())?;
        let path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
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

        let local = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)?;
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
            let (ahead, behind, upstream) = match branch.upstream() {
                Ok(upstream_branch) => {
                    let local_oid = branch.get().target().unwrap();
                    let upstream_oid = upstream_branch.get().target().unwrap();
                    let (a, b) = self.repo
                        .graph_ahead_behind(local_oid, upstream_oid)
                        .unwrap_or((0, 0));
                    let upstream_name = upstream_branch.name().ok().flatten().unwrap_or("").to_string();
                    let upstream_short_id = upstream_oid.to_string()[..7].to_string();
                    (a, b, Some(UpstreamInfo {
                        name: upstream_name,
                        short_id: upstream_short_id,
                    }))
                }
                Err(_) => (0, 0, None),
            };

            branches.push(BranchInfo {
                name,
                branch_type: BranchType::Local,
                is_head,
                last_commit,
                ahead,
                behind,
                upstream,
            });
        }

        // Remote branches
        if include_remote {
            for branch_result in self.repo.branches(Some(git2::BranchType::Remote))? {
                let (branch, _) = branch_result?;
                let name = branch.name()?.unwrap_or("").to_string();

                // Skip symbolic refs like origin/HEAD
                if name.ends_with("/HEAD") {
                    continue;
                }

                // Skip refs that may not resolve to a commit
                let commit = match branch.get().peel_to_commit() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let last_commit = CommitInfo::from_commit(&commit);

                branches.push(BranchInfo {
                    name,
                    branch_type: BranchType::Remote,
                    is_head: false,
                    last_commit,
                    ahead: 0,
                    behind: 0,
                    upstream: None,
                });
            }
        }

        Ok(branches)
    }

    /// Check if all commits of branch_a are reachable from branch_b
    /// (i.e., branch_a's tip is an ancestor of branch_b's tip)
    pub fn is_branch_ancestor(&self, branch_a: &BranchInfo, branch_b: &BranchInfo) -> bool {
        let oid_a = match git2::Oid::from_str(&branch_a.last_commit.id) {
            Ok(oid) => oid,
            Err(_) => return false,
        };
        let oid_b = match git2::Oid::from_str(&branch_b.last_commit.id) {
            Ok(oid) => oid,
            Err(_) => return false,
        };

        // Check if oid_a is an ancestor of oid_b
        self.repo.graph_descendant_of(oid_b, oid_a).unwrap_or(false)
    }

    pub fn create_branch(&self, name: &str, target: Option<&str>) -> Result<()> {
        let commit = match target {
            Some(ref_name) => {
                let obj = self.repo.revparse_single(ref_name)?;
                obj.peel_to_commit()?
            }
            None => self.repo.head()?.peel_to_commit()?,
        };

        self.repo.branch(name, &commit, false)?;
        Ok(())
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let mut branch = self.repo.find_branch(name, git2::BranchType::Local)?;

        if branch.is_head() {
            return Err(Error::Git(git2::Error::from_str(
                "Cannot delete current branch",
            )));
        }

        // Check if branch is merged into HEAD
        if !force {
            let branch_commit = branch.get().peel_to_commit()?;
            let head_commit = self.repo.head()?.peel_to_commit()?;

            // Check if branch commit is ancestor of HEAD (i.e., merged)
            let is_merged = self
                .repo
                .merge_base(branch_commit.id(), head_commit.id())
                .map(|base| base == branch_commit.id())
                .unwrap_or(false);

            if !is_merged {
                return Err(Error::Git(git2::Error::from_str(
                    "Branch not fully merged. Use D to force delete.",
                )));
            }
        }

        branch.delete()?;
        Ok(())
    }

    /// Delete a remote branch using git push --delete
    /// If the branch doesn't exist on the remote, delete the local remote-tracking reference
    pub fn delete_remote_branch(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        // First, try to delete from the remote server
        let output = std::process::Command::new("git")
            .args(["push", "--delete", remote_name, branch_name])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if output.status.success() {
            return Ok(());
        }

        // If push --delete failed, try to delete the local remote-tracking reference
        // This handles the case where the branch was already deleted on the remote
        let ref_name = format!("refs/remotes/{}/{}", remote_name, branch_name);
        let output = std::process::Command::new("git")
            .args(["update-ref", "-d", &ref_name])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }
        Ok(())
    }

    /// Get all branches that are fully merged into the current HEAD
    pub fn merged_branches(&self) -> Result<(Vec<String>, Vec<String>)> {
        let head_commit = self.repo.head()?.peel_to_commit()?;
        let head_oid = head_commit.id();
        let current_branch = self.head_name()?.unwrap_or_default();

        let mut local_merged = Vec::new();
        let mut remote_merged = Vec::new();

        // Check local branches
        for branch_result in self.repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();

            // Skip the current branch
            if name == current_branch || branch.is_head() {
                continue;
            }

            // Check if the branch commit is an ancestor of HEAD (i.e., merged)
            if let Ok(branch_commit) = branch.get().peel_to_commit() {
                let is_merged = self
                    .repo
                    .merge_base(branch_commit.id(), head_oid)
                    .map(|base| base == branch_commit.id())
                    .unwrap_or(false);

                if is_merged {
                    local_merged.push(name);
                }
            }
        }

        // Check remote branches
        for branch_result in self.repo.branches(Some(git2::BranchType::Remote))? {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();

            // Skip HEAD refs like origin/HEAD
            if name.ends_with("/HEAD") {
                continue;
            }

            // Skip the remote tracking branch for the current branch
            // e.g., if current is 'main', skip 'origin/main'
            if let Some(remote_branch) = name.split('/').last() {
                if remote_branch == current_branch {
                    continue;
                }
            }

            // Check if the branch commit is an ancestor of HEAD (i.e., merged)
            if let Ok(branch_commit) = branch.get().peel_to_commit() {
                let is_merged = self
                    .repo
                    .merge_base(branch_commit.id(), head_oid)
                    .map(|base| base == branch_commit.id())
                    .unwrap_or(false);

                if is_merged {
                    remote_merged.push(name);
                }
            }
        }

        Ok((local_merged, remote_merged))
    }

    pub fn switch_branch(&self, name: &str, branch_type: BranchType) -> Result<()> {
        match branch_type {
            BranchType::Local => {
                // Local branch - switch directly
                let refname = format!("refs/heads/{}", name);
                let obj = self.repo.revparse_single(&refname)?;
                self.repo.checkout_tree(&obj, None)?;
                self.repo.set_head(&refname)?;
            }
            BranchType::Remote => {
                // Remote branch (e.g., "origin/feature-branch")
                // Extract local branch name from remote ref (strip remote name prefix)
                let local_name = if let Some(slash_pos) = name.find('/') {
                    &name[slash_pos + 1..]
                } else {
                    name
                };

                // Check if a local branch with that name already exists
                if self
                    .repo
                    .find_branch(local_name, git2::BranchType::Local)
                    .is_ok()
                {
                    // Local branch exists, switch to it
                    let refname = format!("refs/heads/{}", local_name);
                    let obj = self.repo.revparse_single(&refname)?;
                    self.repo.checkout_tree(&obj, None)?;
                    self.repo.set_head(&refname)?;
                } else {
                    // Create local branch from remote and switch to it
                    // Use git checkout -b which sets up tracking automatically
                    let output = std::process::Command::new("git")
                        .args(["checkout", "-b", local_name, name])
                        .current_dir(&self.path)
                        .output()
                        .map_err(|e| Error::Io(e))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(Error::Git(git2::Error::from_str(&stderr)));
                    }
                }
            }
        }
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


    /// Reset HEAD to a specific commit
    /// reset_type: "soft" (keep staged), "mixed" (keep working dir), "hard" (discard all)
    pub fn reset_to_commit(&self, commit_id: &str, reset_type: &str) -> Result<()> {
        let obj = self.repo.revparse_single(commit_id)?;
        let commit = obj.peel_to_commit()?;

        let reset_type = match reset_type {
            "soft" => git2::ResetType::Soft,
            "mixed" => git2::ResetType::Mixed,
            "hard" => git2::ResetType::Hard,
            _ => git2::ResetType::Mixed,
        };

        self.repo.reset(commit.as_object(), reset_type, None)?;
        Ok(())
    }

    // Commit operations
    pub fn commits(&self, max_count: usize) -> Result<Vec<CommitInfo>> {
        // Build a map of commit ID -> refs (branches/tags)
        let mut ref_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Collect branch refs
        if let Ok(branches) = self.repo.branches(None) {
            for branch_result in branches {
                if let Ok((branch, _)) = branch_result {
                    if let Ok(commit) = branch.get().peel_to_commit() {
                        let name = branch.name().ok().flatten().unwrap_or("").to_string();
                        if !name.is_empty() {
                            ref_map
                                .entry(commit.id().to_string())
                                .or_default()
                                .push(name);
                        }
                    }
                }
            }
        }

        // Collect tag refs
        if let Ok(tags) = self.repo.tag_names(None) {
            for tag_name in tags.iter().flatten() {
                if let Ok(reference) = self.repo.find_reference(&format!("refs/tags/{}", tag_name))
                {
                    if let Ok(commit) = reference.peel_to_commit() {
                        ref_map
                            .entry(commit.id().to_string())
                            .or_default()
                            .push(format!("tag:{}", tag_name));
                    }
                }
            }
        }

        let mut revwalk = self.repo.revwalk()?;

        // Push all branches (local and remote) to include all commits
        for branch_result in self.repo.branches(None)? {
            if let Ok((branch, _)) = branch_result {
                if let Some(oid) = branch.get().target() {
                    let _ = revwalk.push(oid);
                }
            }
        }

        // Also push HEAD in case it's detached
        let _ = revwalk.push_head();

        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::with_capacity(max_count);

        for (i, oid) in revwalk.enumerate() {
            if i >= max_count {
                break;
            }

            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let refs = ref_map.get(&oid.to_string()).cloned().unwrap_or_default();
            commits.push(CommitInfo::from_commit(&commit).with_refs(refs));
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
        self.repo
            .reset_default(Some(&head.into_object()), &[Path::new(path)])?;
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
        self.repo
            .reset(&head.into_object(), git2::ResetType::Mixed, None)?;
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
            let file_path = delta
                .new_file()
                .path()
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
                            h.old_start(),
                            h.old_lines(),
                            h.new_start(),
                            h.new_lines()
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
        self.repo
            .stash_save(&signature, message.unwrap_or("WIP"), None)?;
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
                let target = reference
                    .target()
                    .map(|oid| oid.to_string()[..7].to_string());

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
                self.repo
                    .tag(name, head.as_object(), &signature, msg, false)?;
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
        let branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)?;
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
        let branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)?;
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
        Ok(remotes
            .iter()
            .filter_map(|r| r.map(|s| s.to_string()))
            .collect())
    }

    /// Fetches GitHub pull requests using the gh CLI tool.
    /// Returns an empty Vec if gh is not installed or this is not a GitHub repo.
    pub fn pull_requests(&self) -> Result<Vec<super::PullRequestInfo>> {
        let repo_dir = self.repo.path().parent().unwrap_or(self.repo.path());

        let output = std::process::Command::new("gh")
            .args([
                "pr", "list",
                "--json", "number,title,author,state,createdAt,baseRefName,headRefName,additions,deletions,isDraft",
                "--limit", "100"
            ])
            .current_dir(repo_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<Vec<super::PullRequestInfo>>(&json_str) {
                    Ok(prs) => Ok(prs),
                    Err(_) => Ok(Vec::new()),
                }
            }
            _ => Ok(Vec::new()), // Return empty on error (gh not installed, not a GitHub repo, etc.)
        }
    }

    /// Fetches GitHub issues using the gh CLI tool.
    /// Returns an empty Vec if gh is not installed or this is not a GitHub repo.
    pub fn issues(&self) -> Result<Vec<super::IssueInfo>> {
        let repo_dir = self.repo.path().parent().unwrap_or(self.repo.path());

        let output = std::process::Command::new("gh")
            .args([
                "issue",
                "list",
                "--json",
                "number,title,author,state,createdAt,labels,comments",
                "--limit",
                "100",
            ])
            .current_dir(repo_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<Vec<super::IssueInfo>>(&json_str) {
                    Ok(issues) => Ok(issues),
                    Err(_) => Ok(Vec::new()),
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Fetches GitHub Actions workflow runs using the gh CLI tool.
    /// Returns an empty Vec if gh is not installed or this is not a GitHub repo.
    pub fn workflow_runs(&self) -> Result<Vec<super::WorkflowRun>> {
        let repo_dir = self.repo.path().parent().unwrap_or(self.repo.path());

        let output = std::process::Command::new("gh")
            .args([
                "run",
                "list",
                "--json",
                "name,headBranch,status,conclusion,createdAt,displayTitle,workflowName",
                "--limit",
                "50",
            ])
            .current_dir(repo_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<Vec<super::WorkflowRun>>(&json_str) {
                    Ok(runs) => Ok(runs),
                    Err(_) => Ok(Vec::new()),
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Fetches GitHub releases using the gh CLI tool.
    /// Returns an empty Vec if gh is not installed or this is not a GitHub repo.
    pub fn releases(&self) -> Result<Vec<super::ReleaseInfo>> {
        let repo_dir = self.repo.path().parent().unwrap_or(self.repo.path());

        let output = std::process::Command::new("gh")
            .args([
                "release",
                "list",
                "--json",
                "tagName,name,publishedAt,isDraft,isPrerelease",
                "--limit",
                "50",
            ])
            .current_dir(repo_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<Vec<super::ReleaseInfo>>(&json_str) {
                    Ok(releases) => Ok(releases),
                    Err(_) => Ok(Vec::new()),
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    pub fn fetch(&self, remote_name: &str) -> Result<()> {
        let output = std::process::Command::new("git")
            .args(["fetch", remote_name])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }
        Ok(())
    }


    /// Push current branch to remote using git command
    pub fn push(&self, remote_name: &str) -> Result<()> {
        let output = std::process::Command::new("git")
            .args(["push", remote_name])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }
        Ok(())
    }

    /// Push a specific branch to remote
    pub fn push_branch(&self, remote_name: &str, branch: &str) -> Result<()> {
        let output = std::process::Command::new("git")
            .args(["push", remote_name, branch])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }
        Ok(())
    }

    /// Push with upstream tracking (-u flag)
    pub fn push_set_upstream(&self, remote_name: &str, branch: &str) -> Result<()> {
        let output = std::process::Command::new("git")
            .args(["push", "-u", remote_name, branch])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }
        Ok(())
    }

    pub fn fetch_branch(&self, remote_name: &str, branch: &str) -> Result<()> {
        let output = std::process::Command::new("git")
            .args(["fetch", remote_name, branch])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }
        Ok(())
    }

    pub fn pull(&self, remote_name: &str) -> Result<MergeResult> {
        let output = std::process::Command::new("git")
            .args(["pull", remote_name])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            // Check for conflict
            if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") {
                return Ok(MergeResult::Conflict);
            }
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }

        // Parse output to determine result
        if stdout.contains("Already up to date") || stdout.contains("Already up-to-date") {
            Ok(MergeResult::UpToDate)
        } else if stdout.contains("Fast-forward") {
            Ok(MergeResult::FastForward)
        } else {
            Ok(MergeResult::Merged)
        }
    }

    pub fn pull_branch(&self, remote_name: &str, branch: &str) -> Result<MergeResult> {
        let output = std::process::Command::new("git")
            .args(["pull", remote_name, branch])
            .current_dir(&self.path)
            .output()
            .map_err(|e| Error::Io(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            // Check for conflict
            if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") {
                return Ok(MergeResult::Conflict);
            }
            return Err(Error::Git(git2::Error::from_str(&stderr)));
        }

        // Parse output to determine result
        if stdout.contains("Already up to date") || stdout.contains("Already up-to-date") {
            Ok(MergeResult::UpToDate)
        } else if stdout.contains("Fast-forward") {
            Ok(MergeResult::FastForward)
        } else {
            Ok(MergeResult::Merged)
        }
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
        let main_head = self
            .head_name()?
            .unwrap_or_else(|| self.head_commit_short().unwrap_or_default());
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

    // Read file content
    pub fn read_file_content(&self, path: &str) -> Result<String> {
        let file_path = self.path.join(path);
        let content = std::fs::read_to_string(&file_path)?;
        Ok(content)
    }

    // File tree operations - lazy loading (one level at a time)
    pub fn file_tree(&self, show_ignored: bool) -> Result<Vec<FileTreeEntry>> {
        self.file_tree_dir("", show_ignored)
    }

    // Load a single directory level (for lazy loading)
    pub fn file_tree_dir(
        &self,
        relative_path: &str,
        show_ignored: bool,
    ) -> Result<Vec<FileTreeEntry>> {
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

        let mut entries = Vec::new();
        let full_path = if relative_path.is_empty() {
            self.path.clone()
        } else {
            self.path.join(relative_path)
        };

        if let Ok(read_dir) = std::fs::read_dir(&full_path) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Always skip .git directory
                if name == ".git" {
                    continue;
                }

                let entry_relative = if relative_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", relative_path, name)
                };

                // Check if the file/directory is ignored by git
                let is_ignored = self.repo.is_path_ignored(&entry_relative).unwrap_or(false);

                // Skip ignored files unless show_ignored is true
                if is_ignored && !show_ignored {
                    continue;
                }

                if entry.path().is_dir() {
                    // Don't recurse - just create the directory entry
                    // Children will be loaded lazily when expanded
                    let mut dir_entry = FileTreeEntry::new_dir(name, entry_relative.clone());
                    if is_ignored {
                        dir_entry.status = Some(FileTreeStatus::Ignored);
                    } else {
                        // Check if any file under this directory has git status
                        // by looking for paths that start with this directory
                        let dir_prefix = format!("{}/", entry_relative);
                        let dir_status = self.get_dir_status(&status_map, &dir_prefix);
                        dir_entry.status = dir_status;
                    }
                    // Check if directory has children (for icon display)
                    if let Ok(mut sub_read) = std::fs::read_dir(entry.path()) {
                        dir_entry.children = if sub_read.next().is_some() {
                            // Has at least one child - use placeholder
                            vec![FileTreeEntry::new_file(
                                "...".to_string(),
                                "".to_string(),
                                None,
                            )]
                        } else {
                            Vec::new()
                        };
                    }
                    entries.push(dir_entry);
                } else {
                    let status = if is_ignored {
                        Some(FileTreeStatus::Ignored)
                    } else {
                        status_map.get(&entry_relative).copied()
                    };
                    entries.push(FileTreeEntry::new_file(name, entry_relative, status));
                }
            }
        }

        // Sort: directories first, then files, alphabetically
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    // Get aggregated status for a directory based on files inside it
    fn get_dir_status(
        &self,
        status_map: &std::collections::HashMap<String, FileTreeStatus>,
        dir_prefix: &str,
    ) -> Option<FileTreeStatus> {
        let mut has_modified = false;
        let mut has_added = false;
        let mut has_deleted = false;
        let mut has_untracked = false;

        for (path, status) in status_map {
            if path.starts_with(dir_prefix) {
                match status {
                    FileTreeStatus::Modified => has_modified = true,
                    FileTreeStatus::Added => has_added = true,
                    FileTreeStatus::Deleted => has_deleted = true,
                    FileTreeStatus::Untracked => has_untracked = true,
                    FileTreeStatus::Ignored => {}
                }
            }
        }

        // Priority: Modified > Added > Deleted > Untracked
        if has_modified {
            Some(FileTreeStatus::Modified)
        } else if has_added {
            Some(FileTreeStatus::Added)
        } else if has_deleted {
            Some(FileTreeStatus::Deleted)
        } else if has_untracked {
            Some(FileTreeStatus::Untracked)
        } else {
            None
        }
    }


    /// Get the set of file paths changed in the given commits
    pub fn files_changed_in_commits(&self, commit_ids: &[String]) -> Result<std::collections::HashSet<String>> {
        let mut files = std::collections::HashSet::new();
        
        for commit_id in commit_ids {
            if let Ok(obj) = self.repo.revparse_single(commit_id) {
                if let Ok(commit) = obj.peel_to_commit() {
                    // Compare with parent(s)
                    let parent_tree = if commit.parent_count() > 0 {
                        commit.parent(0).ok().and_then(|p| p.tree().ok())
                    } else {
                        None
                    };
                    
                    if let Ok(tree) = commit.tree() {
                        if let Ok(diff) = self.repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
                            for delta in diff.deltas() {
                                if let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                                    files.insert(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }

    // Conflict operations
    pub fn conflicts(&self) -> Result<Vec<ConflictEntry>> {
        let index = self.repo.index()?;
        let mut conflicts = Vec::new();

        // Note: conflict detection is done below

        // Alternative: check for conflict markers in files
        for entry in index.conflicts()? {
            if let Ok(conflict) = entry {
                let path = conflict
                    .our
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

    // Log graph operations - use git command for proper graph rendering
    pub fn log_graph(&self, max_count: usize) -> Result<Vec<GraphLine>> {
        use std::process::Command;

        // Use git log --graph for proper ASCII art
        let output = Command::new("git")
            .args([
                "log",
                "--graph",
                "--all",
                "--format=%H|%h|%s|%an|%at|%P|%D",
                &format!("-{}", max_count),
            ])
            .current_dir(&self.path)
            .output()?;

        if !output.status.success() {
            return self.log_graph_simple(max_count);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = Vec::new();

        for line in stdout.lines() {
            let chars: Vec<char> = line.chars().collect();
            let mut graph_end: Option<usize> = None;
            
            // Find the first occurrence of 40 hex characters (commit hash)
            for i in 0..chars.len().saturating_sub(40) {
                let potential_hash: String = chars[i..i+40].iter().collect();
                if potential_hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    graph_end = Some(i);
                    break;
                }
            }

            match graph_end {
                None => {
                    // This is a connector-only line (|\, |/, etc.)
                    lines.push(GraphLine::Connector(line.to_string()));
                }
                Some(graph_end) => {
                    let mut graph_chars: String = chars[..graph_end].iter().collect::<String>().trim_end().to_string();
                    
                    // Ensure there's a space before the commit hash when graph ends with | or similar
                    // e.g., "* |" should stay as "* |" not "* |" -> "* | " for proper spacing
                    if !graph_chars.is_empty() {
                        let last_char = graph_chars.chars().last().unwrap();
                        if last_char != '*' && last_char != ' ' {
                            // Insert space after * if pattern is like "* |"
                            // Find the * position and ensure space after it
                            if let Some(star_pos) = graph_chars.rfind('*') {
                                let after_star = &graph_chars[star_pos + 1..];
                                if !after_star.starts_with(' ') {
                                    // Insert space after *
                                    let before = &graph_chars[..star_pos + 1];
                                    let after = &graph_chars[star_pos + 1..];
                                    graph_chars = format!("{} {}", before, after);
                                }
                            }
                        }
                    }
                    
                    let data_part: String = chars[graph_end..].iter().collect();
                    
                    let parts: Vec<&str> = data_part.split('|').collect();
                    if parts.len() < 7 {
                        continue;
                    }

                    let id = parts[0].to_string();
                    let short_id = parts[1].to_string();
                    let message = parts[2].to_string();
                    let author = parts[3].to_string();
                    let time: i64 = parts[4].parse().unwrap_or(0);
                    let parents: Vec<String> = parts[5]
                        .split_whitespace()
                        .map(|s| s[..7.min(s.len())].to_string())
                        .collect();
                    let refs: Vec<String> = if parts[6].is_empty() {
                        Vec::new()
                    } else {
                        parts[6]
                            .split(", ")
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    };

                    lines.push(GraphLine::Commit(GraphCommit {
                        id,
                        short_id,
                        message,
                        author,
                        time,
                        parents,
                        graph_chars,
                        refs,
                    }));
                }
            }
        }

        // Identify leaf commits (commits that are not parents of any other commit)
        // and insert blank lines before them (except for the first commit)
        let parent_ids: std::collections::HashSet<String> = lines
            .iter()
            .filter_map(|line| {
                if let GraphLine::Commit(c) = line {
                    Some(c.parents.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        let mut result = Vec::new();
        let mut is_first_commit = true;
        
        for line in lines {
            match &line {
                GraphLine::Commit(commit) => {
                    // Check if this is a leaf commit (not a parent of any other commit)
                    let short_id = &commit.id[..7.min(commit.id.len())];
                    let is_leaf = !parent_ids.contains(short_id) && !parent_ids.contains(&commit.id);
                    
                    // Add blank line before leaf commits (except the first commit)
                    if is_leaf && !is_first_commit {
                        // Create a connector line preserving | but replacing * with space
                        let blank_graph: String = commit.graph_chars
                            .chars()
                            .map(|c| match c {
                                '|' => '|',  // Keep vertical lines
                                _ => ' ',    // Replace *, \, /, etc. with space
                            })
                            .collect();
                        result.push(GraphLine::Connector(blank_graph));
                    }
                    
                    result.push(line);
                    is_first_commit = false;
                }
                GraphLine::Connector(_) => {
                    result.push(line);
                }
            }
        }

        Ok(result)
    }


    // Fallback graph implementation using git2
    fn log_graph_simple(&self, max_count: usize) -> Result<Vec<GraphLine>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        for branch in self.repo.branches(None)? {
            if let Ok((branch, _)) = branch {
                if let Some(oid) = branch.get().target() {
                    let _ = revwalk.push(oid);
                }
            }
        }

        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;

        let mut ref_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for branch in self.repo.branches(None)? {
            if let Ok((branch, _)) = branch {
                if let Some(oid) = branch.get().target() {
                    let name = branch.name().ok().flatten().unwrap_or("").to_string();
                    ref_map.entry(oid.to_string()).or_default().push(name);
                }
            }
        }

        for tag_name in self.repo.tag_names(None)?.iter().flatten() {
            let refname = format!("refs/tags/{}", tag_name);
            if let Ok(reference) = self.repo.find_reference(&refname) {
                if let Some(oid) = reference.target() {
                    ref_map
                        .entry(oid.to_string())
                        .or_default()
                        .push(format!("tag: {}", tag_name));
                }
            }
        }

        let mut commits = Vec::new();
        let mut graph_state: Vec<Option<git2::Oid>> = Vec::new();

        for oid in revwalk.take(max_count) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let graph_chars = self.build_graph_line(&commit, &mut graph_state);

            let parents: Vec<String> = commit
                .parents()
                .map(|p| p.id().to_string()[..7].to_string())
                .collect();

            let refs = ref_map.get(&oid.to_string()).cloned().unwrap_or_default();

            commits.push(GraphLine::Commit(GraphCommit {
                id: oid.to_string(),
                short_id: oid.to_string()[..7].to_string(),
                message: commit.summary().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                time: commit.time().seconds(),
                parents,
                graph_chars,
                refs,
            }));
        }

        Ok(commits)
    }

    fn build_graph_line(
        &self,
        commit: &git2::Commit,
        state: &mut Vec<Option<git2::Oid>>,
    ) -> String {
        let oid = commit.id();
        let parent_count = commit.parent_count();

        // Find this commit in state (it might be a parent we were tracking)
        let existing_col = state.iter().position(|s| *s == Some(oid));

        // Determine column for this commit
        let col = if let Some(c) = existing_col {
            c
        } else {
            // New branch - find empty slot or add new
            if let Some(pos) = state.iter().position(|s| s.is_none()) {
                pos
            } else {
                state.push(None);
                state.len() - 1
            }
        };

        // Check if other columns are also pointing to this commit (merge-base)
        let merging_cols: Vec<usize> = state
            .iter()
            .enumerate()
            .filter(|(i, s)| *i != col && **s == Some(oid))
            .map(|(i, _)| i)
            .collect();

        let mut chars = String::new();
        let state_len = state.len().max(col + 1);

        // Ensure state has enough slots
        while state.len() < state_len {
            state.push(None);
        }

        // Build the commit line
        for i in 0..state_len {
            let slot = &state[i];
            
            if i == col {
                chars.push('*');
            } else if merging_cols.contains(&i) {
                // Branch ends here (merge-base) - just show vertical line
                chars.push('|');
            } else if slot.is_some() {
                chars.push('|');
            } else {
                chars.push(' ');
            }
            chars.push(' ');
        }

        // Clear merging columns
        for mc in &merging_cols {
            state[*mc] = None;
        }

        // Update state for parents
        if parent_count == 0 {
            state[col] = None;
        } else if parent_count == 1 {
            state[col] = Some(commit.parent_id(0).unwrap());
        } else {
            // Merge commit
            state[col] = Some(commit.parent_id(0).unwrap());
            for i in 1..parent_count {
                if let Ok(parent_id) = commit.parent_id(i) {
                    if !state.iter().any(|s| *s == Some(parent_id)) {
                        if let Some(pos) = state.iter().position(|s| s.is_none()) {
                            state[pos] = Some(parent_id);
                        } else {
                            state.push(Some(parent_id));
                        }
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
