# Keybindings Design

## Overview

Dual input support: Vim-style keys AND arrow keys. All keybindings are customizable via config file.

## Global Keybindings

These work in all panels and modes.

| Key         | Vim Key     | Action                    |
| ----------- | ----------- | ------------------------- |
| `↑`         | `k`         | Move up                   |
| `↓`         | `j`         | Move down                 |
| `←`         | `h`         | Previous panel / collapse |
| `→`         | `l`         | Next panel / expand       |
| `Home`      | `g`         | Go to first item          |
| `End`       | `G`         | Go to last item           |
| `PageUp`    | `Ctrl+u`    | Page up                   |
| `PageDown`  | `Ctrl+d`    | Page down                 |
| `1-4`       | `1-4`       | Jump to panel             |
| `Tab`       | `Tab`       | Next panel                |
| `Shift+Tab` | `Shift+Tab` | Previous panel            |
| `?`         | `?`         | Toggle help               |
| `q`         | `q`         | Quit                      |
| `Esc`       | `Esc`       | Cancel / close dialog     |
| `/`         | `/`         | Search                    |
| `:`         | `:`         | Command mode              |

## Panel-Specific Keybindings

### Status Panel

| Key               | Action                                  |
| ----------------- | --------------------------------------- |
| `Enter` / `Space` | Stage/unstage selected file             |
| `a`               | Stage all files                         |
| `A`               | Unstage all files                       |
| `d`               | Discard changes (with confirmation)     |
| `D`               | Discard all changes (with confirmation) |
| `c`               | Open commit dialog                      |
| `e`               | Edit file in $EDITOR                    |
| `i`               | Add to .gitignore                       |

### Branch Panel

| Key     | Action                            |
| ------- | --------------------------------- |
| `Enter` | Checkout selected branch          |
| `n`     | Create new branch                 |
| `d`     | Delete branch (with confirmation) |
| `D`     | Force delete branch               |
| `r`     | Rename branch                     |
| `m`     | Merge into current branch         |
| `R`     | Rebase current onto selected      |
| `f`     | Fetch remote branch               |
| `t`     | Toggle show/hide remote branches  |

### Commit Panel

| Key     | Action                            |
| ------- | --------------------------------- |
| `Enter` | Show commit details in main panel |
| `/`     | Search commits                    |
| `n`     | Next search result                |
| `N`     | Previous search result            |
| `c`     | Cherry-pick commit                |
| `r`     | Revert commit                     |
| `R`     | Interactive rebase from here      |
| `y`     | Copy commit hash                  |

### Main Panel (Diff View)

| Key               | Action                 |
| ----------------- | ---------------------- |
| `Enter` / `Space` | Stage/unstage hunk     |
| `s`               | Stage hunk             |
| `u`               | Unstage hunk           |
| `[`               | Previous hunk          |
| `]`               | Next hunk              |
| `{`               | Previous file          |
| `}`               | Next file              |
| `e`               | Edit hunk manually     |
| `+` / `=`         | Increase context lines |
| `-`               | Decrease context lines |

### Stash Panel

| Key     | Action                         |
| ------- | ------------------------------ |
| `Enter` | Apply stash                    |
| `p`     | Pop stash                      |
| `d`     | Drop stash (with confirmation) |
| `n`     | New stash                      |
| `N`     | New stash with message         |
| `b`     | Create branch from stash       |

### Tag Panel

| Key     | Action                         |
| ------- | ------------------------------ |
| `Enter` | Checkout tag                   |
| `n`     | Create new tag                 |
| `N`     | Create annotated tag           |
| `d`     | Delete tag (with confirmation) |
| `p`     | Push tag to remote             |
| `P`     | Push all tags                  |

## Dialog Keybindings

### Confirmation Dialog

| Key                 | Action  |
| ------------------- | ------- |
| `y` / `Y` / `Enter` | Confirm |
| `n` / `N` / `Esc`   | Cancel  |

### Input Dialog

| Key         | Action              |
| ----------- | ------------------- |
| `Enter`     | Submit              |
| `Esc`       | Cancel              |
| `←` / `→`   | Move cursor         |
| `Ctrl+a`    | Beginning of line   |
| `Ctrl+e`    | End of line         |
| `Ctrl+k`    | Delete to end       |
| `Ctrl+u`    | Delete to beginning |
| `Backspace` | Delete character    |
| `Ctrl+w`    | Delete word         |

