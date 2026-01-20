# Feature Specifications

## Overview

Detailed specifications for all 14 features.

---

## 1. Branch List Display

### Description

Display all local and remote branches in a scrollable list.

### UI

```
┌─────────────────────┐
│ [2] Branches        │
│ ─────────────────── │
│ Local               │
│ ● main              │  ← Current branch (green bullet)
│   feature/auth      │
│   feature/ui        │
│ Remote              │
│   origin/main       │
│   origin/develop    │
└─────────────────────┘
```

### Data Model

```rust
pub struct BranchView {
    pub local_branches: Vec<BranchItem>,
    pub remote_branches: Vec<BranchItem>,
    pub show_remote: bool,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

pub struct BranchItem {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub tracking: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}
```

### Actions

- `t` - Toggle remote branches visibility
- `Enter` - Checkout selected branch
- `/` - Search branches

---

## 2. Current Branch Display

### Description

Always visible in header bar, showing current branch name.

### UI (Header)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ g - [repo-name] | branch: main | ↑2 ↓0 | ✓ clean                        │
└─────────────────────────────────────────────────────────────────────────┘
                         ^^^^^^^
                    Current branch
```

### Detached HEAD

```
│ g - [repo-name] | HEAD: a1b2c3d | detached                              │
```

### Data Model

```rust
pub struct HeaderState {
    pub repo_name: String,
    pub branch: Option<String>,      // None if detached
    pub commit_hash: Option<String>, // Shown if detached
    pub ahead: usize,
    pub behind: usize,
    pub is_clean: bool,
}
```

---

## 3. Commit History Display

### Description

Scrollable list of commits with hash, message, author, and date.

### UI

```
┌─────────────────────────────────────────────────────────────────────────┐
│ [3] Commits                                                             │
│ ─────────────────────────────────────────────────────────────────────── │
│ a1b2c3d  Add user authentication             John Doe    2 hours ago    │
│ e4f5g6h  Fix login redirect bug              Jane Smith  5 hours ago    │
│ i7j8k9l  Update dependencies                 John Doe    1 day ago      │
│ m1n2o3p  Initial commit                      Jane Smith  3 days ago     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Data Model

```rust
pub struct CommitView {
    pub commits: Vec<CommitItem>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub total_count: usize,
    pub loading_more: bool,
}

pub struct CommitItem {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date: DateTime,
    pub date_relative: String,
}
```

### Lazy Loading

- Initially load `max_commits` (default: 1000)
- Load more when scrolling near bottom
- Show loading indicator

---

## 4. Commit History Search

### Description

Search commits by message or author name.

### UI

```
┌───────────────────────────────────────┐
│  Search commits: /fix bug█            │
│                                       │
│  Results: 3 commits                   │
│  [Enter] search  [n/N] next/prev      │
└───────────────────────────────────────┘
```

### Search Modes

- Message search (default)
- Author search (prefix with `@`)
- Hash search (prefix with `#`)

### Data Model

```rust
pub struct SearchState {
    pub query: String,
    pub cursor: usize,
    pub mode: SearchMode,
    pub results: Vec<usize>,  // Indices into commit list
    pub current_result: usize,
}

pub enum SearchMode {
    Message,
    Author,
    Hash,
}
```

### Implementation

```rust
impl CommitView {
    pub fn search(&mut self, query: &str) -> Vec<usize> {
        let (mode, query) = Self::parse_search_query(query);

        self.commits.iter().enumerate()
            .filter(|(_, commit)| match mode {
                SearchMode::Message => commit.message.to_lowercase()
                    .contains(&query.to_lowercase()),
                SearchMode::Author => commit.author.to_lowercase()
                    .contains(&query.to_lowercase()),
                SearchMode::Hash => commit.hash.starts_with(&query),
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn parse_search_query(query: &str) -> (SearchMode, &str) {
        if let Some(q) = query.strip_prefix('@') {
            (SearchMode::Author, q)
        } else if let Some(q) = query.strip_prefix('#') {
            (SearchMode::Hash, q)
        } else {
            (SearchMode::Message, query)
        }
    }
}
```

---

## 5. Branch Switching

### Description

Checkout a different branch.

### Flow

1. User selects branch in Branch panel
2. Press `Enter`
3. If working tree is clean: switch immediately
4. If working tree is dirty: show confirmation dialog

### Dirty Working Tree Dialog

