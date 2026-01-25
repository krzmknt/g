use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    pub name: String,
    pub head_branch: String,
    pub status: String,             // "completed", "in_progress", "queued"
    pub conclusion: Option<String>, // "success", "failure", "cancelled", etc.
    pub created_at: String,
    pub display_title: String,
    #[serde(default)]
    pub workflow_name: String,
}