### Search Mode

| Key            | Action          |
| -------------- | --------------- |
| `Enter`        | Execute search  |
| `Esc`          | Cancel search   |
| `Ctrl+n` / `↓` | Next result     |
| `Ctrl+p` / `↑` | Previous result |

## Command Mode

Enter with `:`. Supports commands like:

| Command            | Action                   |
| ------------------ | ------------------------ |
| `:q`               | Quit                     |
| `:w`               | Write (commit if staged) |
| `:branch <name>`   | Create branch            |
| `:checkout <name>` | Checkout branch          |
| `:merge <name>`    | Merge branch             |
| `:stash`           | Create stash             |
| `:stash pop`       | Pop stash                |
| `:tag <name>`      | Create tag               |
| `:push`            | Push to remote           |
| `:pull`            | Pull from remote         |
| `:fetch`           | Fetch from remote        |
| `:help`            | Show help                |

## Keybinding Data Structure

```rust
pub struct KeyBindings {
    pub global: HashMap<KeyEvent, Action>,
    pub status: HashMap<KeyEvent, Action>,
    pub branches: HashMap<KeyEvent, Action>,
    pub commits: HashMap<KeyEvent, Action>,
    pub diff: HashMap<KeyEvent, Action>,
    pub stash: HashMap<KeyEvent, Action>,
    pub tags: HashMap<KeyEvent, Action>,
    pub dialog: HashMap<KeyEvent, Action>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = Self::new();

        // Global
        bindings.global.insert(key!('q'), Action::Quit);
        bindings.global.insert(key!('?'), Action::ToggleHelp);
        bindings.global.insert(key!(Esc), Action::Cancel);
        bindings.global.insert(key!('/'), Action::Search);

        // Navigation - both Vim and Arrow keys
        bindings.global.insert(key!('j'), Action::MoveDown);
        bindings.global.insert(key!(Down), Action::MoveDown);
        bindings.global.insert(key!('k'), Action::MoveUp);
        bindings.global.insert(key!(Up), Action::MoveUp);
        bindings.global.insert(key!('h'), Action::MovePrevious);
        bindings.global.insert(key!(Left), Action::MovePrevious);
        bindings.global.insert(key!('l'), Action::MoveNext);
        bindings.global.insert(key!(Right), Action::MoveNext);

        // Panel switching
        bindings.global.insert(key!('1'), Action::FocusPanel(0));
        bindings.global.insert(key!('2'), Action::FocusPanel(1));
        bindings.global.insert(key!('3'), Action::FocusPanel(2));
        bindings.global.insert(key!('4'), Action::FocusPanel(3));
        bindings.global.insert(key!(Tab), Action::NextPanel);
        bindings.global.insert(key!(BackTab), Action::PreviousPanel);

        // ... more bindings
        bindings
    }
}
```

## Action Enum

```rust
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveNext,
    MovePrevious,
    GoToTop,
    GoToBottom,
    PageUp,
    PageDown,

    // Panel
    FocusPanel(usize),
    NextPanel,
    PreviousPanel,

    // Global
    Quit,
    ToggleHelp,
    Cancel,
    Search,
    Command,

    // Status actions
    StageFile,
    UnstageFile,
    StageAll,
    UnstageAll,
    DiscardChanges,
    DiscardAllChanges,

    // Branch actions
    CheckoutBranch,
    CreateBranch,
    DeleteBranch,
    RenameBranch,
    MergeBranch,
    RebaseBranch,

    // Commit actions
    Commit,
    ShowCommitDetails,
    CherryPick,
    RevertCommit,
    CopyCommitHash,

    // Diff actions
    StageHunk,
    UnstageHunk,
    NextHunk,
    PreviousHunk,
    NextFile,
    PreviousFile,
    IncreaseContext,
    DecreaseContext,

    // Stash actions
    ApplyStash,
    PopStash,
    DropStash,
    NewStash,

    // Tag actions
    CreateTag,
    DeleteTag,
    PushTag,

    // Remote actions
    Push,
    Pull,
    Fetch,

    // Custom
    Custom(String),
}
```