```
┌─────────────────────────────────────────┐
│  Working tree has uncommitted changes   │
│                                         │
│  [s] Stash and switch                   │
│  [d] Discard and switch                 │
│  [c] Cancel                             │
└─────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn checkout_branch(&mut self, name: &str) -> Result<(), Error> {
        if !self.repo.is_clean()? {
            self.show_dialog(Dialog::DirtyCheckout {
                branch: name.to_string(),
            });
            return Ok(());
        }

        self.repo.switch_branch(name)?;
        self.refresh_all();
        Ok(())
    }

    pub fn handle_dirty_checkout(&mut self, choice: DirtyChoice) -> Result<(), Error> {
        match choice {
            DirtyChoice::Stash => {
                self.repo.stash_save(Some("Auto-stash before checkout"))?;
                self.repo.switch_branch(&self.pending_branch)?;
            }
            DirtyChoice::Discard => {
                self.repo.discard_all()?;
                self.repo.switch_branch(&self.pending_branch)?;
            }
            DirtyChoice::Cancel => {}
        }
        self.close_dialog();
        Ok(())
    }
}
```

---

## 6. Branch Create/Delete

### Description

Create new branches and delete existing ones.

### Create Branch Dialog

```
┌─────────────────────────────────────────┐
│  Create new branch                      │
│  ┌─────────────────────────────────────┐│
│  │ feature/new-feature█               ││
│  └─────────────────────────────────────┘│
│  Base: main (current)                   │
│                                         │
│  [Enter] create  [Tab] change base      │
└─────────────────────────────────────────┘
```

### Delete Branch Confirmation

```
┌─────────────────────────────────────────┐
│  Delete branch 'feature/old'?           │
│                                         │
│  This branch has 3 unmerged commits.    │
│                                         │
│  [y] Yes  [n] No  [D] Force delete      │
└─────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn create_branch(&mut self, name: &str, base: Option<&str>) -> Result<(), Error> {
        self.repo.create_branch(name, base)?;
        self.refresh_branches();
        self.show_message(&format!("Created branch '{}'", name));
        Ok(())
    }

    pub fn delete_branch(&mut self, name: &str, force: bool) -> Result<(), Error> {
        // Check if branch is merged
        if !force && !self.repo.is_branch_merged(name)? {
            let unmerged = self.repo.unmerged_commits(name)?;
            self.show_dialog(Dialog::ConfirmDelete {
                branch: name.to_string(),
                unmerged_count: unmerged.len(),
            });
            return Ok(());
        }

        self.repo.delete_branch(name, force)?;
        self.refresh_branches();
        self.show_message(&format!("Deleted branch '{}'", name));
        Ok(())
    }
}
```

---

## 7. Staging Operations

### Description

Stage and unstage files for commit.

### UI

```
┌─────────────────────┐
│ [1] Status          │
│ ─────────────────── │
│ Staged (2)          │
│ > M  src/app.rs     │  ← Selected
│   A  src/new.rs     │
│ Unstaged (1)        │
│   M  README.md      │
│ Untracked (1)       │
│   ?  notes.txt      │
└─────────────────────┘
```

### Actions

| Key               | Action                             |
| ----------------- | ---------------------------------- |
| `Enter` / `Space` | Toggle stage/unstage selected file |
| `a`               | Stage all files                    |
| `A`               | Unstage all files                  |
| `d`               | Discard changes in selected file   |
| `D`               | Discard all unstaged changes       |
| `i`               | Add selected file to .gitignore    |

