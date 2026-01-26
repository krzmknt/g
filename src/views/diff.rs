use crate::config::Theme;
use crate::git::{DiffInfo, FileDiff, IssueInfo, LineType, PullRequestInfo};
use crate::tui::{str_display_width, unicode_width, Buffer, Color, Rect, Style};
use crate::widgets::{Block, Borders, Scrollbar, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    Inline,
    SideBySide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewType {
    Diff,
    FileContent,
    PullRequest,
    Commit,
    Issue,
}

#[derive(Debug, Clone)]
pub struct FileContent {
    pub path: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PullRequestPreview {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub is_draft: bool,
    pub base_branch: String,
    pub head_branch: String,
    pub additions: u32,
    pub deletions: u32,
    pub body: String,
    pub url: String,
    pub created_at: String,
}

impl From<&PullRequestInfo> for PullRequestPreview {
    fn from(pr: &PullRequestInfo) -> Self {
        Self {
            number: pr.number,
            title: pr.title.clone(),
            author: pr.author.login.clone(),
            state: pr.state.clone(),
            is_draft: pr.is_draft,
            base_branch: pr.base_ref_name.clone(),
            head_branch: pr.head_ref_name.clone(),
            additions: pr.additions,
            deletions: pr.deletions,
            body: pr.body.clone(),
            url: pr.url.clone(),
            created_at: pr.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitPreview {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IssueCommentPreview {
    pub author: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct IssuePreview {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub state: String,
    pub labels: Vec<String>,
    pub body: String,
    pub url: String,
    pub created_at: String,
    pub comment_list: Vec<IssueCommentPreview>,
}

impl From<&IssueInfo> for IssuePreview {
    fn from(issue: &IssueInfo) -> Self {
        Self {
            number: issue.number,
            title: issue.title.clone(),
            author: issue.author.login.clone(),
            state: issue.state.clone(),
            labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
            body: issue.body.clone(),
            url: issue.url.clone(),
            created_at: issue.created_at.clone(),
            comment_list: issue
                .comments
                .iter()
                .map(|c| IssueCommentPreview {
                    author: c.author.login.clone(),
                    body: c.body.clone(),
                    created_at: c.created_at.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntaxType {
    None,
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    C,
    Cpp,
    Shell,
    Toml,
    Json,
}

impl SyntaxType {
    fn from_path(path: &str) -> Self {
        let filename = path.rsplit('/').next().unwrap_or(path).to_lowercase();
        match filename.as_str() {
            "makefile" | "dockerfile" => return SyntaxType::Shell,
            "cargo.toml" => return SyntaxType::Toml,
            _ => {}
        }
        let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
        match ext.as_str() {
            "rs" => SyntaxType::Rust,
            "js" | "mjs" | "jsx" => SyntaxType::JavaScript,
            "ts" | "tsx" => SyntaxType::TypeScript,
            "py" => SyntaxType::Python,
            "go" => SyntaxType::Go,
            "c" | "h" => SyntaxType::C,
            "cpp" | "cc" | "hpp" => SyntaxType::Cpp,
            "sh" | "bash" | "zsh" => SyntaxType::Shell,
            "toml" => SyntaxType::Toml,
            "json" => SyntaxType::Json,
            _ => SyntaxType::None,
        }
    }

    fn keywords(&self) -> &[&str] {
        match self {
            SyntaxType::Rust => &[
                "fn", "let", "mut", "const", "if", "else", "match", "for", "while", "loop",
                "break", "continue", "return", "pub", "mod", "use", "struct", "enum", "impl",
                "trait", "type", "async", "await", "move", "self", "Self", "super", "crate", "as",
                "in", "dyn", "unsafe", "where",
            ],
            SyntaxType::JavaScript | SyntaxType::TypeScript => &[
                "function",
                "const",
                "let",
                "var",
                "if",
                "else",
                "for",
                "while",
                "return",
                "class",
                "extends",
                "new",
                "this",
                "import",
                "export",
                "default",
                "from",
                "async",
                "await",
                "try",
                "catch",
                "throw",
                "true",
                "false",
                "null",
                "undefined",
            ],
            SyntaxType::Python => &[
                "def", "class", "if", "elif", "else", "for", "while", "try", "except", "finally",
                "with", "as", "import", "from", "return", "yield", "raise", "pass", "break",
                "continue", "and", "or", "not", "in", "is", "lambda", "True", "False", "None",
            ],
            SyntaxType::Go => &[
                "func",
                "var",
                "const",
                "type",
                "struct",
                "interface",
                "map",
                "chan",
                "if",
                "else",
                "for",
                "range",
                "switch",
                "case",
                "default",
                "break",
                "continue",
                "return",
                "go",
                "defer",
                "select",
                "package",
                "import",
                "true",
                "false",
                "nil",
            ],
            SyntaxType::C | SyntaxType::Cpp => &[
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "default",
                "break",
                "continue",
                "return",
                "struct",
                "enum",
                "typedef",
                "const",
                "static",
                "void",
                "int",
                "char",
                "float",
                "double",
                "class",
                "public",
                "private",
                "protected",
                "virtual",
                "new",
                "delete",
                "true",
                "false",
                "nullptr",
            ],
            SyntaxType::Shell => &[
                "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "do", "done",
                "in", "function", "return", "exit", "local", "export", "true", "false",
            ],
            _ => &[],
        }
    }
}

#[derive(Clone, Copy)]
enum TokenType {
    Normal,
    Keyword,
    String,
    Comment,
    Number,
}

struct SyntaxHighlighter {
    syntax_type: SyntaxType,
}

impl SyntaxHighlighter {
    fn new(path: &str) -> Self {
        Self {
            syntax_type: SyntaxType::from_path(path),
        }
    }

    fn highlight_line(&self, line: &str) -> Vec<(String, Color)> {
        if self.syntax_type == SyntaxType::None {
            return vec![(line.to_string(), Color::Rgb(205, 214, 244))];
        }
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let keywords = self.syntax_type.keywords();

        while i < chars.len() {
            // Comments
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                tokens.push((chars[i..].iter().collect(), Color::Rgb(108, 112, 134)));
                break;
            }
            if chars[i] == '#'
                && matches!(
                    self.syntax_type,
                    SyntaxType::Python | SyntaxType::Shell | SyntaxType::Toml
                )
            {
                tokens.push((chars[i..].iter().collect(), Color::Rgb(108, 112, 134)));
                break;
            }
            // Strings
            if chars[i] == '"' || chars[i] == '\'' {
                let q = chars[i];
                let mut j = i + 1;
                while j < chars.len() && chars[j] != q {
                    if chars[j] == '\\' {
                        j += 1;
                    }
                    j += 1;
                }
                if j < chars.len() {
                    j += 1;
                }
                tokens.push((chars[i..j].iter().collect(), Color::Rgb(166, 227, 161)));
                i = j;
                continue;
            }
            // Numbers
            if chars[i].is_ascii_digit() {
                let mut j = i;
                while j < chars.len()
                    && (chars[j].is_ascii_digit()
                        || chars[j] == '.'
                        || chars[j] == 'x'
                        || chars[j].is_ascii_hexdigit())
                {
                    j += 1;
                }
                tokens.push((chars[i..j].iter().collect(), Color::Rgb(250, 179, 135)));
                i = j;
                continue;
            }
            // Identifiers/keywords
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let word: String = chars[i..j].iter().collect();
                let color = if keywords.contains(&word.as_str()) {
                    Color::Rgb(203, 166, 247) // keyword purple
                } else if j < chars.len() && chars[j] == '(' {
                    Color::Rgb(137, 180, 250) // function blue
                } else if word
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    Color::Rgb(249, 226, 175) // type yellow
                } else {
                    Color::Rgb(205, 214, 244) // normal
                };
                tokens.push((word, color));
                i = j;
                continue;
            }
            tokens.push((chars[i].to_string(), Color::Rgb(205, 214, 244)));
            i += 1;
        }
        if tokens.is_empty() {
            tokens.push((line.to_string(), Color::Rgb(205, 214, 244)));
        }
        tokens
    }
}

pub struct DiffView {
    pub diff: DiffInfo,
    pub current_file: usize,
    pub scroll: usize,
    pub h_offset: usize,
    pub show_line_numbers: bool,
    pub mode: DiffMode,
    pub max_content_width: usize,
    pub view_width: usize,
    pub preview_type: PreviewType,
    pub file_content: Option<FileContent>,
    pub pr_preview: Option<PullRequestPreview>,
    pub commit_preview: Option<CommitPreview>,
    pub issue_preview: Option<IssuePreview>,
    // Visual mode (line selection)
    pub visual_mode: bool,
    pub visual_start: usize, // Starting line of selection
    pub cursor_line: usize,  // Current cursor line
    // Search
    pub search_query: Option<String>,
    pub search_matches: Vec<(usize, usize, usize)>, // (line_idx, start_col, end_col)
    pub current_match: usize,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            diff: DiffInfo { files: Vec::new() },
            current_file: 0,
            scroll: 0,
            h_offset: 0,
            show_line_numbers: true,
            mode: DiffMode::Inline,
            max_content_width: 0,
            view_width: 0,
            preview_type: PreviewType::Diff,
            file_content: None,
            pr_preview: None,
            commit_preview: None,
            issue_preview: None,
            visual_mode: false,
            visual_start: 0,
            cursor_line: 0,
            search_query: None,
            search_matches: Vec::new(),
            current_match: 0,
        }
    }

    pub fn set_pr_preview(&mut self, pr: &PullRequestInfo) {
        self.pr_preview = Some(PullRequestPreview::from(pr));
        self.preview_type = PreviewType::PullRequest;
        self.scroll = 0;
    }

    pub fn set_commit_preview(&mut self, commit: &crate::git::CommitInfo) {
        // Simple timestamp formatting
        let secs = commit.time;
        let days = secs / 86400;
        let years = 1970 + days / 365;
        let remaining_days = days % 365;
        let months = remaining_days / 30 + 1;
        let day = remaining_days % 30 + 1;
        let hours = (secs % 86400) / 3600;
        let minutes = (secs % 3600) / 60;
        let date = format!("{:04}-{:02}-{:02} {:02}:{:02}", years, months, day, hours, minutes);

        self.commit_preview = Some(CommitPreview {
            id: commit.id.clone(),
            short_id: commit.short_id.clone(),
            message: commit.message.clone(),
            author: commit.author.clone(),
            email: commit.email.clone(),
            date,
            refs: commit.refs.clone(),
        });
        self.preview_type = PreviewType::Commit;
        self.scroll = 0;
        self.h_offset = 0;
    }

    pub fn clear_commit_preview(&mut self) {
        self.commit_preview = None;
        if self.preview_type == PreviewType::Commit {
            self.preview_type = PreviewType::Diff;
        }
    }

    pub fn clear_pr_preview(&mut self) {
        self.pr_preview = None;
        if self.preview_type == PreviewType::PullRequest {
            self.preview_type = PreviewType::Diff;
        }
    }

    pub fn set_issue_preview(&mut self, issue: &IssueInfo) {
        self.issue_preview = Some(IssuePreview::from(issue));
        self.preview_type = PreviewType::Issue;
        self.scroll = 0;
        self.h_offset = 0;
    }

    pub fn clear_issue_preview(&mut self) {
        self.issue_preview = None;
        if self.preview_type == PreviewType::Issue {
            self.preview_type = PreviewType::Diff;
        }
    }

    pub fn set_mode(&mut self, mode: DiffMode) {
        self.mode = mode;
    }

    pub fn can_scroll_left(&self) -> bool {
        self.h_offset > 0
    }

    pub fn can_scroll_right(&self) -> bool {
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
        self.h_offset += 4;
    }

    pub fn update(&mut self, diff: DiffInfo) {
        self.diff = diff;
        self.current_file = 0;
        self.scroll = 0;
        self.h_offset = 0;
        self.cursor_line = 0;
        self.visual_mode = false;
        self.preview_type = PreviewType::Diff;
        self.file_content = None;
    }

    pub fn set_file_content(&mut self, path: String, content: String) {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        self.file_content = Some(FileContent { path, lines });
        self.preview_type = PreviewType::FileContent;
        self.scroll = 0;
        self.h_offset = 0;
        self.cursor_line = 0;
        self.visual_mode = false;
    }

    pub fn clear(&mut self) {
        self.diff = DiffInfo { files: Vec::new() };
        self.current_file = 0;
        self.scroll = 0;
        self.h_offset = 0;
        self.cursor_line = 0;
        self.visual_mode = false;
        self.file_content = None;
        self.preview_type = PreviewType::Diff;
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        self.diff.files.get(self.current_file)
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    /// Move cursor up (for normal mode navigation)
    pub fn cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            // Scroll to keep cursor visible
            if self.cursor_line < self.scroll {
                self.scroll = self.cursor_line;
            }
        }
    }

    /// Move cursor down (for normal mode navigation)
    pub fn cursor_down(&mut self, visible_height: usize) {
        let max_lines = self.get_total_lines();
        if self.cursor_line < max_lines.saturating_sub(1) {
            self.cursor_line += 1;
            // Scroll to keep cursor visible
            if self.cursor_line >= self.scroll + visible_height {
                self.scroll = self.cursor_line - visible_height + 1;
            }
        }
    }

    /// Move cursor to top
    pub fn cursor_to_top(&mut self) {
        self.cursor_line = 0;
        self.scroll = 0;
    }

    /// Move cursor to bottom
    pub fn cursor_to_bottom(&mut self, visible_height: usize) {
        let max_lines = self.get_total_lines();
        if max_lines > 0 {
            self.cursor_line = max_lines - 1;
            if self.cursor_line >= visible_height {
                self.scroll = self.cursor_line - visible_height + 1;
            }
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
        self.cursor_line = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        // Set to a large number, rendering will clamp it
        let max_lines = self.get_total_lines();
        if max_lines > 0 {
            self.cursor_line = max_lines - 1;
        }
        self.scroll = usize::MAX / 2;
    }

    pub fn select_at_row(&mut self, row: usize) {
        // For diff view, clicking sets the cursor position
        self.cursor_line = self.scroll + row;
    }

    pub fn next_file(&mut self) {
        if self.current_file + 1 < self.diff.files.len() {
            self.current_file += 1;
            self.scroll = 0;
        }
    }

    pub fn prev_file(&mut self) {
        if self.current_file > 0 {
            self.current_file -= 1;
            self.scroll = 0;
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DiffMode::Inline => DiffMode::SideBySide,
            DiffMode::SideBySide => DiffMode::Inline,
        };
        self.scroll = 0;
    }

    pub fn next_hunk(&mut self) {
        // TODO: Implement hunk navigation
    }

    pub fn prev_hunk(&mut self) {
        // TODO: Implement hunk navigation
    }

    /// Enter visual (line selection) mode
    pub fn enter_visual_mode(&mut self) {
        self.visual_mode = true;
        self.visual_start = self.cursor_line;
    }

    /// Exit visual mode
    pub fn exit_visual_mode(&mut self) {
        self.visual_mode = false;
    }

    /// Check if in visual mode
    pub fn is_visual_mode(&self) -> bool {
        self.visual_mode
    }

    /// Move cursor up in visual mode
    pub fn visual_move_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            // Adjust scroll to keep cursor visible
            if self.cursor_line < self.scroll {
                self.scroll = self.cursor_line;
            }
        }
    }

    /// Move cursor down in visual mode
    pub fn visual_move_down(&mut self, max_lines: usize) {
        if self.cursor_line < max_lines.saturating_sub(1) {
            self.cursor_line += 1;
        }
    }

    /// Get the selected line range (start, end) inclusive
    pub fn get_selection_range(&self) -> (usize, usize) {
        if self.visual_start <= self.cursor_line {
            (self.visual_start, self.cursor_line)
        } else {
            (self.cursor_line, self.visual_start)
        }
    }

    /// Get the selected lines as a string (for clipboard)
    pub fn get_selected_text(&self) -> Option<String> {
        if !self.visual_mode {
            return None;
        }

        let (start, end) = self.get_selection_range();

        match self.preview_type {
            PreviewType::FileContent => {
                if let Some(ref fc) = self.file_content {
                    let selected: Vec<&str> = fc
                        .lines
                        .iter()
                        .skip(start)
                        .take(end - start + 1)
                        .map(|s| s.as_str())
                        .collect();
                    Some(selected.join("\n"))
                } else {
                    None
                }
            }
            PreviewType::Diff => {
                // For diff, collect lines from hunks
                if let Some(file) = self.diff.files.get(self.current_file) {
                    let mut all_lines: Vec<String> = Vec::new();
                    for hunk in &file.hunks {
                        all_lines.push(hunk.header.clone());
                        for line in &hunk.lines {
                            all_lines.push(line.content.clone());
                        }
                    }
                    let selected: Vec<&str> = all_lines
                        .iter()
                        .skip(start)
                        .take(end - start + 1)
                        .map(|s| s.as_str())
                        .collect();
                    Some(selected.join(""))
                } else {
                    None
                }
            }
            PreviewType::PullRequest => {
                // PR preview doesn't support text selection
                None
            }
            PreviewType::Commit => {
                // Commit preview doesn't support text selection
                None
            }
            PreviewType::Issue => {
                // Issue preview doesn't support text selection
                None
            }
        }
    }

    /// Get total line count for the current content
    pub fn get_total_lines(&self) -> usize {
        match self.preview_type {
            PreviewType::FileContent => self
                .file_content
                .as_ref()
                .map(|fc| fc.lines.len())
                .unwrap_or(0),
            PreviewType::Diff => {
                if let Some(file) = self.diff.files.get(self.current_file) {
                    let mut count = 0;
                    for hunk in &file.hunks {
                        count += 1; // header
                        count += hunk.lines.len();
                    }
                    count
                } else {
                    0
                }
            }
            PreviewType::PullRequest => {
                // PR preview line count not tracked for visual mode
                0
            }
            PreviewType::Commit => {
                // Commit preview line count not tracked for visual mode
                0
            }
            PreviewType::Issue => {
                // Issue preview line count not tracked for visual mode
                0
            }
        }
    }

    /// Adjust scroll to keep cursor visible (called after cursor movement)
    pub fn ensure_cursor_visible(&mut self, visible_height: usize) {
        if self.cursor_line < self.scroll {
            self.scroll = self.cursor_line;
        } else if self.cursor_line >= self.scroll + visible_height {
            self.scroll = self.cursor_line - visible_height + 1;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_matches.clear();
        self.current_match = 0;

        let query_lower = query.to_lowercase();

        // Get lines based on current preview type
        let lines: Vec<String> = match self.preview_type {
            PreviewType::FileContent => self
                .file_content
                .as_ref()
                .map(|fc| fc.lines.clone())
                .unwrap_or_default(),
            PreviewType::Diff => {
                let mut lines = Vec::new();
                if let Some(file) = self.diff.files.get(self.current_file) {
                    for hunk in &file.hunks {
                        lines.push(hunk.header.clone());
                        for line in &hunk.lines {
                            lines.push(line.content.clone());
                        }
                    }
                }
                lines
            }
            PreviewType::PullRequest => {
                // PR preview: search in body
                if let Some(ref pr) = self.pr_preview {
                    pr.body.lines().map(|s| s.to_string()).collect()
                } else {
                    Vec::new()
                }
            }
            PreviewType::Commit => {
                // Commit preview: search in message
                if let Some(ref commit) = self.commit_preview {
                    commit.message.lines().map(|s| s.to_string()).collect()
                } else {
                    Vec::new()
                }
            }
            PreviewType::Issue => {
                // Issue preview: search in body
                if let Some(ref issue) = self.issue_preview {
                    issue.body.lines().map(|s| s.to_string()).collect()
                } else {
                    Vec::new()
                }
            }
        };

        // Find all matches
        for (line_idx, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query_lower) {
                let match_start = start + pos;
                let match_end = match_start + query.len();
                self.search_matches.push((line_idx, match_start, match_end));
                start = match_end;
            }
        }

        // Jump to first result
        if !self.search_matches.is_empty() {
            let (line_idx, _, _) = self.search_matches[0];
            self.scroll = line_idx.saturating_sub(5); // Show some context above
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_matches.clear();
        self.current_match = 0;
    }

    pub fn next_search_result(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match = (self.current_match + 1) % self.search_matches.len();
        let (line_idx, _, _) = self.search_matches[self.current_match];
        self.scroll = line_idx.saturating_sub(5);
    }

    pub fn prev_search_result(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        if self.current_match == 0 {
            self.current_match = self.search_matches.len() - 1;
        } else {
            self.current_match -= 1;
        }
        let (line_idx, _, _) = self.search_matches[self.current_match];
        self.scroll = line_idx.saturating_sub(5);
    }

    /// Check if a line has search matches and return them
    fn get_line_search_matches(&self, line_idx: usize) -> Vec<(usize, usize)> {
        self.search_matches
            .iter()
            .filter(|(idx, _, _)| *idx == line_idx)
            .map(|(_, start, end)| (*start, *end))
            .collect()
    }

    /// Check if this match is the current/highlighted one
    fn is_current_match(&self, line_idx: usize, start: usize) -> bool {
        if self.search_matches.is_empty() {
            return false;
        }
        let (cur_line, cur_start, _) = self.search_matches[self.current_match];
        cur_line == line_idx && cur_start == start
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) {
        let border_color = if focused {
            theme.border_focused
        } else {
            theme.border_unfocused
        };

        let visual_indicator = if self.visual_mode { " [VISUAL] " } else { "" };

        let title = match self.preview_type {
            PreviewType::Diff => {
                let mode_indicator = match self.mode {
                    DiffMode::Inline => "inline",
                    DiffMode::SideBySide => "split",
                };

                if let Some(file) = self.current_file() {
                    format!(
                        " Preview: {} (+{} -{}) [{}]{} ",
                        file.path,
                        file.additions(),
                        file.deletions(),
                        mode_indicator,
                        visual_indicator
                    )
                } else {
                    format!(" Preview [{}]{} ", mode_indicator, visual_indicator)
                }
            }
            PreviewType::FileContent => {
                if let Some(ref fc) = self.file_content {
                    format!(" Preview: {}{} ", fc.path, visual_indicator)
                } else {
                    format!(" Preview{} ", visual_indicator)
                }
            }
            PreviewType::PullRequest => {
                if let Some(ref pr) = self.pr_preview {
                    let state = if pr.is_draft { "DRAFT" } else { &pr.state };
                    format!(" PR #{} [{}] ", pr.number, state)
                } else {
                    " Pull Request ".to_string()
                }
            }
            PreviewType::Commit => {
                if let Some(ref commit) = self.commit_preview {
                    format!(" Commit: {} ", commit.short_id)
                } else {
                    " Commit ".to_string()
                }
            }
            PreviewType::Issue => {
                if let Some(ref issue) = self.issue_preview {
                    format!(" Issue #{} [{}] ", issue.number, issue.state)
                } else {
                    " Issue ".to_string()
                }
            }
        };

        let block = Block::new()
            .title(&title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        // Clear the inner area first to prevent old content from showing
        // Reset each cell individually to handle wide characters properly
        for row in 0..inner.height {
            for col in 0..inner.width {
                let cell = buf.get_mut(inner.x + col, inner.y + row);
                cell.reset();
            }
        }

        match self.preview_type {
            PreviewType::FileContent => self.render_file_content(inner, buf, theme),
            PreviewType::Diff => match self.mode {
                DiffMode::Inline => self.render_inline(inner, buf, theme),
                DiffMode::SideBySide => self.render_side_by_side(inner, buf, theme),
            },
            PreviewType::PullRequest => self.render_pr_preview(inner, buf, theme),
            PreviewType::Commit => self.render_commit_preview(inner, buf, theme),
            PreviewType::Issue => self.render_issue_preview(inner, buf, theme),
        }
    }

    fn render_issue_preview(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
        let Some(ref issue) = self.issue_preview else {
            let msg = "No issue selected";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
            return;
        };

        let width = inner.width as usize;
        let mut lines: Vec<(String, Color)> = Vec::new();

        // Title
        lines.push((format!("#{} {}", issue.number, issue.title), theme.branch_current));

        // State
        let state_color = if issue.state == "OPEN" {
            theme.diff_add
        } else {
            theme.diff_remove
        };
        lines.push((format!("State: {}", issue.state), state_color));

        // Author and date
        lines.push((format!("Author: {}", issue.author), theme.foreground));
        lines.push((format!("Created: {}", issue.created_at), theme.foreground));

        // Labels
        if !issue.labels.is_empty() {
            lines.push((format!("Labels: {}", issue.labels.join(", ")), theme.diff_hunk));
        }

        // Comments count
        lines.push((format!("Comments: {}", issue.comment_list.len()), theme.foreground));

        // URL
        if !issue.url.is_empty() {
            lines.push((format!("URL: {}", issue.url), theme.branch_remote));
        }

        // Empty line before body
        lines.push((String::new(), theme.foreground));

        // Body (may be multiline)
        lines.push(("─── Description ───".to_string(), theme.diff_hunk));
        if issue.body.is_empty() {
            lines.push(("(No description)".to_string(), theme.untracked));
        } else {
            for line in issue.body.lines() {
                lines.push((line.to_string(), theme.foreground));
            }
        }

        // Comments
        if !issue.comment_list.is_empty() {
            lines.push((String::new(), theme.foreground));
            lines.push((format!("─── Comments ({}) ───", issue.comment_list.len()), theme.diff_hunk));
            for comment in &issue.comment_list {
                lines.push((String::new(), theme.foreground));
                lines.push((
                    format!("@{} - {}", comment.author, comment.created_at),
                    theme.branch_current,
                ));
                for line in comment.body.lines() {
                    lines.push((format!("  {}", line), theme.foreground));
                }
            }
        }

        // Render with scrolling
        let visible_height = inner.height as usize;
        let total_lines = lines.len();

        // Clamp scroll
        if self.scroll > total_lines.saturating_sub(visible_height) {
            self.scroll = total_lines.saturating_sub(visible_height);
        }

        for (i, (line, color)) in lines.iter().skip(self.scroll).take(visible_height).enumerate() {
            let y = inner.y + i as u16;
            let display_line: String = line.chars().skip(self.h_offset).take(width).collect();
            buf.set_string(inner.x, y, &display_line, Style::new().fg(*color));
        }

        // Render scrollbar
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(total_lines, visible_height, self.scroll);
            let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
            scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
        }
    }

    fn render_commit_preview(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
        let Some(ref commit) = self.commit_preview else {
            let msg = "No commit selected";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
            return;
        };

        let width = inner.width as usize;
        let mut lines: Vec<(String, Color)> = Vec::new();

        // Header info
        lines.push((format!("commit {}", commit.id), theme.diff_hunk));
        lines.push((format!("Author: {} <{}>", commit.author, commit.email), theme.foreground));
        lines.push((format!("Date:   {}", commit.date), theme.foreground));

        // Refs (if any)
        if !commit.refs.is_empty() {
            lines.push((format!("Refs:   {}", commit.refs.join(", ")), theme.branch_current));
        }

        // Empty line before message
        lines.push((String::new(), theme.foreground));

        // Message (may be multiline)
        for line in commit.message.lines() {
            lines.push((format!("    {}", line), theme.foreground));
        }

        // Render with scrolling
        let visible_height = inner.height as usize;
        let total_lines = lines.len();

        // Clamp scroll
        if self.scroll > total_lines.saturating_sub(visible_height) {
            self.scroll = total_lines.saturating_sub(visible_height);
        }

        for (i, (line, color)) in lines.iter().skip(self.scroll).take(visible_height).enumerate() {
            let y = inner.y + i as u16;
            let display_line: String = line.chars().skip(self.h_offset).take(width).collect();
            buf.set_string(inner.x, y, &display_line, Style::new().fg(*color));
        }

        // Render scrollbar
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(total_lines, visible_height, self.scroll);
            let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
            scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
        }
    }

    fn render_pr_preview(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
        let Some(ref pr) = self.pr_preview else {
            let msg = "No pull request selected";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
            return;
        };

        // Build display lines
        let mut lines: Vec<(String, Style)> = Vec::new();

        // Title
        lines.push((pr.title.clone(), Style::new().fg(theme.foreground).bold()));
        lines.push((String::new(), Style::new()));

        // Metadata
        let state_color = if pr.is_draft {
            theme.border_unfocused
        } else {
            match pr.state.as_str() {
                "OPEN" => theme.diff_add,
                "MERGED" => theme.diff_hunk,
                "CLOSED" => theme.diff_remove,
                _ => theme.foreground,
            }
        };
        let state_str = if pr.is_draft {
            "DRAFT".to_string()
        } else {
            pr.state.clone()
        };
        lines.push((
            format!("State: {}", state_str),
            Style::new().fg(state_color),
        ));
        lines.push((
            format!("Author: {}", pr.author),
            Style::new().fg(theme.foreground),
        ));
        lines.push((
            format!("Branch: {} <- {}", pr.base_branch, pr.head_branch),
            Style::new().fg(theme.branch_local),
        ));
        lines.push((
            format!("Changes: +{} -{}", pr.additions, pr.deletions),
            Style::new().fg(theme.foreground),
        ));
        if !pr.created_at.is_empty() {
            // Format date (just take the date part if it's ISO format)
            let date = pr.created_at.split('T').next().unwrap_or(&pr.created_at);
            lines.push((
                format!("Created: {}", date),
                Style::new().fg(theme.border_unfocused),
            ));
        }
        if !pr.url.is_empty() {
            lines.push((
                format!("URL: {}", pr.url),
                Style::new().fg(theme.border_unfocused),
            ));
        }

        // Separator
        lines.push((String::new(), Style::new()));
        lines.push((
            "─".repeat(inner.width.saturating_sub(2) as usize),
            Style::new().fg(theme.border),
        ));
        lines.push((String::new(), Style::new()));

        // Body
        if pr.body.is_empty() {
            lines.push((
                "No description provided.".to_string(),
                Style::new().fg(theme.border_unfocused),
            ));
        } else {
            // Split body into lines, handling long lines
            for line in pr.body.lines() {
                // Wrap long lines
                let max_width = inner.width.saturating_sub(2) as usize;
                if line.chars().count() <= max_width {
                    lines.push((line.to_string(), Style::new().fg(theme.foreground)));
                } else {
                    // Simple word wrap
                    let mut current_line = String::new();
                    for word in line.split_whitespace() {
                        if current_line.is_empty() {
                            current_line = word.to_string();
                        } else if current_line.chars().count() + 1 + word.chars().count()
                            <= max_width
                        {
                            current_line.push(' ');
                            current_line.push_str(word);
                        } else {
                            lines.push((current_line, Style::new().fg(theme.foreground)));
                            current_line = word.to_string();
                        }
                    }
                    if !current_line.is_empty() {
                        lines.push((current_line, Style::new().fg(theme.foreground)));
                    }
                }
            }
        }

        // Scroll handling
        let visible_height = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(visible_height);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }

        // Render lines
        let content_width = inner.width.saturating_sub(1); // Leave space for scrollbar
        for (i, (line, style)) in lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            buf.set_string_truncated(inner.x, y, line, content_width, *style);
        }

        // Scrollbar
        if lines.len() > visible_height {
            let scrollbar =
                crate::widgets::Scrollbar::new(lines.len(), visible_height, self.scroll);
            let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
            scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
        }
    }

    fn render_inline(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
        // Collect all lines from all hunks (owned data to avoid borrow issues)
        let lines: Vec<(Option<u32>, Option<u32>, LineType, String)> = {
            let Some(file) = self.current_file() else {
                // No diff to show
                let msg = "No changes to display";
                let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
                return;
            };

            let mut lines = Vec::new();
            for hunk in &file.hunks {
                // Hunk header
                lines.push((None, None, LineType::Context, hunk.header.clone()));

                for line in &hunk.lines {
                    lines.push((
                        line.old_lineno,
                        line.new_lineno,
                        line.line_type,
                        line.content.clone(),
                    ));
                }
            }
            lines
        };

        // Adjust scroll (now safe because we don't hold a borrow on self)
        let max_scroll = lines.len().saturating_sub(inner.height as usize);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }

        let line_num_width = if self.show_line_numbers { 8 } else { 0 };
        let visible_height = inner.height as usize;
        let content_area_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        // Calculate max content width using display width (accounting for wide characters)
        self.view_width = (content_area_width.saturating_sub(line_num_width)) as usize;
        self.max_content_width = lines
            .iter()
            .map(|(_, _, _, content)| {
                str_display_width(content.trim_end_matches('\n')) + 1 // +1 for prefix
            })
            .max()
            .unwrap_or(0)
            + 2; // +2 for scrollbar (1) + margin (1)

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        for (i, (old_line, new_line, line_type, content)) in lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;

            // Line numbers
            if self.show_line_numbers {
                let old_str = old_line
                    .map(|n| format!("{:>3}", n))
                    .unwrap_or_else(|| "   ".to_string());
                let new_str = new_line
                    .map(|n| format!("{:>3}", n))
                    .unwrap_or_else(|| "   ".to_string());
                let line_nums = format!("{} {} |", old_str, new_str);
                buf.set_string(
                    inner.x,
                    y,
                    &line_nums,
                    Style::new().fg(theme.untracked).dim(),
                );
            }

            // Line content
            let content_x = inner.x + line_num_width;
            let content_width = content_area_width.saturating_sub(line_num_width);

            let (prefix, style) = match line_type {
                LineType::Addition => {
                    ("+", Style::new().fg(theme.foreground).bg(theme.diff_add_bg))
                }
                LineType::Deletion => (
                    "-",
                    Style::new().fg(theme.foreground).bg(theme.diff_remove_bg),
                ),
                LineType::Context => {
                    if content.starts_with("@@") {
                        (" ", Style::new().fg(theme.diff_hunk).bold())
                    } else {
                        (" ", Style::new().fg(theme.foreground))
                    }
                }
            };

            // Fill the entire line with background color for additions/deletions
            // Use set_string_truncated to respect the content_width boundary
            if matches!(line_type, LineType::Addition | LineType::Deletion) {
                let blank = " ".repeat(content_width as usize);
                buf.set_string_truncated(content_x, y, &blank, content_width, style);
            }

            buf.set_string(content_x, y, prefix, style);

            // Remove trailing newline from content and render character by character
            // to properly handle wide characters at the boundary
            let content = content.trim_end_matches('\n');
            let max_content_chars = content_width.saturating_sub(1); // -1 for prefix

            // Get search matches for this line
            let absolute_line = self.scroll + i;
            let line_matches = self.get_line_search_matches(absolute_line);

            let mut x_offset: u16 = 0;
            let mut char_idx: usize = 0;
            for c in content.chars() {
                let cw = unicode_width(c) as u16;
                if x_offset + cw > max_content_chars {
                    break;
                }

                // Check if this character is part of a search match
                let mut char_style = style;
                for (match_start, match_end) in &line_matches {
                    if char_idx >= *match_start && char_idx < *match_end {
                        // Highlight matched text
                        let is_current = self.is_current_match(absolute_line, *match_start);
                        if is_current {
                            char_style = Style::new().fg(theme.foreground).bg(theme.diff_hunk);
                        } else {
                            char_style = Style::new().fg(theme.diff_hunk);
                        }
                        break;
                    }
                }

                buf.get_mut(content_x + 1 + x_offset, y)
                    .set_char(c)
                    .set_style(char_style);
                if cw == 2 && x_offset + cw <= max_content_chars {
                    buf.get_mut(content_x + 1 + x_offset + 1, y)
                        .set_symbol("")
                        .set_style(char_style);
                }
                x_offset += cw;
                char_idx += 1;
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(lines.len(), visible_height, self.scroll);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn render_side_by_side(&mut self, inner: Rect, buf: &mut Buffer, theme: &Theme) {
        // Build paired lines for side-by-side view
        let paired_lines: Vec<(Option<(u32, String)>, Option<(u32, String)>)> = {
            let Some(file) = self.current_file() else {
                let msg = "No changes to display";
                let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
                let y = inner.y + inner.height / 2;
                buf.set_string(x, y, msg, Style::new().fg(theme.untracked));
                return;
            };

            let mut pairs = Vec::new();

            for hunk in &file.hunks {
                // Hunk header spans both sides
                pairs.push((
                    Some((0, hunk.header.clone())),
                    Some((0, hunk.header.clone())),
                ));

                // Collect deletions and additions separately, then pair them
                let mut deletions: Vec<(u32, String)> = Vec::new();
                let mut additions: Vec<(u32, String)> = Vec::new();

                for line in &hunk.lines {
                    match line.line_type {
                        LineType::Context => {
                            // Flush any pending deletions/additions
                            Self::flush_pairs(&mut pairs, &mut deletions, &mut additions);

                            let old_no = line.old_lineno.unwrap_or(0);
                            let new_no = line.new_lineno.unwrap_or(0);
                            let content = line.content.trim_end_matches('\n').to_string();
                            pairs.push((Some((old_no, content.clone())), Some((new_no, content))));
                        }
                        LineType::Deletion => {
                            let line_no = line.old_lineno.unwrap_or(0);
                            let content = line.content.trim_end_matches('\n').to_string();
                            deletions.push((line_no, content));
                        }
                        LineType::Addition => {
                            let line_no = line.new_lineno.unwrap_or(0);
                            let content = line.content.trim_end_matches('\n').to_string();
                            additions.push((line_no, content));
                        }
                    }
                }

                // Flush remaining
                Self::flush_pairs(&mut pairs, &mut deletions, &mut additions);
            }

            pairs
        };

        // Adjust scroll
        let max_scroll = paired_lines.len().saturating_sub(inner.height as usize);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }

        let visible_height = inner.height as usize;
        let total_width = inner.width.saturating_sub(1); // Leave space for scrollbar
        let half_width = total_width / 2;
        let line_num_width: u16 = 4;

        // Calculate max content width for side-by-side mode (using display width)
        self.view_width = (half_width.saturating_sub(line_num_width + 1)) as usize;
        self.max_content_width = paired_lines
            .iter()
            .map(|(left, right)| {
                let left_len = left
                    .as_ref()
                    .map(|(_, c)| str_display_width(c))
                    .unwrap_or(0);
                let right_len = right
                    .as_ref()
                    .map(|(_, c)| str_display_width(c))
                    .unwrap_or(0);
                left_len.max(right_len)
            })
            .max()
            .unwrap_or(0)
            + 2; // +2 for scrollbar (1) + margin (1)

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        // Draw separator line (seamless vertical line)
        let sep_x = inner.x + half_width;
        for y in inner.y..inner.y + inner.height {
            buf.set_string(sep_x, y, "│", Style::new().fg(theme.border));
        }

        for (i, (left, right)) in paired_lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let left_content_width = half_width.saturating_sub(line_num_width + 1);
            let right_content_width = half_width.saturating_sub(line_num_width + 2);

            // Left side (old/deletion)
            if let Some((line_no, content)) = left {
                let is_hunk_header = content.starts_with("@@");
                let is_deletion = right.is_none()
                    || (right.is_some()
                        && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c));

                let style = if is_hunk_header {
                    Style::new().fg(theme.diff_hunk).bold()
                } else if is_deletion && !right.is_some() {
                    Style::new().fg(theme.foreground).bg(theme.diff_remove_bg)
                } else if is_deletion
                    && right.is_some()
                    && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c)
                {
                    Style::new().fg(theme.foreground).bg(theme.diff_remove_bg)
                } else {
                    Style::new().fg(theme.foreground)
                };

                // Fill line with background color for deletions
                let is_colored = is_deletion && !is_hunk_header;
                if is_colored {
                    let blank = " ".repeat(left_content_width as usize);
                    buf.set_string_truncated(
                        inner.x + line_num_width,
                        y,
                        &blank,
                        left_content_width,
                        style,
                    );
                }

                // Line number
                if *line_no > 0 {
                    let num_str = format!("{:>3} ", line_no);
                    buf.set_string(inner.x, y, &num_str, Style::new().fg(theme.untracked).dim());
                } else {
                    buf.set_string(inner.x, y, "    ", Style::new().fg(theme.untracked).dim());
                }

                // Content - render character by character for wide char handling
                let mut x_off: u16 = 0;
                for c in content.chars() {
                    let cw = unicode_width(c) as u16;
                    if x_off + cw > left_content_width {
                        break;
                    }
                    buf.get_mut(inner.x + line_num_width + x_off, y)
                        .set_char(c)
                        .set_style(style);
                    if cw == 2 && x_off + cw <= left_content_width {
                        buf.get_mut(inner.x + line_num_width + x_off + 1, y)
                            .set_symbol("")
                            .set_style(style);
                    }
                    x_off += cw;
                }
            }

            // Right side (new/addition)
            let right_x = sep_x + 1;
            if let Some((line_no, content)) = right {
                let is_hunk_header = content.starts_with("@@");
                let is_addition = left.is_none()
                    || (left.is_some()
                        && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c));

                let style = if is_hunk_header {
                    Style::new().fg(theme.diff_hunk).bold()
                } else if is_addition && !left.is_some() {
                    Style::new().fg(theme.foreground).bg(theme.diff_add_bg)
                } else if is_addition
                    && left.is_some()
                    && left.as_ref().map(|(_, c)| c) != right.as_ref().map(|(_, c)| c)
                {
                    Style::new().fg(theme.foreground).bg(theme.diff_add_bg)
                } else {
                    Style::new().fg(theme.foreground)
                };

                // Fill line with background color for additions
                let is_colored = is_addition && !is_hunk_header;
                if is_colored {
                    let blank = " ".repeat(right_content_width as usize);
                    buf.set_string_truncated(
                        right_x + line_num_width,
                        y,
                        &blank,
                        right_content_width,
                        style,
                    );
                }

                // Line number
                if *line_no > 0 {
                    let num_str = format!("{:>3} ", line_no);
                    buf.set_string(right_x, y, &num_str, Style::new().fg(theme.untracked).dim());
                } else {
                    buf.set_string(right_x, y, "    ", Style::new().fg(theme.untracked).dim());
                }

                // Content - render character by character for wide char handling
                let mut x_off: u16 = 0;
                for c in content.chars() {
                    let cw = unicode_width(c) as u16;
                    if x_off + cw > right_content_width {
                        break;
                    }
                    buf.get_mut(right_x + line_num_width + x_off, y)
                        .set_char(c)
                        .set_style(style);
                    if cw == 2 && x_off + cw <= right_content_width {
                        buf.get_mut(right_x + line_num_width + x_off + 1, y)
                            .set_symbol("")
                            .set_style(style);
                    }
                    x_off += cw;
                }
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(paired_lines.len(), visible_height, self.scroll);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(scrollbar_area, buf, Style::new().fg(theme.border));
    }

    fn render_file_content(&mut self, inner: Rect, buf: &mut Buffer, _theme: &Theme) {
        let visible_height = inner.height as usize;
        let content_area_width = inner.width.saturating_sub(1); // Leave space for scrollbar

        let (lines, path) = if let Some(ref fc) = self.file_content {
            (fc.lines.clone(), fc.path.clone())
        } else {
            let msg = "No file selected";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::new().fg(Color::Rgb(108, 112, 134)));
            return;
        };

        // Adjust scroll
        let max_scroll = lines.len().saturating_sub(inner.height as usize);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }

        let line_num_width = if self.show_line_numbers { 6 } else { 0 };

        // Calculate max content width using display width (accounting for wide characters)
        self.view_width = (content_area_width.saturating_sub(line_num_width)) as usize;
        self.max_content_width = lines
            .iter()
            .map(|line| str_display_width(line))
            .max()
            .unwrap_or(0)
            + 2;

        // Clamp h_offset
        if self.max_content_width <= self.view_width {
            self.h_offset = 0;
        } else {
            let max_offset = self.max_content_width.saturating_sub(self.view_width);
            if self.h_offset > max_offset {
                self.h_offset = max_offset;
            }
        }

        // Get selection range for visual mode
        let (sel_start, sel_end) = if self.visual_mode {
            self.get_selection_range()
        } else {
            (usize::MAX, usize::MAX)
        };

        for (i, content) in lines
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let absolute_line = self.scroll + i;
            let line_no = absolute_line + 1;

            // Check if this line is selected in visual mode or is the cursor line
            let is_selected =
                self.visual_mode && absolute_line >= sel_start && absolute_line <= sel_end;
            let is_cursor_line = absolute_line == self.cursor_line;
            let selection_bg = Color::Rgb(60, 60, 100); // Dark blue selection
            let cursor_bg = Color::Rgb(45, 45, 55); // Subtle highlight for cursor line

            // Line numbers
            if self.show_line_numbers {
                let line_nums = format!("{:>4} │", line_no);
                let num_style = if is_selected {
                    Style::new().fg(Color::Rgb(108, 112, 134)).bg(selection_bg)
                } else if is_cursor_line {
                    Style::new().fg(Color::Rgb(150, 150, 170)).bg(cursor_bg)
                } else {
                    Style::new().fg(Color::Rgb(108, 112, 134)).dim()
                };
                buf.set_string(inner.x, y, &line_nums, num_style);
            }

            // Line content with syntax highlighting
            let content_x = inner.x + line_num_width;
            let content_width = content_area_width.saturating_sub(line_num_width);

            // Fill background if selected or cursor line
            if is_selected {
                let blank = " ".repeat(content_width as usize);
                buf.set_string_truncated(
                    content_x,
                    y,
                    &blank,
                    content_width,
                    Style::new().bg(selection_bg),
                );
            } else if is_cursor_line {
                let blank = " ".repeat(content_width as usize);
                buf.set_string_truncated(
                    content_x,
                    y,
                    &blank,
                    content_width,
                    Style::new().bg(cursor_bg),
                );
            }

            // Get search matches for this line (for word-level highlighting)
            let line_matches = self.get_line_search_matches(absolute_line);

            // Render with syntax highlighting, using display width for positioning
            let highlighter = SyntaxHighlighter::new(&path);
            let tokens = highlighter.highlight_line(content);
            let mut x_offset: u16 = 0;
            let mut char_idx: usize = 0;
            for (text, color) in tokens {
                if x_offset >= content_width {
                    break;
                }
                let base_style = if is_selected {
                    Style::new().fg(color).bg(selection_bg)
                } else if is_cursor_line {
                    Style::new().fg(color).bg(cursor_bg)
                } else {
                    Style::new().fg(color)
                };

                // Render character by character to handle wide chars at boundary
                for c in text.chars() {
                    let cw = unicode_width(c) as u16;
                    // Stop if this character would exceed the content area
                    if x_offset + cw > content_width {
                        break;
                    }

                    // Check if this character is part of a search match (word-level highlighting)
                    let mut char_style = base_style;
                    for (match_start, match_end) in &line_matches {
                        if char_idx >= *match_start && char_idx < *match_end {
                            let is_current = self.is_current_match(absolute_line, *match_start);
                            if is_current {
                                char_style = Style::new()
                                    .fg(Color::Rgb(30, 30, 30))
                                    .bg(Color::Rgb(255, 200, 100)); // Current match: bright yellow bg
                            } else {
                                char_style = Style::new().fg(Color::Rgb(255, 200, 100));
                                // Other matches: yellow text
                            }
                            break;
                        }
                    }

                    buf.get_mut(content_x + x_offset, y)
                        .set_char(c)
                        .set_style(char_style);
                    // Mark continuation cell for wide characters
                    if cw == 2 {
                        buf.get_mut(content_x + x_offset + 1, y)
                            .set_symbol("")
                            .set_style(char_style);
                    }
                    x_offset += cw;
                    char_idx += 1;
                }
            }
        }

        // Render scrollbar
        let scrollbar = Scrollbar::new(lines.len(), visible_height, self.scroll);
        let scrollbar_area = Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height);
        scrollbar.render(
            scrollbar_area,
            buf,
            Style::new().fg(Color::Rgb(108, 112, 134)),
        );
    }

    fn flush_pairs(
        pairs: &mut Vec<(Option<(u32, String)>, Option<(u32, String)>)>,
        deletions: &mut Vec<(u32, String)>,
        additions: &mut Vec<(u32, String)>,
    ) {
        let max_len = deletions.len().max(additions.len());
        for i in 0..max_len {
            let left = deletions.get(i).cloned();
            let right = additions.get(i).cloned();
            pairs.push((left, right));
        }
        deletions.clear();
        additions.clear();
    }
}
