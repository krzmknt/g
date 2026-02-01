use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    #[serde(default)]
    pub database_id: u64,
    pub name: String,
    pub head_branch: String,
    pub status: String,             // "completed", "in_progress", "queued"
    pub conclusion: Option<String>, // "success", "failure", "cancelled", etc.
    pub created_at: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    pub display_title: String,
    #[serde(default)]
    pub event: String,              // "push", "pull_request", "schedule", etc.
    #[serde(default)]
    pub workflow_name: String,
    #[serde(default)]
    pub url: String,
}
