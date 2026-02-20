use crate::views::PanelType;
use std::collections::HashMap;

/// A column in the layout
#[derive(Debug, Clone)]
pub struct Column {
    pub width: f32, // Width as percentage (0.0 - 1.0)
    pub panels: Vec<PanelHeight>,
}

/// A panel within a column with its height
#[derive(Debug, Clone)]
pub struct PanelHeight {
    pub panel: PanelType,
    pub height: f32, // Height as percentage within column (0.0 - 1.0)
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub columns: Vec<Column>,
}

pub const LAYOUT_PRESET_NAMES: &[&str] = &["Standard", "Compact", "Review", "Diff Only"];

impl Default for LayoutConfig {
    fn default() -> Self {
        // Default layout: 3 columns
        // Left (20%): Files, Status, Branches, Stash, Tags, Worktrees, Submodules, Remotes
        // Middle (30%): Commits, PullRequests, Issues, Actions, Releases, Conflicts
        // Right (50%): Diff
        Self {
            columns: vec![
                Column {
                    width: 0.20,
                    panels: vec![
                        PanelHeight {
                            panel: PanelType::Files,
                            height: 0.20,
                        },
                        PanelHeight {
                            panel: PanelType::Status,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Branches,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Stash,
                            height: 0.10,
                        },
                        PanelHeight {
                            panel: PanelType::Tags,
                            height: 0.10,
                        },
                        PanelHeight {
                            panel: PanelType::Worktrees,
                            height: 0.10,
                        },
                        PanelHeight {
                            panel: PanelType::Submodules,
                            height: 0.10,
                        },
                        PanelHeight {
                            panel: PanelType::Remotes,
                            height: 0.10,
                        },
                    ],
                },
                Column {
                    width: 0.30,
                    panels: vec![
                        PanelHeight {
                            panel: PanelType::Commits,
                            height: 0.30,
                        },
                        PanelHeight {
                            panel: PanelType::PullRequests,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Issues,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Actions,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Releases,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Conflicts,
                            height: 0.10,
                        },
                    ],
                },
                Column {
                    width: 0.50,
                    panels: vec![PanelHeight {
                        panel: PanelType::Diff,
                        height: 1.0,
                    }],
                },
            ],
        }
    }
}

impl LayoutConfig {
    pub fn from_toml(value: &HashMap<String, super::parser::Value>) -> Self {
        Self::parse_columns_array(value.get("columns"))
    }

    /// Parse all 4 layout presets from TOML.
    /// Reads `layout_preset_0` through `layout_preset_3`.
    /// Falls back to `columns` for preset 0 if no `layout_preset_0` exists.
    /// Falls back to hardcoded `preset(i)` for any missing preset.
    pub fn presets_from_toml(toml: &HashMap<String, super::parser::Value>) -> [LayoutConfig; 4] {
        let has_preset_0 = toml.contains_key("layout_preset_0");

        let preset_0 = if has_preset_0 {
            Self::parse_columns_array(toml.get("layout_preset_0"))
        } else if toml.contains_key("columns") {
            Self::parse_columns_array(toml.get("columns"))
        } else {
            Self::preset(0).unwrap()
        };

        let preset_1 = if toml.contains_key("layout_preset_1") {
            Self::parse_columns_array(toml.get("layout_preset_1"))
        } else {
            Self::preset(1).unwrap()
        };

        let preset_2 = if toml.contains_key("layout_preset_2") {
            Self::parse_columns_array(toml.get("layout_preset_2"))
        } else {
            Self::preset(2).unwrap()
        };

        let preset_3 = if toml.contains_key("layout_preset_3") {
            Self::parse_columns_array(toml.get("layout_preset_3"))
        } else {
            Self::preset(3).unwrap()
        };

        [preset_0, preset_1, preset_2, preset_3]
    }

