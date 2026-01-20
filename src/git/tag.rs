#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub message: Option<String>,
    pub target: String,
    pub is_annotated: bool,
}
