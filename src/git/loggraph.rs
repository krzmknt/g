#[derive(Debug, Clone)]
pub struct GraphCommit {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub parents: Vec<String>,
    pub graph_chars: String,  // ASCII art for graph visualization
    pub refs: Vec<String>,    // Branch/tag names pointing to this commit
}
