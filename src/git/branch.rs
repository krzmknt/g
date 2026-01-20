use super::commit::CommitInfo;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub branch_type: BranchType,
    pub is_head: bool,
    pub last_commit: CommitInfo,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchType {
    Local,
    Remote,
}
