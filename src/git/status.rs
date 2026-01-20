#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub staged: FileStatus,
    pub unstaged: FileStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
}

impl FileStatus {
    pub fn from_index_status(status: git2::Status) -> Self {
        if status.contains(git2::Status::INDEX_NEW) {
            FileStatus::Added
        } else if status.contains(git2::Status::INDEX_MODIFIED) {
            FileStatus::Modified
        } else if status.contains(git2::Status::INDEX_DELETED) {
            FileStatus::Deleted
        } else if status.contains(git2::Status::INDEX_RENAMED) {
            FileStatus::Renamed
        } else if status.contains(git2::Status::INDEX_TYPECHANGE) {
            FileStatus::Modified
        } else {
            FileStatus::Unmodified
        }
    }

    pub fn from_workdir_status(status: git2::Status) -> Self {
        if status.contains(git2::Status::WT_NEW) {
            FileStatus::Untracked
        } else if status.contains(git2::Status::WT_MODIFIED) {
            FileStatus::Modified
        } else if status.contains(git2::Status::WT_DELETED) {
            FileStatus::Deleted
        } else if status.contains(git2::Status::WT_RENAMED) {
            FileStatus::Renamed
        } else if status.contains(git2::Status::WT_TYPECHANGE) {
            FileStatus::Modified
        } else if status.contains(git2::Status::IGNORED) {
            FileStatus::Ignored
        } else {
            FileStatus::Unmodified
        }
    }

    pub fn symbol(&self) -> char {
        match self {
            FileStatus::Unmodified => ' ',
            FileStatus::Modified => 'M',
            FileStatus::Added => 'A',
            FileStatus::Deleted => 'D',
            FileStatus::Renamed => 'R',
            FileStatus::Copied => 'C',
            FileStatus::Untracked => '?',
            FileStatus::Ignored => '!',
        }
    }

    pub fn is_changed(&self) -> bool {
        !matches!(self, FileStatus::Unmodified | FileStatus::Ignored)
    }
}