### Hunk Staging (in Diff panel)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ @@ -10,7 +10,9 @@ fn main() {                                [Stage Hunk]│
│      let config = Config::load();                                       │
│ -    app.run();                                                         │
│ +    if let Err(e) = app.run() {                                        │
│ +        eprintln!("Error: {}", e);                                     │
│ +    }                                                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn stage_selected(&mut self) -> Result<(), Error> {
        let item = self.status_view.selected_item();

        match item.section {
            Section::Staged => {
                self.repo.unstage_file(&item.path)?;
            }
            Section::Unstaged | Section::Untracked => {
                self.repo.stage_file(&item.path)?;
            }
        }

        self.refresh_status();
        Ok(())
    }

    pub fn stage_hunk(&mut self, hunk_index: usize) -> Result<(), Error> {
        let file = self.diff_view.current_file();
        let hunk = &file.hunks[hunk_index];

        self.repo.stage_hunk(&file.path, hunk)?;
        self.refresh_status();
        self.refresh_diff();
        Ok(())
    }
}
```

---

## 8. Commit Creation

### Description

Create commits with a message.

### Commit Dialog

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Commit Message                                                         │
│  ─────────────────────────────────────────────────────────────────────  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ Fix authentication bug when user has expired session              │  │
│  │                                                                   │  │
│  │ - Handle expired JWT tokens gracefully                            │  │
│  │ - Add proper error message for users                              │  │
│  │ - Update tests                                                    │  │
│  │█                                                                  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  Staged: 3 files changed, +45 -12                                       │
│                                                                         │
│  [Ctrl+Enter] commit  [Esc] cancel                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

### Empty Commit Prevention

```
┌─────────────────────────────────────────┐
│  No changes staged for commit           │
│                                         │
│  Stage some changes first.              │
│                                         │
│  [Enter] OK                             │
└─────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn open_commit_dialog(&mut self) -> Result<(), Error> {
        let staged_count = self.status_view.staged_count();

        if staged_count == 0 {
            self.show_message("No changes staged for commit");
            return Ok(());
        }

        self.mode = AppMode::Dialog(Dialog::Commit {
            message: String::new(),
            cursor: 0,
        });

        Ok(())
    }

    pub fn commit(&mut self, message: &str) -> Result<(), Error> {
        if message.trim().is_empty() {
            self.show_message("Commit message cannot be empty");
            return Ok(());
        }

        let oid = self.repo.commit(message)?;
        self.close_dialog();
        self.refresh_all();
        self.show_message(&format!("Created commit {}", &oid[..7]));
        Ok(())
    }
}
```

---

## 9. Diff Display

### Description

Show file differences with syntax highlighting.

### UI

```
┌─────────────────────────────────────────────────────────────────────────┐
│ [4] Diff: src/app.rs (staged)                                           │
│ ─────────────────────────────────────────────────────────────────────── │
│  8   │     let config = Config::load();                                 │
│  9   │     let app = App::new(config);                                  │
│  10  │ -   app.run();                                    [red]          │
│  11  │ +   if let Err(e) = app.run() {                   [green]        │
│  12  │ +       eprintln!("Error: {}", e);                [green]        │
│  13  │ +   }                                             [green]        │
│  14  │ }                                                                │
└─────────────────────────────────────────────────────────────────────────┘
```

### Color Scheme

| Line Type    | Color                          |
| ------------ | ------------------------------ |
| Addition     | Green background or green text |
| Deletion     | Red background or red text     |
| Hunk header  | Cyan                           |
| Line numbers | Dim gray                       |
| Context      | Normal                         |

### Word-Level Diff (optional)

```
│  10  │ -   app.run();                                                   │
│  11  │ +   if let Err(e) = app.run() {                                  │
                            ^^^^^^^^^^ word-level highlight
```

### Implementation

```rust
impl DiffView {
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        for (i, line) in self.visible_lines().enumerate() {
            let y = area.y + i as u16;

            // Line number
            if self.show_line_numbers {
                let line_no = format!("{:4} │ ", line.number.unwrap_or(0));
                buf.set_string(area.x, y, &line_no, Style::default().dim());
            }

            // Line content with style
            let style = match line.line_type {
                LineType::Addition => Style::default().fg(theme.diff_add),
                LineType::Deletion => Style::default().fg(theme.diff_remove),
                LineType::HunkHeader => Style::default().fg(theme.diff_hunk),
                LineType::Context => Style::default(),
            };

            let prefix = match line.line_type {
                LineType::Addition => "+ ",
                LineType::Deletion => "- ",
                _ => "  ",
            };

            buf.set_string(area.x + 7, y, prefix, style);
            buf.set_string(area.x + 9, y, &line.content, style);
        }
    }
}
```

---

## 10. Stash Operations

### Description

Save, apply, pop, and manage stashes.

### UI

```
┌─────────────────────┐
│ Stash (3)           │
│ ─────────────────── │
│ stash@{0}: WIP auth │
│ stash@{1}: temp fix │
│ stash@{2}: backup   │
└─────────────────────┘
```

### Stash Dialog

```
┌─────────────────────────────────────────┐
│  Save stash                             │
│  ┌─────────────────────────────────────┐│
│  │ Work in progress on authentication ││
│  └─────────────────────────────────────┘│
│                                         │
│  [ ] Include untracked files            │
│  [ ] Keep staged changes                │
│                                         │
│  [Enter] save  [Esc] cancel             │
└─────────────────────────────────────────┘
```

### Actions

| Key     | Action                       |
| ------- | ---------------------------- |
| `Enter` | Apply stash (keep in list)   |
| `p`     | Pop stash (apply and remove) |
| `d`     | Drop stash                   |
| `n`     | New stash                    |
| `N`     | New stash with message       |
| `b`     | Create branch from stash     |

### Implementation

```rust
impl App {
    pub fn stash_save(&mut self, message: Option<&str>, include_untracked: bool) -> Result<(), Error> {
        let mut flags = git2::StashFlags::DEFAULT;
        if include_untracked {
            flags |= git2::StashFlags::INCLUDE_UNTRACKED;
        }

        self.repo.stash_save(message)?;
        self.refresh_status();
        self.refresh_stash();
        self.show_message("Changes stashed");
        Ok(())
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<(), Error> {
        self.repo.stash_pop(index)?;
        self.refresh_status();
        self.refresh_stash();
        self.show_message("Stash popped");
        Ok(())
    }
}
```

---

## 11. Merge

### Description

Merge branches into current branch.

### Flow

1. Select branch to merge in Branch panel
2. Press `m`
3. Show merge preview dialog
4. Confirm merge

### Merge Preview

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Merge 'feature/auth' into 'main'                                       │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Commits to merge: 5                                                    │
│    a1b2c3d Add login page                                               │
│    e4f5g6h Add authentication service                                   │
│    i7j8k9l Add JWT handling                                             │
│    ...                                                                  │
│                                                                         │
│  [Enter] merge  [Esc] cancel                                            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Conflict Resolution

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Merge conflict in 2 files                                              │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Conflicting files:                                                     │
│    C  src/auth.rs                                                       │
│    C  src/config.rs                                                     │
│                                                                         │
│  [e] Edit file  [o] Accept ours  [t] Accept theirs  [a] Abort merge     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn merge_branch(&mut self, branch: &str) -> Result<(), Error> {
        match self.repo.merge(branch)? {
            MergeResult::UpToDate => {
                self.show_message("Already up to date");
            }
            MergeResult::FastForward => {
                self.show_message(&format!("Fast-forward merged '{}'", branch));
            }
            MergeResult::Merged => {
                self.show_message(&format!("Merged '{}' successfully", branch));
            }
            MergeResult::Conflict => {
                let conflicts = self.repo.get_conflicts()?;
                self.mode = AppMode::MergeConflict(conflicts);
            }
        }
        self.refresh_all();
        Ok(())
    }
}
```

---

## 12. Rebase

### Description

Rebase current branch onto another branch.

### Flow

1. Select target branch
2. Press `R`
3. Show rebase preview
4. Confirm and execute

### Rebase Preview

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Rebase 'feature/auth' onto 'main'                                      │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Commits to replay: 3                                                   │
│    a1b2c3d Add login page                                               │
│    e4f5g6h Add auth service                                             │
│    i7j8k9l Add tests                                                    │
│                                                                         │
│  ⚠ This will rewrite history. Continue?                                 │
│                                                                         │
│  [Enter] rebase  [Esc] cancel                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### Rebase Conflict

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Rebase conflict                                                        │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Applying: e4f5g6h Add auth service                                     │
│                                                                         │
│  Conflict in: src/auth.rs                                               │
│                                                                         │
│  [e] Edit  [c] Continue  [s] Skip  [a] Abort                            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn rebase_onto(&mut self, upstream: &str) -> Result<(), Error> {
        match self.repo.rebase(upstream)? {
            RebaseResult::Success => {
                self.show_message("Rebase completed successfully");
            }
            RebaseResult::Conflict(commits) => {
                self.mode = AppMode::RebaseConflict {
                    remaining: commits,
                    current: 0,
                };
            }
        }
        self.refresh_all();
        Ok(())
    }

