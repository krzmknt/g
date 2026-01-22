#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: String,
    pub head: String,
    pub is_main: bool,
    pub is_locked: bool,
}
