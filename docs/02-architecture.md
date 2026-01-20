# System Architecture

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        g (main)                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Config    │  │    Input    │  │     TUI Engine      │  │
│  │   Module    │  │   Handler   │  │  (ANSI Renderer)    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│         │                │                    │             │
│         └────────────────┼────────────────────┘             │
│                          │                                  │
│                          ▼                                  │
│              ┌─────────────────────┐                        │
│              │    App State        │                        │
│              │  (Central State)    │                        │
│              └─────────────────────┘                        │
│                          │                                  │
│                          ▼                                  │
│              ┌─────────────────────┐                        │
│              │    Git Module       │                        │
│              │    (git2 crate)     │                        │
│              └─────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs                 # Entry point
├── app.rs                  # Application state and main loop
├── config/
│   ├── mod.rs              # Config module
│   ├── parser.rs           # TOML parser (custom)
│   └── theme.rs            # Color theme definitions
├── tui/
│   ├── mod.rs              # TUI module
│   ├── terminal.rs         # Terminal control (raw mode, size)
│   ├── buffer.rs           # Screen buffer
│   ├── render.rs           # ANSI rendering
│   ├── style.rs            # Colors, attributes
│   └── widgets/
│       ├── mod.rs          # Widget traits
│       ├── list.rs         # Scrollable list
│       ├── text.rs         # Text display
│       ├── input.rs        # Text input
│       └── pane.rs         # Pane container
├── input/
│   ├── mod.rs              # Input module
│   ├── event.rs            # Input events
│   └── keys.rs             # Key parsing
├── git/
│   ├── mod.rs              # Git module
│   ├── repository.rs       # Repository wrapper
│   ├── branch.rs           # Branch operations
│   ├── commit.rs           # Commit operations
│   ├── diff.rs             # Diff operations
│   ├── stash.rs            # Stash operations
│   ├── remote.rs           # Remote operations
│   └── tag.rs              # Tag operations
└── views/
    ├── mod.rs              # View module
    ├── branches.rs         # Branch list view
    ├── commits.rs          # Commit log view
    ├── status.rs           # Status/staging view
    ├── diff.rs             # Diff view
    └── help.rs             # Help overlay
```

## Core Components

### 1. App State (`app.rs`)

Central state management using a single struct that holds:

- Current view/mode
- Git repository state
- UI state (selected items, scroll positions)
- User input buffer

```rust
pub struct App {
    pub repo: GitRepository,
    pub mode: AppMode,
    pub views: Views,
    pub config: Config,
    pub should_quit: bool,
}

pub enum AppMode {
    Normal,
    Command,
    Search,
    Confirm(ConfirmAction),
}
```

### 2. TUI Engine (`tui/`)

Custom TUI implementation using ANSI escape sequences:

- **Terminal**: Raw mode, alternate screen, cursor control
- **Buffer**: Double-buffered rendering for flicker-free updates
- **Render**: ANSI escape sequence generation
- **Widgets**: Reusable UI components

### 3. Input Handler (`input/`)

Platform-specific input handling:

- Unix: termios for raw mode, read from stdin
- Windows: Console API for raw input

### 4. Git Module (`git/`)

Wrapper around git2 crate providing:

- High-level Git operations
- Error handling
- Caching for performance

## Data Flow

```
User Input
    │
    ▼
┌─────────────┐
│   Input     │──────► Key Event
│   Handler   │
└─────────────┘
        │
        ▼
┌─────────────┐
│   App       │──────► State Update
│   State     │
└─────────────┘
        │
        ▼
┌─────────────┐
│   Views     │──────► Widget Tree
│             │
└─────────────┘
        │
        ▼
┌─────────────┐
│   TUI       │──────► ANSI Sequences
│   Engine    │
└─────────────┘
        │
        ▼
    Terminal
```

## Event Loop

```rust
loop {
    // 1. Render current state
    terminal.draw(|frame| {
        app.render(frame);
    })?;

    // 2. Wait for input (with timeout for async updates)
    if let Some(event) = input.poll(Duration::from_millis(100))? {
        // 3. Handle input
        match app.handle_event(event) {
            Action::Quit => break,
            Action::Update => continue,
            Action::None => {}
        }
    }

    // 4. Check for async updates (e.g., push/pull progress)
    app.check_async_updates()?;
}
```

## Platform Abstraction

```rust
// Terminal abstraction for cross-platform support
pub trait TerminalBackend {
    fn enable_raw_mode(&mut self) -> Result<()>;
    fn disable_raw_mode(&mut self) -> Result<()>;
    fn enter_alternate_screen(&mut self) -> Result<()>;
    fn leave_alternate_screen(&mut self) -> Result<()>;
    fn size(&self) -> Result<(u16, u16)>;
    fn write(&mut self, buf: &[u8]) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
    fn read_event(&mut self, timeout: Duration) -> Result<Option<Event>>;
}

// Platform-specific implementations
#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
```

## Error Handling Strategy

```rust
// Custom error type for the application
pub enum Error {
    Git(git2::Error),
    Io(std::io::Error),
    Config(ConfigError),
    Terminal(TerminalError),
}

// Result type alias
pub type Result<T> = std::result::Result<T, Error>;
```
