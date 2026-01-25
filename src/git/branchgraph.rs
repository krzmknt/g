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
        // Sort entries by tree order (parents before children, depth-first)
        let n = entries.len();
        let mut visited = vec![false; n];
        let mut order: Vec<usize> = Vec::new();

        fn visit(
            entries: &[BranchGraphEntry],
            idx: usize,
            visited: &mut [bool],
            order: &mut Vec<usize>,
        ) {
            if visited[idx] {
                return;
            }
            visited[idx] = true;
            order.push(idx);
            for &child_idx in &entries[idx].children {
                visit(entries, child_idx, visited, order);
            }
        }

        // Visit roots first
        for i in 0..n {
            if entries[i].parent.is_none() {
                visit(entries, i, &mut visited, &mut order);
            }
        }

        // Visit any remaining (disconnected branches)
        for i in 0..n {
            if !visited[i] {
                visit(entries, i, &mut visited, &mut order);
            }
        }

        // Build prefixes based on depth and tree structure
        for &idx in &order {
            let depth = entries[idx].depth;
            let has_children = !entries[idx].children.is_empty();

            let mut prefix = String::new();
            for d in 0..depth {
                if d == depth - 1 {
                    // Check if this is the last child of its parent
                    if let Some(parent_idx) = entries[idx].parent {
                        let is_last = entries[parent_idx]
                            .children
                            .last()
                            .map(|&last| last == idx)
                            .unwrap_or(false);
                        if is_last {
                            prefix.push_str("└─");
                        } else {
                            prefix.push_str("├─");
                        }
                    } else {
                        prefix.push_str("  ");
                    }
                } else {
                    // Check if there's a sibling at this depth level that continues
                    prefix.push_str("│ ");
                }
            }

            if has_children {
                prefix.push_str("● ");
            } else {
                prefix.push_str("○ ");
            }

            entries[idx].graph_prefix = prefix;
        }
    }

    /// Get entries sorted by tree order (for display)
    pub fn sorted_entries(&self) -> Vec<&BranchGraphEntry> {
        let n = self.entries.len();
        let mut visited = vec![false; n];
        let mut result: Vec<&BranchGraphEntry> = Vec::new();

        fn visit<'a>(
            entries: &'a [BranchGraphEntry],
            idx: usize,
            visited: &mut [bool],
            result: &mut Vec<&'a BranchGraphEntry>,
        ) {
            if visited[idx] {
                return;
            }
            visited[idx] = true;
            result.push(&entries[idx]);

            // Sort children by commit time (most recent first)
            let mut children: Vec<usize> = entries[idx].children.clone();
            children.sort_by(|&a, &b| {
                entries[b].branch.last_commit.time.cmp(&entries[a].branch.last_commit.time)
            });

            for child_idx in children {
                visit(entries, child_idx, visited, result);
            }
        }

        // Sort roots by commit time (most recent first)
        let mut roots: Vec<usize> = (0..n)
            .filter(|&i| self.entries[i].parent.is_none())
            .collect();
        roots.sort_by(|&a, &b| {
            self.entries[b].branch.last_commit.time.cmp(&self.entries[a].branch.last_commit.time)
        });

        for root_idx in roots {
            visit(&self.entries, root_idx, &mut visited, &mut result);
        }

        // Add any remaining entries
        for i in 0..n {
            if !visited[i] {
                visit(&self.entries, i, &mut visited, &mut result);
            }
        }

        result
    }
}
