use super::commit::CommitInfo;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub branch_type: BranchType,
    pub is_head: bool,
    pub last_commit: CommitInfo,
    pub ahead: usize,
    pub behind: usize,
    /// Upstream tracking branch name (e.g., "origin/feature/hoge")
    pub upstream: Option<UpstreamInfo>,
}

#[derive(Debug, Clone)]
pub struct UpstreamInfo {
    pub name: String,
    pub short_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchType {
    Local,
    Remote,
}