    pub fn rebase_continue(&mut self) -> Result<(), Error> {
        self.repo.rebase_continue()?;
        self.refresh_all();
        Ok(())
    }

    pub fn rebase_abort(&mut self) -> Result<(), Error> {
        self.repo.rebase_abort()?;
        self.mode = AppMode::Normal;
        self.refresh_all();
        Ok(())
    }
}
```

---

## 13. Push/Pull

### Description

Push to and pull from remote repositories.

### Push Dialog

```
┌─────────────────────────────────────────┐
│  Push to remote                         │
│  ─────────────────────────────────────  │
│                                         │
│  Remote: origin                         │
│  Branch: main → origin/main             │
│                                         │
│  Commits to push: 3                     │
│                                         │
│  [ ] Force push                         │
│  [ ] Set upstream                       │
│                                         │
│  [Enter] push  [Esc] cancel             │
└─────────────────────────────────────────┘
```

### Pull Dialog

```
┌─────────────────────────────────────────┐
│  Pull from remote                       │
│  ─────────────────────────────────────  │
│                                         │
│  Remote: origin                         │
│  Branch: origin/main → main             │
│                                         │
│  New commits: 5                         │
│                                         │
│  [Enter] pull  [r] Rebase  [Esc] cancel │
└─────────────────────────────────────────┘
```

### Progress Indicator

```
┌─────────────────────────────────────────┐
│  Pushing to origin...                   │
│  ─────────────────────────────────────  │
│                                         │
│  ████████████░░░░░░░░  60%              │
│  Compressing objects: 3/5               │
│                                         │
└─────────────────────────────────────────┘
```

### Implementation

```rust
impl App {
    pub fn push(&mut self, force: bool) -> Result<(), Error> {
        let remote = self.config.git.default_remote.clone();
        let branch = self.repo.current_branch()?.unwrap();

        let callbacks = RemoteCallbacks {
            credentials: Some(Box::new(|| self.get_credentials())),
            progress: Some(Box::new(|received, total| {
                self.update_progress(received, total);
            })),
        };

        self.show_progress("Pushing...");

        if force {
            self.repo.push_force(&remote, &branch, callbacks)?;
        } else {
            self.repo.push(&remote, &branch, callbacks)?;
        }

        self.hide_progress();
        self.refresh_all();
        self.show_message("Push completed");
        Ok(())
    }

