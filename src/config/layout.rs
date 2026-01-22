use crate::views::PanelType;
use std::collections::HashMap;

/// A column in the layout
#[derive(Debug, Clone)]
pub struct Column {
    pub width: f32,  // Width as percentage (0.0 - 1.0)
    pub panels: Vec<PanelHeight>,
}

/// A panel within a column with its height
#[derive(Debug, Clone)]
pub struct PanelHeight {
    pub panel: PanelType,
    pub height: f32,  // Height as percentage within column (0.0 - 1.0)
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub columns: Vec<Column>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        // Default layout: 3 columns
        // Left (25%): Status, Branches, Stash
        // Middle (25%): Commits, Tags, Remotes
        // Right (50%): Diff, LogGraph
        Self {
            columns: vec![
                Column {
                    width: 0.25,
                    panels: vec![
                        PanelHeight { panel: PanelType::Status, height: 0.40 },
                        PanelHeight { panel: PanelType::Branches, height: 0.30 },
                        PanelHeight { panel: PanelType::Stash, height: 0.30 },
                    ],
                },
                Column {
                    width: 0.25,
                    panels: vec![
                        PanelHeight { panel: PanelType::Commits, height: 0.50 },
                        PanelHeight { panel: PanelType::Tags, height: 0.25 },
                        PanelHeight { panel: PanelType::Remotes, height: 0.25 },
                    ],
                },
                Column {
                    width: 0.50,
                    panels: vec![
                        PanelHeight { panel: PanelType::Diff, height: 0.65 },
                        PanelHeight { panel: PanelType::LogGraph, height: 0.35 },
                    ],
                },
            ],
        }
    }
}

impl LayoutConfig {
    pub fn from_toml(value: &HashMap<String, super::parser::Value>) -> Self {
        use super::parser::Value;

        let mut config = Self::default();

        if let Some(Value::Array(columns_arr)) = value.get("columns") {
            let mut columns = Vec::new();

            for col_val in columns_arr {
                if let Value::Table(col_table) = col_val {
                    let width = col_table.get("width")
                        .and_then(|v| match v {
                            Value::Float(f) => Some(*f as f32),
                            Value::Integer(i) => Some(*i as f32),
                            _ => None,
                        })
                        .unwrap_or(0.25);

                    let mut panels = Vec::new();

                    if let Some(Value::Array(panels_arr)) = col_table.get("panels") {
                        for panel_val in panels_arr {
                            if let Value::Table(panel_table) = panel_val {
                                let panel_type = panel_table.get("type")
                                    .and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None })
                                    .and_then(Self::parse_panel_type);

                                let height = panel_table.get("height")
                                    .and_then(|v| match v {
                                        Value::Float(f) => Some(*f as f32),
                                        Value::Integer(i) => Some(*i as f32),
                                        _ => None,
                                    })
                                    .unwrap_or(0.25);

                                if let Some(pt) = panel_type {
                                    panels.push(PanelHeight { panel: pt, height });
                                }
                            }
                        }
                    }

                    if !panels.is_empty() {
                        columns.push(Column { width, panels });
                    }
                }
            }

            if !columns.is_empty() {
                config.columns = columns;
            }
        }

        config
    }

    fn parse_panel_type(s: &str) -> Option<PanelType> {
        match s.to_lowercase().as_str() {
            "status" => Some(PanelType::Status),
            "branches" => Some(PanelType::Branches),
            "commits" => Some(PanelType::Commits),
            "stash" => Some(PanelType::Stash),
            "diff" => Some(PanelType::Diff),
            "tags" => Some(PanelType::Tags),
            "remotes" => Some(PanelType::Remotes),
            "worktrees" => Some(PanelType::Worktrees),
            "submodules" => Some(PanelType::Submodules),
            "blame" => Some(PanelType::Blame),
            "files" => Some(PanelType::Files),
            "conflicts" => Some(PanelType::Conflicts),
            "loggraph" | "log_graph" | "graph" => Some(PanelType::LogGraph),
            _ => None,
        }
    }

    pub fn all_panels(&self) -> Vec<PanelType> {
        self.columns.iter()
            .flat_map(|col| col.panels.iter().map(|p| p.panel))
            .collect()
    }

    /// Find the column and panel index for a given panel type
    pub fn find_panel(&self, panel: PanelType) -> Option<(usize, usize)> {
        for (col_idx, col) in self.columns.iter().enumerate() {
            for (panel_idx, p) in col.panels.iter().enumerate() {
                if p.panel == panel {
                    return Some((col_idx, panel_idx));
                }
            }
        }
        None
    }

    /// Get the panel above the given panel (in the same column)
    pub fn panel_above(&self, panel: PanelType) -> Option<PanelType> {
        if let Some((col_idx, panel_idx)) = self.find_panel(panel) {
            if panel_idx > 0 {
                return Some(self.columns[col_idx].panels[panel_idx - 1].panel);
            }
        }
        None
    }

    /// Get the panel below the given panel (in the same column)
    pub fn panel_below(&self, panel: PanelType) -> Option<PanelType> {
        if let Some((col_idx, panel_idx)) = self.find_panel(panel) {
            let col = &self.columns[col_idx];
            if panel_idx + 1 < col.panels.len() {
                return Some(col.panels[panel_idx + 1].panel);
            }
        }
        None
    }

    /// Get a panel in the column to the left (at similar vertical position)
    pub fn panel_left(&self, panel: PanelType) -> Option<PanelType> {
        if let Some((col_idx, panel_idx)) = self.find_panel(panel) {
            if col_idx > 0 {
                let left_col = &self.columns[col_idx - 1];
                // Try to find panel at similar position, or just take the first one
                let target_idx = panel_idx.min(left_col.panels.len() - 1);
                return Some(left_col.panels[target_idx].panel);
            }
        }
        None
    }

    /// Get a panel in the column to the right (at similar vertical position)
    pub fn panel_right(&self, panel: PanelType) -> Option<PanelType> {
        if let Some((col_idx, panel_idx)) = self.find_panel(panel) {
            if col_idx + 1 < self.columns.len() {
                let right_col = &self.columns[col_idx + 1];
                let target_idx = panel_idx.min(right_col.panels.len() - 1);
                return Some(right_col.panels[target_idx].panel);
            }
        }
        None
    }
}
