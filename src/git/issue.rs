use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IssueAuthor {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueLabel {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueInfo {
    pub number: u32,
    pub title: String,
    pub author: IssueAuthor,
    pub state: String, // "OPEN", "CLOSED"
    pub created_at: String,
    #[serde(default)]
    pub labels: Vec<IssueLabel>,
    #[serde(default)]
    pub comments: u32,
}
