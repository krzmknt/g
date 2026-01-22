#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub status: Option<FileTreeStatus>,
    pub children: Vec<FileTreeEntry>,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTreeStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Ignored,
}

impl FileTreeEntry {
    pub fn new_file(name: String, path: String, status: Option<FileTreeStatus>) -> Self {
        Self {
            name,
            path,
            is_dir: false,
            status,
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn new_dir(name: String, path: String) -> Self {
        Self {
            name,
            path,
            is_dir: true,
            status: None,
            children: Vec::new(),
            expanded: false,
        }
    }
}
