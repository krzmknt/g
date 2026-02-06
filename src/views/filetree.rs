use crate::config::Theme;
use crate::git::{FileTreeEntry, FileTreeStatus};
use crate::tui::{Buffer, Color, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileViewMode {
    Tree,
    Flat,
    TreeWithIgnored,
}

pub struct FileTreeView {
    pub entries: Vec<FileTreeEntry>,
    pub flat_file_entries: Vec<FileTreeEntry>, // All files with full paths for flat mode
    pub flat_entries: Vec<FlatEntry>,
    pub selected: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub view_mode: FileViewMode,
    pub max_content_width: usize, // Maximum width of content for scroll limiting
    pub view_width: usize,        // Current view width
    pub search_query: Option<String>,
    pub search_results: Vec<usize>,
    /// Filter to show only certain paths (from marked commits)
    pub filter_paths: Option<HashSet<String>>,
}

#[derive(Debug, Clone)]
pub struct FlatEntry {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    pub has_children: bool,
    pub status: Option<FileTreeStatus>,
    pub is_last_at_depth: Vec<bool>, // For each depth level, whether this entry is the last sibling
}

impl FileTreeView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            flat_file_entries: Vec::new(),
            flat_entries: Vec::new(),
            selected: 0,
            offset: 0,
            h_offset: 0,
            view_mode: FileViewMode::Tree,
            max_content_width: 0,
            view_width: 0,
            search_query: None,
            search_results: Vec::new(),
            filter_paths: None,
        }
    }

    pub fn show_ignored(&self) -> bool {
        self.view_mode == FileViewMode::TreeWithIgnored
    }

    pub fn cycle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            FileViewMode::Tree => FileViewMode::Flat,
            FileViewMode::Flat => FileViewMode::TreeWithIgnored,
            FileViewMode::TreeWithIgnored => FileViewMode::Tree,
        };
        self.rebuild_flat_list();
    }

    /// Set filter to show only files in the given paths
    pub fn set_filter(&mut self, paths: HashSet<String>) {
        if paths.is_empty() {
            self.filter_paths = None;
        } else {
            self.filter_paths = Some(paths);
        }
        self.rebuild_flat_list();
    }

    /// Clear the filter
    pub fn clear_filter(&mut self) {
        self.filter_paths = None;
        self.rebuild_flat_list();
    }

    /// Check if a path matches the filter (or no filter is set)
    fn matches_filter(&self, path: &str) -> bool {
        match &self.filter_paths {
            None => true,
            Some(paths) => paths.contains(path),
        }
    }

    /// Check if a directory has any children that match the filter
    fn dir_has_matching_children(&self, entry: &FileTreeEntry) -> bool {
        if self.filter_paths.is_none() {
            return true;
        }
        if !entry.is_dir {
            return self.matches_filter(&entry.path);
        }
        // For directories whose children haven't been loaded yet (lazy loading),
        // check if any filter path starts with this directory's prefix
        let dir_prefix = format!("{}/", entry.path);
        if let Some(ref paths) = self.filter_paths {
            if paths.iter().any(|p| p.starts_with(&dir_prefix)) {
                return true;
            }
        }
        // Also check already-loaded children recursively
        for child in &entry.children {
            if child.is_dir {
                if self.dir_has_matching_children(child) {
                    return true;
                }
            } else if self.matches_filter(&child.path) {
                return true;
            }
        }
        false
    }

    pub fn can_scroll_left(&self) -> bool {
        self.h_offset > 0
    }

    pub fn can_scroll_right(&self) -> bool {
        // If view_width not yet set (before first render), assume scrolling possible if content exists
        if self.view_width == 0 {
            return self.max_content_width > 0;
        }
        self.max_content_width > self.view_width
            && self.h_offset < self.max_content_width.saturating_sub(self.view_width)
    }

    pub fn scroll_left(&mut self) {
        self.h_offset = self.h_offset.saturating_sub(4);
    }

    pub fn scroll_right(&mut self) {
        // Just increment - render will clamp to valid range
        self.h_offset += 4;
    }

    pub fn update(&mut self, entries: Vec<FileTreeEntry>) {
        self.entries = entries;
        self.rebuild_flat_list();
        if self.selected >= self.flat_entries.len() && !self.flat_entries.is_empty() {
            self.selected = self.flat_entries.len() - 1;
        }
    }

    pub fn update_flat(&mut self, entries: Vec<FileTreeEntry>) {
        self.flat_file_entries = entries;
        self.rebuild_flat_list();
        if self.selected >= self.flat_entries.len() && !self.flat_entries.is_empty() {
            self.selected = self.flat_entries.len() - 1;
        }
    }

    fn rebuild_flat_list(&mut self) {
        self.flat_entries.clear();
        match self.view_mode {
            FileViewMode::Tree | FileViewMode::TreeWithIgnored => {
                let entries = self.entries.clone();
                let mut is_last_stack: Vec<bool> = Vec::new();
                self.flatten_entries(&entries, 0, &mut is_last_stack);
            }
            FileViewMode::Flat => {
                let source = if let Some(ref paths) = self.filter_paths {
                    // With filter: use filter paths, looking up status from flat_file_entries
                    let mut filtered: Vec<&FileTreeEntry> = self
                        .flat_file_entries
                        .iter()
                        .filter(|e| paths.contains(&e.path))
                        .collect();
                    if filtered.is_empty() {
                        // Fallback: create entries from filter paths directly
                        let mut paths_vec: Vec<String> = paths.iter().cloned().collect();
                        paths_vec.sort();
                        for path in paths_vec {
                            let status = self.find_entry_status(&path);
                            self.flat_entries.push(FlatEntry {
                                name: path.clone(),
                                path,
                                depth: 0,
                                is_dir: false,
                                expanded: false,
                                has_children: false,
                                status,
                                is_last_at_depth: vec![],
                            });
                        }
                        None
                    } else {
                        filtered.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
                        Some(filtered)
                    }
                } else {
                    // Without filter: use all flat_file_entries
                    let all: Vec<&FileTreeEntry> = self.flat_file_entries.iter().collect();
                    Some(all)
                };
                if let Some(entries) = source {
                    for entry in entries {
                        self.flat_entries.push(FlatEntry {
                            name: entry.path.clone(), // Full path as display name
                            path: entry.path.clone(),
                            depth: 0,
                            is_dir: false,
                            expanded: false,
                            has_children: false,
                            status: entry.status,
                            is_last_at_depth: vec![],
                        });
                    }
                }
            }
        }
        self.recalc_max_content_width();
    }

    fn recalc_max_content_width(&mut self) {
        self.max_content_width = self
            .flat_entries
            .iter()
            .map(|entry| {
                let edge_width = entry.depth * 3;
                let icon_width = 4; // " X  " (space + icon + 2 spaces)
                let name_width = entry.name.chars().count();
                let status_width =
                    if entry.status.is_some() && entry.status != Some(FileTreeStatus::Ignored) {
                        2
                    } else {
                        0
                    };
                edge_width + icon_width + name_width + status_width
            })
            .max()
            .unwrap_or(0)
            + 2; // +2 for scrollbar (1) + margin (1)
    }

    fn flatten_entries(
        &mut self,
        entries: &[FileTreeEntry],
        depth: usize,
        is_last_stack: &mut Vec<bool>,
    ) {
        // Filter entries based on filter_paths
        let filtered_entries: Vec<&FileTreeEntry> = entries
            .iter()
            .filter(|e| {
                if e.is_dir {
                    self.dir_has_matching_children(e)
                } else {
                    self.matches_filter(&e.path)
                }
            })
            .collect();

        let len = filtered_entries.len();
        for (i, entry) in filtered_entries.iter().enumerate() {
            let is_last = i == len - 1;

            // Build is_last_at_depth: copy current stack and add current level
            let mut is_last_at_depth = is_last_stack.clone();
            is_last_at_depth.push(is_last);

            self.flat_entries.push(FlatEntry {
                path: entry.path.clone(),
                name: entry.name.clone(),
                depth,
                is_dir: entry.is_dir,
                expanded: entry.expanded,
                has_children: !entry.children.is_empty(),
                status: entry.status,
                is_last_at_depth,
            });

            if entry.is_dir && entry.expanded {
                // Push whether current entry is last at this depth for children to reference
                is_last_stack.push(is_last);
                self.flatten_entries(&entry.children, depth + 1, is_last_stack);
                is_last_stack.pop();
            }
        }
    }

    fn find_entry_status(&self, path: &str) -> Option<FileTreeStatus> {
        fn search(entries: &[FileTreeEntry], path: &str) -> Option<FileTreeStatus> {
            for entry in entries {
                if entry.path == path {
                    return entry.status;
                }
                if entry.is_dir {
                    if let Some(status) = search(&entry.children, path) {
                        return Some(status);
                    }
                }
            }
            None
        }
        search(&self.entries, path)
    }

    pub fn selected_entry(&self) -> Option<&FlatEntry> {
        self.flat_entries.get(self.selected)
    }

    /// Toggle expand/collapse. Returns Some(path) if children need to be loaded, None otherwise.
    pub fn toggle_expand(&mut self) -> Option<String> {
        if let Some(entry) = self.flat_entries.get(self.selected) {
            if entry.is_dir {
                let path = entry.path.clone();
                let needs_load = self.toggle_dir_expanded(&path);
                self.rebuild_flat_list();
                if needs_load {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Load children for a directory (called after lazy load)
    pub fn load_children(&mut self, path: &str, children: Vec<FileTreeEntry>) {
        fn load_in_entries(
            entries: &mut [FileTreeEntry],
            path: &str,
            children: Vec<FileTreeEntry>,
        ) -> bool {
            for entry in entries {
                if entry.path == path {
                    entry.children = children;
                    return true;
                }
                if entry.is_dir && load_in_entries(&mut entry.children, path, children.clone()) {
                    return true;
                }
            }
            false
        }
        load_in_entries(&mut self.entries, path, children);
        self.rebuild_flat_list();
    }

    /// Returns true if children need to be loaded (was collapsed and has placeholder)
    fn toggle_dir_expanded(&mut self, path: &str) -> bool {
        fn toggle_in_entries(entries: &mut [FileTreeEntry], path: &str) -> (bool, bool) {
            for entry in entries {
                if entry.path == path {
                    let was_collapsed = !entry.expanded;
                    entry.expanded = !entry.expanded;
                    // Check if children need loading (has placeholder "...")
                    let needs_load = was_collapsed
                        && entry.children.len() == 1
                        && entry.children[0].name == "...";
                    return (true, needs_load);
                }
                if entry.is_dir {
                    let (found, needs_load) = toggle_in_entries(&mut entry.children, path);
                    if found {
                        return (true, needs_load);
                    }
                }
            }
            (false, false)
        }
        let (_, needs_load) = toggle_in_entries(&mut self.entries, path);
        needs_load
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.flat_entries.is_empty() && self.selected + 1 < self.flat_entries.len() {
            self.selected += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.flat_entries.is_empty() {
            self.selected = self.flat_entries.len() - 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) {
        let index = self.offset + row;
        if index < self.flat_entries.len() {
            self.selected = index;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_results.clear();

        let query_lower = query.to_lowercase();

        for (i, entry) in self.flat_entries.iter().enumerate() {
            if entry.name.to_lowercase().contains(&query_lower) {
                self.search_results.push(i);
            }
        }

        // Jump to first result
        if let Some(&first) = self.search_results.first() {
            self.selected = first;
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_results.clear();
    }

    pub fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(pos) = self.search_results.iter().position(|&i| i > self.selected) {
            self.selected = self.search_results[pos];
        } else {
            // Wrap around
            self.selected = self.search_results[0];
        }
    }

    pub fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(pos) = self.search_results.iter().rposition(|&i| i < self.selected) {
            self.selected = self.search_results[pos];
        } else {
            // Wrap around
            self.selected = *self.search_results.last().unwrap();
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let title = match self.view_mode {
            FileViewMode::Tree => " Files ",
            FileViewMode::Flat => " Files [flat] ",
            FileViewMode::TreeWithIgnored => " Files [+ignored] ",
        };

        let block = Block::new()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let height = inner.height as usize;

        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }

        let content_width = inner.width.saturating_sub(1);
        let edge_color = Color::Rgb(70, 70, 70); // Light gray for edges

        // Store view width for scroll limiting
        self.view_width = content_width as usize;

        // Clamp h_offset if it exceeds the valid range
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        if self.flat_entries.is_empty() {
            let msg = "Empty";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
        } else {
            for (i, entry) in self
                .flat_entries
                .iter()
                .skip(self.offset)
                .take(height)
                .enumerate()
            {
                let y = inner.y + i as u16;
                let is_selected = self.selected == self.offset + i;
                let is_search_match = self.search_results.contains(&(self.offset + i));

                // Get appropriate icon
                let icon = if entry.is_dir {
                    Self::get_dir_icon(entry.expanded, entry.has_children)
                } else {
                    Self::get_file_icon(&entry.name)
                };

                // Git status indicator with bright colors
                let (status_indicator, status_sign_color) = match entry.status {
                    Some(FileTreeStatus::Modified) => (" M", Some(Color::Rgb(255, 200, 50))), // bright yellow
                    Some(FileTreeStatus::Added) => (" A", Some(Color::Rgb(100, 255, 100))), // bright green
                    Some(FileTreeStatus::Deleted) => (" D", Some(Color::Rgb(255, 100, 100))), // bright red
                    Some(FileTreeStatus::Untracked) => (" ?", Some(Color::Rgb(200, 200, 200))), // light gray
                    Some(FileTreeStatus::Ignored) => ("", None), // No indicator for ignored
                    None => ("", None),
                };

                // Check if file is ignored - use gray for both icon and name
                let is_ignored = entry.status == Some(FileTreeStatus::Ignored);
                let ignored_color = Color::Rgb(100, 100, 100); // Gray for ignored files

                // Get icon color (also used for directory names)
                let icon_color = if is_ignored {
                    ignored_color
                } else {
                    Self::get_icon_color(&entry.name, entry.is_dir)
                };

                // For directories, use icon color for name too. For files, use status color.
                let name_color = if is_ignored {
                    ignored_color
                } else if entry.is_dir {
                    icon_color
                } else {
                    match entry.status {
                        Some(FileTreeStatus::Modified) => theme.unstaged,
                        Some(FileTreeStatus::Added) => theme.staged,
                        Some(FileTreeStatus::Deleted) => theme.diff_remove,
                        Some(FileTreeStatus::Untracked) => theme.untracked,
                        Some(FileTreeStatus::Ignored) => ignored_color,
                        None => icon_color, // Use icon color for files without git status too
                    }
                };

                let style = if is_selected && focused {
                    Style::new().fg(theme.selection_text).bg(theme.selection)
                } else if is_search_match {
                    Style::new().fg(theme.diff_hunk)
                } else {
                    Style::new().fg(name_color)
                };

                // Fill full line width when selected and focused
                if is_selected && focused {
                    let blank_line = " ".repeat(content_width as usize);
                    buf.set_string(inner.x, y, &blank_line, style);
                }
                let icon_style = if is_selected && focused {
                    Style::new().fg(icon_color).bg(theme.selection)
                } else {
                    Style::new().fg(icon_color)
                };

                // Draw vertical edge lines (light gray)
                // Use │ for continuing edges, ╵ for last child at each depth
                let edge_style = if is_selected && focused {
                    style
                } else {
                    Style::new().fg(edge_color)
                };

                // Build edge string based on is_last_at_depth
                // Each depth level = 3 chars: " X " where X is │ or ╵
                let mut edge_str = String::new();
                for d in 0..entry.depth {
                    // Check if ancestor at depth d was the last sibling
                    // If it was last, no edge needed (empty space)
                    // If it was not last, draw continuing edge │
                    if d < entry.is_last_at_depth.len() && entry.is_last_at_depth[d] {
                        edge_str.push_str("   "); // No edge for last sibling's descendants
                    } else {
                        edge_str.push_str(" │ "); // Continuing edge
                    }
                }
                let edge_width = entry.depth * 3;

                // Icon with spacing: " icon  " (space + icon + 2 spaces)
                let icon_with_space = format!(" {}  ", icon);

                // Status sign style
                let status_style = if is_selected && focused {
                    Style::new()
                        .fg(status_sign_color.unwrap_or(theme.foreground))
                        .bg(theme.selection)
                } else {
                    Style::new().fg(status_sign_color.unwrap_or(theme.foreground))
                };

                // Apply horizontal scroll
                if self.h_offset < edge_width {
                    // Scroll is within edge area
                    let visible_edge_chars = edge_width - self.h_offset;
                    let display_edge: String = edge_str.chars().skip(self.h_offset).collect();
                    buf.set_string(inner.x, y, &display_edge, edge_style);

                    let icon_x = inner.x + visible_edge_chars as u16;
                    let icon_char_count = icon_with_space.chars().count() as u16;
                    let name_x = icon_x + icon_char_count;
                    let remaining_width = content_width.saturating_sub(visible_edge_chars as u16);

                    if remaining_width > 0 {
                        buf.set_string(icon_x, y, &icon_with_space, icon_style);
                        let name_char_count = entry.name.chars().count() as u16;
                        let name_width = remaining_width.saturating_sub(icon_char_count);
                        if name_width > 0 {
                            buf.set_string_truncated(name_x, y, &entry.name, name_width, style);
                            // Draw status indicator with its own color
                            if !status_indicator.is_empty() {
                                let status_x = name_x + name_char_count.min(name_width);
                                let status_width = name_width.saturating_sub(name_char_count);
                                if status_width > 0 {
                                    buf.set_string_truncated(
                                        status_x,
                                        y,
                                        status_indicator,
                                        status_width,
                                        status_style,
                                    );
                                }
                            }
                        }
                    }
                } else {
                    // Scroll is past edges (or no edges at depth 0), render each part with its own style
                    let content_offset = self.h_offset - edge_width;
                    let icon_chars: Vec<char> = icon_with_space.chars().collect();
                    let name_chars: Vec<char> = entry.name.chars().collect();
                    let status_chars: Vec<char> = status_indicator.chars().collect();

                    let icon_len = icon_chars.len();
                    let name_len = name_chars.len();

                    let mut x_pos = inner.x;
                    let mut remaining = content_width;

                    // Render icon part (if visible after offset)
                    if content_offset < icon_len {
                        let skip = content_offset;
                        let icon_text: String = icon_chars.iter().skip(skip).collect();
                        let display_len = (icon_len - skip).min(remaining as usize);
                        if display_len > 0 {
                            let display_text: String =
                                icon_text.chars().take(display_len).collect();
                            buf.set_string(x_pos, y, &display_text, icon_style);
                            x_pos += display_len as u16;
                            remaining = remaining.saturating_sub(display_len as u16);
                        }
                    }

                    // Render name part
                    let name_start = icon_len;
                    if content_offset < name_start + name_len && remaining > 0 {
                        let skip = content_offset.saturating_sub(name_start);
                        let name_text: String = name_chars.iter().skip(skip).collect();
                        let display_len = (name_len - skip).min(remaining as usize);
                        if display_len > 0 {
                            let display_text: String =
                                name_text.chars().take(display_len).collect();
                            buf.set_string(x_pos, y, &display_text, style);
                            x_pos += display_len as u16;
                            remaining = remaining.saturating_sub(display_len as u16);
                        }
                    }

                    // Render status indicator with its own color
                    let status_start = name_start + name_len;
                    if !status_indicator.is_empty()
                        && content_offset < status_start + status_chars.len()
                        && remaining > 0
                    {
                        let skip = content_offset.saturating_sub(status_start);
                        let status_text: String = status_chars.iter().skip(skip).collect();
                        let display_len = (status_chars.len() - skip).min(remaining as usize);
                        if display_len > 0 {
                            let display_text: String =
                                status_text.chars().take(display_len).collect();
                            buf.set_string(x_pos, y, &display_text, status_style);
                        }
                    }
                }
            }
        }

        let scrollbar = Scrollbar::new(self.flat_entries.len(), height, self.offset);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn get_dir_icon(expanded: bool, _has_children: bool) -> &'static str {
        // NerdFont icons: folder open = \uf115, folder closed = \uf114
        if expanded {
            "\u{f115}" // nf-fa-folder_open
        } else {
            "\u{f114}" // nf-fa-folder
        }
    }

    fn get_icon_color(name: &str, is_dir: bool) -> Color {
        if is_dir {
            return Color::Rgb(86, 156, 214); // Blue for folders
        }

        // Check special filenames first
        let lower_name = name.to_lowercase();
        match lower_name.as_str() {
            "makefile" | "gnumakefile" => return Color::Rgb(229, 77, 62), // Red
            "dockerfile" => return Color::Rgb(56, 152, 214),              // Docker blue
            "cargo.toml" | "cargo.lock" => return Color::Rgb(222, 165, 132), // Rust brown/orange
            "package.json" | "package-lock.json" => return Color::Rgb(139, 195, 74), // Node green
            ".gitignore" | ".gitattributes" | ".gitmodules" => return Color::Rgb(240, 80, 50), // Git orange
            ".env" | ".env.local" | ".env.example" => return Color::Rgb(234, 197, 77), // Yellow
            "readme.md" | "readme" | "readme.txt" => return Color::Rgb(66, 165, 245),  // Blue
            "license" | "license.md" | "license.txt" => return Color::Rgb(255, 167, 38), // Orange
            ".dockerignore" => return Color::Rgb(56, 152, 214), // Docker blue
            "tsconfig.json" => return Color::Rgb(49, 120, 198), // TypeScript blue
            _ => {}
        };

        // Get extension
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();

        match ext.as_str() {
            // Rust
            "rs" => Color::Rgb(222, 165, 132), // Rust brown/orange
            // JavaScript/TypeScript
            "js" | "mjs" | "cjs" => Color::Rgb(241, 224, 90), // JS yellow
            "ts" => Color::Rgb(49, 120, 198),                 // TypeScript blue
            "tsx" | "jsx" => Color::Rgb(97, 218, 251),        // React cyan
            // Web
            "html" | "htm" => Color::Rgb(228, 77, 38), // HTML orange
            "css" => Color::Rgb(66, 165, 245),         // CSS blue
            "scss" | "sass" => Color::Rgb(205, 103, 153), // Sass pink
            "less" => Color::Rgb(29, 54, 93),          // Less blue
            "vue" => Color::Rgb(65, 184, 131),         // Vue green
            "svelte" => Color::Rgb(255, 62, 0),        // Svelte orange
            // Data/Config
            "json" => Color::Rgb(251, 193, 45), // JSON yellow
            "yaml" | "yml" => Color::Rgb(203, 56, 55), // YAML red
            "toml" => Color::Rgb(156, 154, 150), // TOML gray
            "xml" => Color::Rgb(227, 119, 40),  // XML orange
            "csv" => Color::Rgb(34, 139, 34),   // CSV green
            // Shell/Scripts
            "sh" | "bash" | "zsh" | "fish" => Color::Rgb(137, 224, 81), // Shell green
            "ps1" | "bat" | "cmd" => Color::Rgb(1, 188, 211),           // Powershell cyan
            // Python
            "py" => Color::Rgb(53, 114, 165), // Python blue
            // Ruby
            "rb" => Color::Rgb(204, 52, 45), // Ruby red
            // Go
            "go" => Color::Rgb(0, 173, 216), // Go cyan
            // Java/Kotlin
            "java" => Color::Rgb(244, 67, 54),         // Java red
            "kt" | "kts" => Color::Rgb(169, 123, 255), // Kotlin purple
            // C/C++
            "c" => Color::Rgb(85, 85, 255),                 // C blue
            "cpp" | "cc" | "cxx" => Color::Rgb(0, 89, 156), // C++ blue
            "h" | "hpp" => Color::Rgb(146, 131, 194),       // Header purple
            // C#
            "cs" => Color::Rgb(104, 33, 122), // C# purple
            // PHP
            "php" => Color::Rgb(119, 123, 180), // PHP purple
            // Swift
            "swift" => Color::Rgb(255, 172, 69), // Swift orange
            // Markdown/Docs
            "md" | "markdown" => Color::Rgb(66, 165, 245), // Markdown blue
            "txt" => Color::Rgb(175, 175, 175),            // Text gray
            "pdf" => Color::Rgb(244, 67, 54),              // PDF red
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => Color::Rgb(156, 39, 176), // Image purple
            "svg" => Color::Rgb(255, 177, 60), // SVG orange
            // Videos
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Color::Rgb(255, 87, 34), // Video orange
            // Audio
            "mp3" | "wav" | "flac" | "ogg" | "m4a" => Color::Rgb(251, 140, 0), // Audio orange
            // Archives
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => Color::Rgb(175, 180, 43), // Archive yellow-green
            // Lock files
            "lock" => Color::Rgb(255, 214, 0), // Lock yellow
            // SQL
            "sql" => Color::Rgb(0, 150, 136), // SQL teal
            // Log
            "log" => Color::Rgb(158, 158, 158), // Log gray
            // Lua
            "lua" => Color::Rgb(0, 0, 128), // Lua blue
            // Vim
            "vim" => Color::Rgb(1, 152, 51), // Vim green
            // Haskell
            "hs" => Color::Rgb(94, 80, 134), // Haskell purple
            // Elixir
            "ex" | "exs" => Color::Rgb(110, 74, 126), // Elixir purple
            // Erlang
            "erl" => Color::Rgb(163, 51, 82), // Erlang red
            // Clojure
            "clj" | "cljs" => Color::Rgb(100, 181, 240), // Clojure blue
            // Scala
            "scala" => Color::Rgb(222, 56, 68), // Scala red
            // R
            "r" => Color::Rgb(25, 118, 210), // R blue
            // Nix
            "nix" => Color::Rgb(126, 186, 228), // Nix blue
            // Default
            _ => Color::Rgb(175, 175, 175), // Default gray
        }
    }

    fn get_file_icon(name: &str) -> &'static str {
        // Check special filenames first
        let lower_name = name.to_lowercase();
        match lower_name.as_str() {
            "makefile" | "gnumakefile" => return "\u{e673}", // nf-seti-makefile
            "dockerfile" => return "\u{f308}",               // nf-linux-docker
            "cargo.toml" | "cargo.lock" => return "\u{e7a8}", // nf-dev-rust
            "package.json" | "package-lock.json" => return "\u{e718}", // nf-dev-nodejs_small
            ".gitignore" | ".gitattributes" | ".gitmodules" => return "\u{f1d3}", // nf-fa-git
            ".env" | ".env.local" | ".env.example" => return "\u{f462}", // nf-oct-key
            "readme.md" | "readme" | "readme.txt" => return "\u{f48a}", // nf-oct-book
            "license" | "license.md" | "license.txt" => return "\u{f718}", // nf-md-license
            ".dockerignore" => return "\u{f308}",            // nf-linux-docker
            "tsconfig.json" => return "\u{e628}",            // nf-seti-typescript
            _ => {}
        };

        // Get extension
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();

        match ext.as_str() {
            // Rust
            "rs" => "\u{e7a8}", // nf-dev-rust
            // JavaScript/TypeScript
            "js" | "mjs" | "cjs" => "\u{e74e}", // nf-dev-javascript
            "ts" => "\u{e628}",                 // nf-seti-typescript
            "tsx" => "\u{e7ba}",                // nf-dev-react
            "jsx" => "\u{e7ba}",                // nf-dev-react
            // Web
            "html" | "htm" => "\u{e736}",  // nf-dev-html5
            "css" => "\u{e749}",           // nf-dev-css3
            "scss" | "sass" => "\u{e603}", // nf-dev-sass
            "less" => "\u{e758}",          // nf-dev-less
            "vue" => "\u{e6a0}",           // nf-seti-vue
            "svelte" => "\u{e697}",        // nf-seti-svelte
            // Data/Config
            "json" => "\u{e60b}",         // nf-seti-json
            "yaml" | "yml" => "\u{e6a8}", // nf-seti-yml
            "toml" => "\u{e6b2}",         // nf-seti-settings
            "xml" => "\u{e619}",          // nf-seti-xml
            "csv" => "\u{f1c3}",          // nf-fa-file_excel_o
            // Shell/Scripts
            "sh" | "bash" | "zsh" | "fish" => "\u{e795}", // nf-dev-terminal
            "ps1" | "bat" | "cmd" => "\u{e70f}",          // nf-dev-windows
            // Python
            "py" => "\u{e73c}", // nf-dev-python
            // Ruby
            "rb" => "\u{e739}", // nf-dev-ruby
            // Go
            "go" => "\u{e626}", // nf-seti-go
            // Java/Kotlin
            "java" => "\u{e738}",       // nf-dev-java
            "kt" | "kts" => "\u{e634}", // nf-seti-kotlin
            // C/C++
            "c" => "\u{e61e}",                  // nf-custom-c
            "cpp" | "cc" | "cxx" => "\u{e61d}", // nf-custom-cpp
            "h" | "hpp" => "\u{e61e}",          // nf-custom-c
            // C#
            "cs" => "\u{f81a}", // nf-md-language_csharp
            // PHP
            "php" => "\u{e73d}", // nf-dev-php
            // Swift
            "swift" => "\u{e755}", // nf-dev-swift
            // Markdown/Docs
            "md" | "markdown" => "\u{e73e}", // nf-dev-markdown
            "txt" => "\u{f15c}",             // nf-fa-file_text
            "pdf" => "\u{f1c1}",             // nf-fa-file_pdf_o
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => "\u{f1c5}", // nf-fa-file_image_o
            "svg" => "\u{e697}",                                                   // nf-seti-svg
            // Videos
            "mp4" | "mkv" | "avi" | "mov" | "webm" => "\u{f1c8}", // nf-fa-file_video_o
            // Audio
            "mp3" | "wav" | "flac" | "ogg" | "m4a" => "\u{f1c7}", // nf-fa-file_audio_o
            // Archives
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "\u{f1c6}", // nf-fa-file_archive_o
            // Lock files
            "lock" => "\u{f023}", // nf-fa-lock
            // SQL
            "sql" => "\u{e706}", // nf-dev-database
            // Log
            "log" => "\u{f15c}", // nf-fa-file_text
            // Lua
            "lua" => "\u{e620}", // nf-seti-lua
            // Vim
            "vim" => "\u{e62b}", // nf-seti-vim
            // Haskell
            "hs" => "\u{e61f}", // nf-seti-haskell
            // Elixir
            "ex" | "exs" => "\u{e62d}", // nf-seti-elixir
            // Erlang
            "erl" => "\u{e7b1}", // nf-dev-erlang
            // Clojure
            "clj" | "cljs" => "\u{e768}", // nf-dev-clojure
            // Scala
            "scala" => "\u{e737}", // nf-dev-scala
            // R
            "r" => "\u{f25d}", // nf-fa-registered
            // Nix
            "nix" => "\u{f313}", // nf-linux-nixos
            // Default
            _ => "\u{f016}", // nf-fa-file_o (outlined/hollow)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{FileTreeEntry, FileTreeStatus};

    fn make_file(name: &str, path: &str, status: Option<FileTreeStatus>) -> FileTreeEntry {
        FileTreeEntry {
            name: name.to_string(),
            path: path.to_string(),
            is_dir: false,
            status,
            children: vec![],
            expanded: false,
        }
    }

    fn make_dir(
        name: &str,
        path: &str,
        children: Vec<FileTreeEntry>,
        expanded: bool,
    ) -> FileTreeEntry {
        FileTreeEntry {
            name: name.to_string(),
            path: path.to_string(),
            is_dir: true,
            status: None,
            children,
            expanded,
        }
    }

    #[test]
    fn test_default_view_mode_is_tree() {
        let view = FileTreeView::new();
        assert_eq!(view.view_mode, FileViewMode::Tree);
    }

    #[test]
    fn test_cycle_view_mode() {
        let mut view = FileTreeView::new();
        assert_eq!(view.view_mode, FileViewMode::Tree);
        view.cycle_view_mode();
        assert_eq!(view.view_mode, FileViewMode::Flat);
        view.cycle_view_mode();
        assert_eq!(view.view_mode, FileViewMode::TreeWithIgnored);
        view.cycle_view_mode();
        assert_eq!(view.view_mode, FileViewMode::Tree);
    }

    #[test]
    fn test_flat_mode_shows_full_paths() {
        let mut view = FileTreeView::new();
        // Simulate repo providing tree entries (lazy-loaded, only top-level)
        let tree_entries = vec![make_dir("src", "src", vec![], false)];
        view.update(tree_entries);

        // Simulate repo providing flat file entries (all files recursively)
        let flat_entries = vec![
            make_file("src/main.rs", "src/main.rs", None),
            make_file("src/views/diff.rs", "src/views/diff.rs", None),
        ];
        view.cycle_view_mode(); // Switch to flat
        view.update_flat(flat_entries);

        assert_eq!(view.flat_entries.len(), 2);
        assert_eq!(view.flat_entries[0].name, "src/main.rs");
        assert_eq!(view.flat_entries[0].path, "src/main.rs");
        assert_eq!(view.flat_entries[0].depth, 0);
        assert!(!view.flat_entries[0].is_dir);
        assert_eq!(view.flat_entries[1].name, "src/views/diff.rs");
    }

    #[test]
    fn test_flat_mode_entries_from_repo_exclude_ignored() {
        let mut view = FileTreeView::new();
        view.cycle_view_mode(); // Switch to flat

        // Repo's file_tree_flat() already excludes ignored files,
        // so flat_file_entries should only contain non-ignored files
        let flat_entries = vec![make_file(
            "visible.rs",
            "visible.rs",
            Some(FileTreeStatus::Modified),
        )];
        view.update_flat(flat_entries);

        assert_eq!(view.flat_entries.len(), 1);
        assert_eq!(view.flat_entries[0].name, "visible.rs");
    }

    #[test]
    fn test_flat_mode_sorted_alphabetically() {
        let mut view = FileTreeView::new();
        view.cycle_view_mode(); // Switch to flat

        // Repo's file_tree_flat() returns entries sorted alphabetically
        let flat_entries = vec![
            make_file("alpha.rs", "alpha.rs", None),
            make_file("mid.rs", "mid.rs", None),
            make_file("zebra.rs", "zebra.rs", None),
        ];
        view.update_flat(flat_entries);

        let names: Vec<&str> = view.flat_entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha.rs", "mid.rs", "zebra.rs"]);
    }

    #[test]
    fn test_flat_mode_with_filter_uses_filter_paths() {
        let mut view = FileTreeView::new();

        // Provide flat file entries (simulating repo data)
        let flat_entries = vec![
            make_file("src/main.rs", "src/main.rs", Some(FileTreeStatus::Modified)),
            make_file("src/lib.rs", "src/lib.rs", None),
        ];

        // Set filter before switching to flat
        let mut filter = HashSet::new();
        filter.insert("src/main.rs".to_string());
        view.set_filter(filter);

        view.cycle_view_mode(); // Switch to flat
        view.update_flat(flat_entries);

        assert_eq!(view.flat_entries.len(), 1);
        assert_eq!(view.flat_entries[0].name, "src/main.rs");
        assert_eq!(view.flat_entries[0].status, Some(FileTreeStatus::Modified));
    }

    #[test]
    fn test_tree_mode_preserves_depth() {
        let mut view = FileTreeView::new();
        let entries = vec![make_dir(
            "src",
            "src",
            vec![make_file("main.rs", "src/main.rs", None)],
            true,
        )];
        view.update(entries);

        // Should be in tree mode by default
        assert_eq!(view.view_mode, FileViewMode::Tree);
        assert_eq!(view.flat_entries.len(), 2); // dir + file
        assert_eq!(view.flat_entries[0].depth, 0); // dir at depth 0
        assert!(view.flat_entries[0].is_dir);
        assert_eq!(view.flat_entries[1].depth, 1); // file at depth 1
    }

    #[test]
    fn test_toggle_back_to_tree_restores_hierarchy() {
        let mut view = FileTreeView::new();
        let tree_entries = vec![make_dir(
            "src",
            "src",
            vec![make_file("main.rs", "src/main.rs", None)],
            true,
        )];
        view.update(tree_entries);

        // Switch to flat and provide flat data
        view.cycle_view_mode(); // flat
        let flat_entries = vec![make_file("src/main.rs", "src/main.rs", None)];
        view.update_flat(flat_entries);
        assert_eq!(view.flat_entries.len(), 1); // only the file
        assert_eq!(view.flat_entries[0].depth, 0);

        view.cycle_view_mode(); // TreeWithIgnored (still tree layout)
        assert_eq!(view.flat_entries.len(), 2); // dir + file
        assert!(view.flat_entries[0].is_dir);

        view.cycle_view_mode(); // back to Tree
        assert_eq!(view.flat_entries.len(), 2); // dir + file
        assert!(view.flat_entries[0].is_dir);
    }
}
