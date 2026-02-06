use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestAuthor {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestInfo {
    pub number: u32,
    pub title: String,
    pub author: PullRequestAuthor,
    pub state: String, // "OPEN", "CLOSED", "MERGED"
    pub created_at: String,
    pub base_ref_name: String,
    pub head_ref_name: String,
    pub additions: u32,
    pub deletions: u32,
    pub is_draft: bool,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub mergeable: String, // "MERGEABLE", "CONFLICTING", "UNKNOWN"
}
