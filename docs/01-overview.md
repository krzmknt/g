# g - Git TUI Overview

## Project Summary

`g` is a blazingly fast Git TUI (Text User Interface) application written in Rust. It provides a rich, interactive interface for Git operations with minimal external dependencies.

## Goals

1. **Blazingly Fast** - Instant startup, responsive UI, efficient Git operations
2. **Rich TUI** - Multi-pane layout with syntax highlighting and visual feedback
3. **Minimal Dependencies** - Only Rust std and rust-lang official crates (git2)
4. **Cross-Platform** - Full support for macOS, Linux, and Windows

## Technology Stack

| Component      | Technology                                       |
| -------------- | ------------------------------------------------ |
| Language       | Rust                                             |
| Git Operations | git2 crate (rust-lang official libgit2 bindings) |
| TUI Rendering  | Custom (ANSI escape sequences)                   |
| Configuration  | TOML format                                      |

## Design Principles

### 1. Zero External Dependencies (except rust-lang official)

- No third-party TUI frameworks
- No third-party utility crates
- Only `std` and `git2` (rust-lang maintained)

### 2. Performance First

- Lazy loading of Git data
- Efficient rendering (only redraw changed regions)
- Asynchronous operations where beneficial

### 3. User Experience

- Vim-style AND arrow key navigation
- Intuitive multi-pane layout (lazygit-inspired)
- Dark/Light theme support
- Customizable via config file

## Target Platforms

| Platform | Terminal Support                                |
| -------- | ----------------------------------------------- |
| macOS    | Terminal.app, iTerm2, Alacritty, etc.           |
| Linux    | GNOME Terminal, Konsole, Alacritty, xterm, etc. |
| Windows  | Windows Terminal, ConEmu, cmd.exe (Windows 10+) |

## Feature Set

### View Operations

- Branch list (local/remote)
- Current branch display
- Commit history with search
- Diff view with syntax highlighting

### Git Operations

- Branch: create, delete, switch
- Staging: add, unstage (file/hunk level)
- Commit: create with message
- Stash: save, pop, list, drop
- Merge: merge branches
- Rebase: rebase operations
- Remote: push, pull
- Tags: list, create, delete

## Configuration

Location: `~/.config/g/config.toml`

Configurable items:

- Color theme (dark/light)
- Custom keybindings
- Default behaviors
