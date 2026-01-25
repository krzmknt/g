use super::branch::{BranchInfo, BranchType};

/// Represents a branch with its parent-child relationships
#[derive(Debug, Clone)]
pub struct BranchGraphEntry {
    pub branch: BranchInfo,
    /// Indices of child branches (branches that have all commits of this branch in their history)
    pub children: Vec<usize>,
    /// Index of parent branch (the branch whose commits are all reachable from this branch)
    pub parent: Option<usize>,
    /// Depth in the tree (0 for root branches)
    pub depth: usize,
    /// Graph prefix characters for visualization
    pub graph_prefix: String,
}

/// A tree-structured representation of branches
#[derive(Debug, Clone)]
pub struct BranchGraph {
    pub entries: Vec<BranchGraphEntry>,
}

impl BranchGraph {
    /// Build a branch graph from a list of branches
    /// parent_checker: closure that returns true if all commits of branch A are reachable from branch B
    pub fn build<F>(branches: Vec<BranchInfo>, is_ancestor: F) -> Self
    where
        F: Fn(&BranchInfo, &BranchInfo) -> bool,
    {
        if branches.is_empty() {
            return Self { entries: Vec::new() };
        }

        let n = branches.len();

        // Create entries without relationships
        let mut entries: Vec<BranchGraphEntry> = branches
            .into_iter()
            .map(|branch| BranchGraphEntry {
                branch,
                children: Vec::new(),
                parent: None,
                depth: 0,
                graph_prefix: String::new(),
            })
            .collect();

        // Build parent-child relationships
        // For each branch pair, check if one is an ancestor of the other
        // A is parent of B if: all commits of A are reachable from B (i.e., A is ancestor of B)
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                // Check if branch[i] is an ancestor of branch[j]
                // (all commits of i are reachable from j)
                if is_ancestor(&entries[i].branch, &entries[j].branch) {
                    // i is potentially a parent of j
                    // But we only want direct parent, not grandparent
                    // For now, record all potential relationships
                    entries[j].children.push(i); // temporarily store potential parents in children
                }
            }
        }

        // Find direct parent for each branch (closest ancestor)
        // The direct parent is the one with the most recent commit among all ancestors
        for i in 0..n {
            let potential_parents: Vec<usize> = entries[i].children.clone();
            entries[i].children.clear();

            if potential_parents.is_empty() {
                continue;
            }

            // Find the most recent ancestor (direct parent)
            let direct_parent = potential_parents
                .iter()
                .max_by_key(|&&idx| entries[idx].branch.last_commit.time)
                .copied();

            entries[i].parent = direct_parent;
        }

        // Build children lists from parent relationships
        for i in 0..n {
            if let Some(parent_idx) = entries[i].parent {
                entries[parent_idx].children.push(i);
            }
        }

        // Calculate depths
        fn calculate_depth(entries: &mut [BranchGraphEntry], idx: usize, depth: usize) {
            entries[idx].depth = depth;
            let children: Vec<usize> = entries[idx].children.clone();
            for child_idx in children {
                calculate_depth(entries, child_idx, depth + 1);
            }
        }

        // Find root branches (no parent) and calculate depths
        let roots: Vec<usize> = (0..n)
            .filter(|&i| entries[i].parent.is_none())
            .collect();

        for root_idx in roots {
            calculate_depth(&mut entries, root_idx, 0);
        }

        // Build graph prefixes for visualization
        Self::build_graph_prefixes(&mut entries);

        Self { entries }
    }

    fn build_graph_prefixes(entries: &mut [BranchGraphEntry]) {
        let n = entries.len();
        if n == 0 {
            return;
        }

        // Sort by commit time (most recent first)
        let mut display_order: Vec<usize> = (0..n).collect();
        display_order.sort_by(|&a, &b| {
            entries[b].branch.last_commit.time.cmp(&entries[a].branch.last_commit.time)
        });

        // Column assignment strategy:
        // - Process in display order (most recent first)
        // - Each branch tries to reuse its parent's column (FF relationship)
        // - If parent doesn't have a column yet, create one and share it with parent
        // - If parent already has a column taken by another child (diverged), create new column
        
        let mut branch_column: Vec<Option<usize>> = vec![None; n];
        let mut next_column: usize = 0;
        // Track which branch currently "owns" each column
        let mut column_owner: Vec<Option<usize>> = Vec::new();
        
        for &branch_idx in &display_order {
            let parent_idx = entries[branch_idx].parent;
            
            let my_col: usize;
            
            if let Some(p_idx) = parent_idx {
                if let Some(parent_col) = branch_column[p_idx] {
                    // Parent already has a column
                    // Check if parent still owns it (no other child took it)
                    if column_owner.get(parent_col) == Some(&Some(p_idx)) {
                        // Take over parent's column (FF from parent)
                        my_col = parent_col;
                    } else {
                        // Another sibling took the column - we diverge, need new column
                        my_col = next_column;
                        next_column += 1;
                    }
                } else {
                    // Parent doesn't have a column yet
                    // Create a column and share with parent (FF relationship)
                    my_col = next_column;
                    next_column += 1;
                    branch_column[p_idx] = Some(my_col);
                }
            } else {
                // Root branch - new column
                my_col = next_column;
                next_column += 1;
            }
            
            branch_column[branch_idx] = Some(my_col);
            
            // Update column ownership
            if my_col >= column_owner.len() {
                column_owner.resize(my_col + 1, None);
            }
            column_owner[my_col] = Some(branch_idx);
        }

        // Build prefixes - track active columns during display
        let max_columns = next_column;
        if max_columns == 0 {
            return;
        }
        
        let mut active_columns: Vec<bool> = vec![false; max_columns];
        
        // Find last display position for each column
        let mut column_last_pos: Vec<usize> = vec![0; max_columns];
        for (pos, &branch_idx) in display_order.iter().enumerate() {
            if let Some(col) = branch_column[branch_idx] {
                column_last_pos[col] = pos;
            }
        }
        
        for (pos, &branch_idx) in display_order.iter().enumerate() {
            let my_col = branch_column[branch_idx].unwrap_or(0);
            
            // Activate this column
            active_columns[my_col] = true;
            
            // Build prefix
            let mut prefix = String::new();
            for col in 0..max_columns {
                if col == my_col {
                    prefix.push('*');
                } else if active_columns[col] {
                    prefix.push('|');
                } else {
                    prefix.push(' ');
                }
                prefix.push(' ');
            }
            
            // Trim and format
            entries[branch_idx].graph_prefix = prefix.trim_end().to_string();
            if !entries[branch_idx].graph_prefix.is_empty() {
                entries[branch_idx].graph_prefix.push(' ');
            }
            
            // Deactivate column if this is its last branch in display order
            if column_last_pos[my_col] == pos {
                active_columns[my_col] = false;
            }
        }
    }

    /// Get entries sorted by commit time (most recent first) to match graph display
    pub fn sorted_entries(&self) -> Vec<&BranchGraphEntry> {
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        order.sort_by(|&a, &b| {
            self.entries[b].branch.last_commit.time.cmp(&self.entries[a].branch.last_commit.time)
        });
        
        order.iter().map(|&idx| &self.entries[idx]).collect()
    }
}