    /// Parse a LayoutConfig from an array of column tables (shared by from_toml and presets_from_toml).
    fn parse_columns_array(columns_value: Option<&super::parser::Value>) -> Self {
        use super::parser::Value;

        let mut config = Self::default();

        if let Some(Value::Array(columns_arr)) = columns_value {
            let mut columns = Vec::new();

            for col_val in columns_arr {
                if let Value::Table(col_table) = col_val {
                    let width = col_table
                        .get("width")
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
                                let panel_type = panel_table
                                    .get("type")
                                    .and_then(|v| {
                                        if let Value::String(s) = v {
                                            Some(s.as_str())
                                        } else {
                                            None
                                        }
                                    })
                                    .and_then(Self::parse_panel_type);

                                let height = panel_table
                                    .get("height")
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
            "pullrequests" | "prs" => Some(PanelType::PullRequests),
            "issues" => Some(PanelType::Issues),
            "actions" => Some(PanelType::Actions),
            "releases" => Some(PanelType::Releases),
            _ => None,
        }
    }

    pub fn all_panels(&self) -> Vec<PanelType> {
        self.columns
            .iter()
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

    /// Get a layout preset by index (0-based).
    pub fn preset(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::preset_standard()),
            1 => Some(Self::preset_compact()),
            2 => Some(Self::preset_review()),
            3 => Some(Self::preset_diff_only()),
            _ => None,
        }
    }

    /// Get a layout preset with its name. Returns `(name, layout)`.
    pub fn apply_preset(index: usize) -> Option<(&'static str, Self)> {
        LAYOUT_PRESET_NAMES
            .get(index)
            .and_then(|name| Self::preset(index).map(|layout| (*name, layout)))
    }

    /// Preset 1: Standard 3-column layout (same as default)
    fn preset_standard() -> Self {
        Self::default()
    }

    /// Preset 2: Compact 2-column layout
    fn preset_compact() -> Self {
        Self {
            columns: vec![
                Column {
                    width: 0.25,
                    panels: vec![
                        PanelHeight {
                            panel: PanelType::Files,
                            height: 0.25,
                        },
                        PanelHeight {
                            panel: PanelType::Status,
                            height: 0.20,
                        },
                        PanelHeight {
                            panel: PanelType::Branches,
                            height: 0.20,
                        },
                        PanelHeight {
                            panel: PanelType::Stash,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Tags,
                            height: 0.20,
                        },
                    ],
                },
                Column {
                    width: 0.75,
                    panels: vec![PanelHeight {
                        panel: PanelType::Diff,
                        height: 1.0,
                    }],
                },
            ],
        }
    }

    /// Preset 3: Review 2-column layout
    fn preset_review() -> Self {
        Self {
            columns: vec![
                Column {
                    width: 0.35,
                    panels: vec![
                        PanelHeight {
                            panel: PanelType::Commits,
                            height: 0.30,
                        },
                        PanelHeight {
                            panel: PanelType::PullRequests,
                            height: 0.20,
                        },
                        PanelHeight {
                            panel: PanelType::Issues,
                            height: 0.20,
                        },
                        PanelHeight {
                            panel: PanelType::Branches,
                            height: 0.15,
                        },
                        PanelHeight {
                            panel: PanelType::Status,
                            height: 0.15,
                        },
                    ],
                },
                Column {
                    width: 0.65,
                    panels: vec![PanelHeight {
                        panel: PanelType::Diff,
                        height: 1.0,
                    }],
                },
            ],
        }
    }

    /// Preset 4: Diff-only 1-column layout
    fn preset_diff_only() -> Self {
        Self {
            columns: vec![Column {
                width: 1.0,
                panels: vec![PanelHeight {
                    panel: PanelType::Diff,
                    height: 1.0,
                }],
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_preset_names_count() {
        assert_eq!(LAYOUT_PRESET_NAMES.len(), 4);
    }

    #[test]
    fn test_layout_preset_names_values() {
        assert_eq!(LAYOUT_PRESET_NAMES[0], "Standard");
        assert_eq!(LAYOUT_PRESET_NAMES[1], "Compact");
        assert_eq!(LAYOUT_PRESET_NAMES[2], "Review");
        assert_eq!(LAYOUT_PRESET_NAMES[3], "Diff Only");
    }

    #[test]
    fn test_preset_valid_indices() {
        for i in 0..4 {
            assert!(
                LayoutConfig::preset(i).is_some(),
                "preset({}) should be Some",
                i
            );
        }
    }

    #[test]
    fn test_preset_invalid_index() {
        assert!(LayoutConfig::preset(4).is_none());
        assert!(LayoutConfig::preset(99).is_none());
    }

    #[test]
    fn test_preset_standard_is_3_columns() {
        let layout = LayoutConfig::preset(0).unwrap();
        assert_eq!(layout.columns.len(), 3);
    }

    #[test]
    fn test_preset_compact_is_2_columns() {
        let layout = LayoutConfig::preset(1).unwrap();
        assert_eq!(layout.columns.len(), 2);
    }

    #[test]
    fn test_preset_review_is_2_columns() {
        let layout = LayoutConfig::preset(2).unwrap();
        assert_eq!(layout.columns.len(), 2);
    }

    #[test]
    fn test_preset_diff_only_is_1_column() {
        let layout = LayoutConfig::preset(3).unwrap();
        assert_eq!(layout.columns.len(), 1);
    }

    #[test]
    fn test_preset_widths_sum_to_1() {
        for i in 0..4 {
            let layout = LayoutConfig::preset(i).unwrap();
            let total: f32 = layout.columns.iter().map(|c| c.width).sum();
            assert!(
                (total - 1.0).abs() < 0.01,
                "preset {} widths sum to {} (expected 1.0)",
                i,
                total
            );
        }
    }

    #[test]
    fn test_preset_heights_sum_to_1_per_column() {
        for i in 0..4 {
            let layout = LayoutConfig::preset(i).unwrap();
            for (col_idx, col) in layout.columns.iter().enumerate() {
                let total: f32 = col.panels.iter().map(|p| p.height).sum();
                assert!(
                    (total - 1.0).abs() < 0.01,
                    "preset {} column {} heights sum to {} (expected 1.0)",
                    i,
                    col_idx,
                    total
                );
            }
        }
    }

    #[test]
    fn test_preset_diff_only_contains_diff_panel() {
        let layout = LayoutConfig::preset(3).unwrap();
        assert_eq!(layout.columns[0].panels.len(), 1);
        assert_eq!(layout.columns[0].panels[0].panel, PanelType::Diff);
    }

    #[test]
    fn test_apply_preset_valid() {
        let result = LayoutConfig::apply_preset(0);
        assert!(result.is_some());
        let (name, layout) = result.unwrap();
        assert_eq!(name, "Standard");
        assert_eq!(layout.columns.len(), 3);
    }

    #[test]
    fn test_apply_preset_invalid() {
        assert!(LayoutConfig::apply_preset(4).is_none());
    }

    #[test]
    fn test_apply_preset_all_names_match() {
        for i in 0..4 {
            let (name, _) = LayoutConfig::apply_preset(i).unwrap();
            assert_eq!(name, LAYOUT_PRESET_NAMES[i]);
        }
    }

    #[test]
    fn test_preset_standard_matches_default() {
        let preset = LayoutConfig::preset(0).unwrap();
        let default = LayoutConfig::default();
        assert_eq!(preset.columns.len(), default.columns.len());
        for (i, (pc, dc)) in preset
            .columns
            .iter()
            .zip(default.columns.iter())
            .enumerate()
        {
            assert!(
                (pc.width - dc.width).abs() < 0.001,
                "column {} width mismatch",
                i
            );
            assert_eq!(
                pc.panels.len(),
                dc.panels.len(),
                "column {} panel count mismatch",
                i
            );
        }
    }

    #[test]
    fn test_preset_compact_has_diff_in_right_column() {
        let layout = LayoutConfig::preset(1).unwrap();
        let last_col = layout.columns.last().unwrap();
        assert_eq!(last_col.panels.len(), 1);
        assert_eq!(last_col.panels[0].panel, PanelType::Diff);
    }

    #[test]
    fn test_preset_review_has_diff_in_right_column() {
        let layout = LayoutConfig::preset(2).unwrap();
        let last_col = layout.columns.last().unwrap();
        assert_eq!(last_col.panels.len(), 1);
        assert_eq!(last_col.panels[0].panel, PanelType::Diff);
    }

    fn parse_toml(input: &str) -> HashMap<String, super::super::parser::Value> {
        super::super::parser::parse(input).unwrap()
    }

    #[test]
    fn test_presets_from_toml_all_four_presets() {
        let input = r#"
[[layout_preset_0]]
width = 0.200
panels = [
  { type = "files", height = 0.500 },
  { type = "status", height = 0.500 },
]

[[layout_preset_0]]
width = 0.800
panels = [
  { type = "diff", height = 1.000 },
]

[[layout_preset_1]]
width = 0.300
panels = [
  { type = "branches", height = 1.000 },
]

[[layout_preset_1]]
width = 0.700
panels = [
  { type = "diff", height = 1.000 },
]

[[layout_preset_2]]
width = 0.400
panels = [
  { type = "commits", height = 1.000 },
]

[[layout_preset_2]]
width = 0.600
panels = [
  { type = "diff", height = 1.000 },
]

[[layout_preset_3]]
width = 1.000
panels = [
  { type = "diff", height = 1.000 },
]
"#;
        let toml = parse_toml(input);
        let presets = LayoutConfig::presets_from_toml(&toml);

        // Preset 0: 2 columns
        assert_eq!(presets[0].columns.len(), 2);
        assert!((presets[0].columns[0].width - 0.200).abs() < 0.001);
        assert_eq!(presets[0].columns[0].panels.len(), 2);
        assert_eq!(presets[0].columns[0].panels[0].panel, PanelType::Files);

        // Preset 1: 2 columns
        assert_eq!(presets[1].columns.len(), 2);
        assert!((presets[1].columns[0].width - 0.300).abs() < 0.001);
        assert_eq!(presets[1].columns[0].panels[0].panel, PanelType::Branches);

        // Preset 2: 2 columns
        assert_eq!(presets[2].columns.len(), 2);
        assert!((presets[2].columns[0].width - 0.400).abs() < 0.001);
        assert_eq!(presets[2].columns[0].panels[0].panel, PanelType::Commits);

        // Preset 3: 1 column
        assert_eq!(presets[3].columns.len(), 1);
        assert!((presets[3].columns[0].width - 1.000).abs() < 0.001);
    }

    #[test]
    fn test_presets_from_toml_backward_compat_columns_as_preset_0() {
        let input = r#"
[[columns]]
width = 0.250
panels = [
  { type = "files", height = 1.000 },
]

[[columns]]
width = 0.750
panels = [
  { type = "diff", height = 1.000 },
]
"#;
        let toml = parse_toml(input);
        let presets = LayoutConfig::presets_from_toml(&toml);

        // Preset 0 should come from columns
        assert_eq!(presets[0].columns.len(), 2);
        assert!((presets[0].columns[0].width - 0.250).abs() < 0.001);
        assert_eq!(presets[0].columns[0].panels[0].panel, PanelType::Files);

        // Presets 1-3 should be hardcoded defaults
        let default_1 = LayoutConfig::preset(1).unwrap();
        assert_eq!(presets[1].columns.len(), default_1.columns.len());

        let default_2 = LayoutConfig::preset(2).unwrap();
        assert_eq!(presets[2].columns.len(), default_2.columns.len());

        let default_3 = LayoutConfig::preset(3).unwrap();
        assert_eq!(presets[3].columns.len(), default_3.columns.len());
    }

    #[test]
    fn test_presets_from_toml_missing_presets_use_defaults() {
        // Only preset 0 is defined, rest should fall back to hardcoded defaults
        let input = r#"
[[layout_preset_0]]
width = 1.000
panels = [
  { type = "diff", height = 1.000 },
]
"#;
        let toml = parse_toml(input);
        let presets = LayoutConfig::presets_from_toml(&toml);

        // Preset 0: from config
        assert_eq!(presets[0].columns.len(), 1);
        assert_eq!(presets[0].columns[0].panels[0].panel, PanelType::Diff);

        // Presets 1-3: hardcoded defaults
        let default_1 = LayoutConfig::preset(1).unwrap();
        assert_eq!(presets[1].columns.len(), default_1.columns.len());

        let default_2 = LayoutConfig::preset(2).unwrap();
        assert_eq!(presets[2].columns.len(), default_2.columns.len());

        let default_3 = LayoutConfig::preset(3).unwrap();
        assert_eq!(presets[3].columns.len(), default_3.columns.len());
    }

    #[test]
    fn test_presets_from_toml_empty_uses_all_defaults() {
        let toml = parse_toml("");
        let presets = LayoutConfig::presets_from_toml(&toml);

        for i in 0..4 {
            let default = LayoutConfig::preset(i).unwrap();
            assert_eq!(
                presets[i].columns.len(),
                default.columns.len(),
                "preset {} should match hardcoded default",
                i
            );
        }
    }

    #[test]
    fn test_presets_from_toml_layout_preset_takes_precedence_over_columns() {
        // Both layout_preset_0 and columns exist; layout_preset_0 should win
        let input = r#"
[[columns]]
width = 0.500
panels = [
  { type = "status", height = 1.000 },
]

[[columns]]
width = 0.500
panels = [
  { type = "diff", height = 1.000 },
]

[[layout_preset_0]]
width = 1.000
panels = [
  { type = "branches", height = 1.000 },
]
"#;
        let toml = parse_toml(input);
        let presets = LayoutConfig::presets_from_toml(&toml);

        // layout_preset_0 should win over columns
        assert_eq!(presets[0].columns.len(), 1);
        assert_eq!(presets[0].columns[0].panels[0].panel, PanelType::Branches);
    }
}