## Key Macro Helper

```rust
macro_rules! key {
    ($char:literal) => {
        KeyEvent {
            code: KeyCode::Char($char),
            modifiers: Modifiers::empty(),
        }
    };
    (Ctrl + $char:literal) => {
        KeyEvent {
            code: KeyCode::Char($char),
            modifiers: Modifiers::CTRL,
        }
    };
    (Alt + $char:literal) => {
        KeyEvent {
            code: KeyCode::Char($char),
            modifiers: Modifiers::ALT,
        }
    };
    ($code:ident) => {
        KeyEvent {
            code: KeyCode::$code,
            modifiers: Modifiers::empty(),
        }
    };
}
```

## Input Handler

```rust
pub struct InputHandler {
    bindings: KeyBindings,
    mode: InputMode,
}

pub enum InputMode {
    Normal,
    Search(SearchState),
    Command(CommandState),
    Dialog(DialogState),
}

impl InputHandler {
    pub fn handle(&mut self, event: Event, panel: PanelType) -> Option<Action> {
        match event {
            Event::Key(key) => self.handle_key(key, panel),
            Event::Resize(w, h) => Some(Action::Resize(w, h)),
            _ => None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent, panel: PanelType) -> Option<Action> {
        match self.mode {
            InputMode::Normal => {
                // Check panel-specific bindings first
                let panel_bindings = match panel {
                    PanelType::Status => &self.bindings.status,
                    PanelType::Branches => &self.bindings.branches,
                    PanelType::Commits => &self.bindings.commits,
                    PanelType::Main => &self.bindings.diff,
                    PanelType::Stash => &self.bindings.stash,
                    PanelType::Tags => &self.bindings.tags,
                };

                panel_bindings.get(&key)
                    .or_else(|| self.bindings.global.get(&key))
                    .cloned()
            }
            InputMode::Search(ref mut state) => {
                self.handle_search_key(key, state)
            }
            InputMode::Command(ref mut state) => {
                self.handle_command_key(key, state)
            }
            InputMode::Dialog(ref mut state) => {
                self.handle_dialog_key(key, state)
            }
        }
    }
}
```

## Configuration Format

```toml
# ~/.config/g/config.toml

[keybindings.global]
quit = ["q", "Ctrl+c"]
help = ["?", "F1"]
search = ["/", "Ctrl+f"]

[keybindings.status]
stage = ["Enter", "Space", "s"]
stage_all = ["a"]
commit = ["c"]

[keybindings.branches]
checkout = ["Enter"]
new = ["n"]
delete = ["d"]

[keybindings.commits]
details = ["Enter"]
cherry_pick = ["c"]
revert = ["r"]

[keybindings.diff]
stage_hunk = ["Enter", "s"]
next_hunk = ["]"]
prev_hunk = ["["]
```

## Keybinding Parser

```rust
impl KeyBindings {
    pub fn from_config(config: &Config) -> Self {
        let mut bindings = Self::default();

        if let Some(kb_config) = &config.keybindings {
            for (action_name, keys) in &kb_config.global {
                if let Some(action) = Action::from_name(action_name) {
                    for key_str in keys {
                        if let Some(key) = parse_key(key_str) {
                            bindings.global.insert(key, action.clone());
                        }
                    }
                }
            }
            // ... parse other sections
        }

        bindings
    }
}

fn parse_key(s: &str) -> Option<KeyEvent> {
    let parts: Vec<&str> = s.split('+').collect();

    let mut modifiers = Modifiers::empty();
    let code_str = parts.last()?;

    for part in &parts[..parts.len()-1] {
        match part.to_lowercase().as_str() {
            "ctrl" => modifiers |= Modifiers::CTRL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            _ => return None,
        }
    }

    let code = match code_str.to_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Escape,
        "tab" => KeyCode::Tab,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        s if s.len() == 1 => KeyCode::Char(s.chars().next()?),
        s if s.starts_with('f') => {
            let n: u8 = s[1..].parse().ok()?;
            KeyCode::F(n)
        }
        _ => return None,
    };

    Some(KeyEvent { code, modifiers })
}
```