    pub fn pull(&mut self, rebase: bool) -> Result<(), Error> {
        let remote = self.config.git.default_remote.clone();
        let branch = self.repo.current_branch()?.unwrap();

        let callbacks = RemoteCallbacks {
            credentials: Some(Box::new(|| self.get_credentials())),
            progress: Some(Box::new(|received, total| {
                self.update_progress(received, total);
            })),
        };

        self.show_progress("Pulling...");

        if rebase {
            self.repo.pull_rebase(&remote, &branch, callbacks)?;
        } else {
            self.repo.pull(&remote, &branch, callbacks)?;
        }

        self.hide_progress();
        self.refresh_all();
        self.show_message("Pull completed");
        Ok(())
    }
}
```

---

## 14. Tag Management

### Description

List, create, and delete Git tags.

### UI

```
┌─────────────────────┐
│ Tags (5)            │
│ ─────────────────── │
│ v2.0.0              │
│ v1.1.0              │
│ v1.0.0              │
│ v0.2.0              │
│ v0.1.0              │
└─────────────────────┘
```

### Create Tag Dialog

```
┌─────────────────────────────────────────┐
│  Create tag                             │
│  ┌─────────────────────────────────────┐│
│  │ v2.1.0█                             ││
│  └─────────────────────────────────────┘│
│                                         │
│  Target: a1b2c3d (HEAD)                 │
│                                         │
│  [ ] Annotated tag                      │
│  ┌─────────────────────────────────────┐│
│  │ Release version 2.1.0               ││
│  └─────────────────────────────────────┘│
│                                         │
│  [Enter] create  [Esc] cancel           │
└─────────────────────────────────────────┘
```

### Actions

| Key     | Action                       |
| ------- | ---------------------------- |
| `n`     | Create lightweight tag       |
| `N`     | Create annotated tag         |
| `d`     | Delete tag                   |
| `p`     | Push tag to remote           |
| `P`     | Push all tags                |
| `Enter` | Checkout tag (detached HEAD) |

### Implementation

```rust
impl App {
    pub fn create_tag(&mut self, name: &str, message: Option<&str>) -> Result<(), Error> {
        self.repo.create_tag(name, message)?;
        self.refresh_tags();
        self.show_message(&format!("Created tag '{}'", name));
        Ok(())
    }

    pub fn delete_tag(&mut self, name: &str) -> Result<(), Error> {
        self.repo.delete_tag(name)?;
        self.refresh_tags();
        self.show_message(&format!("Deleted tag '{}'", name));
        Ok(())
    }

    pub fn push_tag(&mut self, name: &str) -> Result<(), Error> {
        let remote = self.config.git.default_remote.clone();
        self.repo.push_tag(&remote, name)?;
        self.show_message(&format!("Pushed tag '{}' to {}", name, remote));
        Ok(())
    }
}
```
