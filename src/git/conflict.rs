#[derive(Debug, Clone)]
pub struct ConflictEntry {
    pub path: String,
    pub conflict_type: ConflictType,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub ancestor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    BothModified,
    BothAdded,
    DeletedByUs,
    DeletedByThem,
    AddedByUs,
    AddedByThem,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::BothModified => write!(f, "both modified"),
            ConflictType::BothAdded => write!(f, "both added"),
            ConflictType::DeletedByUs => write!(f, "deleted by us"),
            ConflictType::DeletedByThem => write!(f, "deleted by them"),
            ConflictType::AddedByUs => write!(f, "added by us"),
            ConflictType::AddedByThem => write!(f, "added by them"),
        }
    }
}
