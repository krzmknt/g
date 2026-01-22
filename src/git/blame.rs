#[derive(Debug, Clone)]
pub struct BlameInfo {
    pub path: String,
    pub lines: Vec<BlameLine>,
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub date: String,
    pub content: String,
}
