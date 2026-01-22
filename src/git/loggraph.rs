#[derive(Debug, Clone)]
pub struct GraphCommit {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub time: i64,  // Unix timestamp for relative time calculation
    pub parents: Vec<String>,
    pub graph_chars: String,  // ASCII art for graph visualization
    pub refs: Vec<String>,    // Branch/tag names pointing to this commit
}

impl GraphCommit {
    pub fn relative_time(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let diff = now - self.time;

        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            let mins = diff / 60;
            format!("{} min ago", mins)
        } else if diff < 86400 {
            let hours = diff / 3600;
            format!("{} hour ago", hours)
        } else if diff < 604800 {
            let days = diff / 86400;
            format!("{} day ago", days)
        } else if diff < 2592000 {
            let weeks = diff / 604800;
            format!("{} week ago", weeks)
        } else if diff < 31536000 {
            let months = diff / 2592000;
            format!("{} month ago", months)
        } else {
            let years = diff / 31536000;
            format!("{} year ago", years)
        }
    }
}
