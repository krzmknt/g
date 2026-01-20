# TUI Framework Design

## Overview

Custom TUI framework built on ANSI escape sequences. No external dependencies.

## ANSI Escape Sequences

### Cursor Control

| Sequence            | Description               |
| ------------------- | ------------------------- |
| `\x1b[H`            | Move cursor to home (0,0) |
| `\x1b[{row};{col}H` | Move cursor to position   |
| `\x1b[?25l`         | Hide cursor               |
| `\x1b[?25h`         | Show cursor               |
| `\x1b[s`            | Save cursor position      |
| `\x1b[u`            | Restore cursor position   |

### Screen Control

| Sequence      | Description                      |
| ------------- | -------------------------------- |
| `\x1b[2J`     | Clear entire screen              |
| `\x1b[K`      | Clear from cursor to end of line |
| `\x1b[?1049h` | Enter alternate screen buffer    |
| `\x1b[?1049l` | Leave alternate screen buffer    |

### Text Styling

| Sequence  | Description             |
| --------- | ----------------------- |
| `\x1b[0m` | Reset all attributes    |
| `\x1b[1m` | Bold                    |
| `\x1b[2m` | Dim                     |
| `\x1b[3m` | Italic                  |
| `\x1b[4m` | Underline               |
| `\x1b[7m` | Reverse (invert colors) |

### Colors (256-color mode)

| Sequence         | Description                  |
| ---------------- | ---------------------------- |
| `\x1b[38;5;{n}m` | Set foreground color (0-255) |
| `\x1b[48;5;{n}m` | Set background color (0-255) |

### True Color (24-bit)

| Sequence                 | Description        |
| ------------------------ | ------------------ |
| `\x1b[38;2;{r};{g};{b}m` | Set foreground RGB |
| `\x1b[48;2;{r};{g};{b}m` | Set background RGB |

## Terminal Module

### Raw Mode

```rust
// Unix implementation
#[cfg(unix)]
pub fn enable_raw_mode() -> Result<Termios> {
    use std::os::unix::io::AsRawFd;
    let fd = std::io::stdin().as_raw_fd();
    let original = termios::tcgetattr(fd)?;

    let mut raw = original.clone();
    // Disable canonical mode, echo, signals
    raw.c_lflag &= !(ICANON | ECHO | ISIG | IEXTEN);
    // Disable input processing
    raw.c_iflag &= !(IXON | ICRNL | BRKINT | INPCK | ISTRIP);
    // Disable output processing
    raw.c_oflag &= !(OPOST);
    // Set character size to 8 bits
    raw.c_cflag |= CS8;
    // Read returns immediately with available bytes
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 0;

    termios::tcsetattr(fd, TCSAFLUSH, &raw)?;
    Ok(original)
}

// Windows implementation
#[cfg(windows)]
pub fn enable_raw_mode() -> Result<ConsoleMode> {
    use windows_sys::Win32::System::Console::*;

    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut mode = 0;
    unsafe { GetConsoleMode(handle, &mut mode) };
    let original = mode;

    // Disable line input and echo
    mode &= !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
    // Enable virtual terminal processing
    mode |= ENABLE_VIRTUAL_TERMINAL_INPUT;

    unsafe { SetConsoleMode(handle, mode) };

    // Also enable VT processing for output
    let out_handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    let mut out_mode = 0;
    unsafe { GetConsoleMode(out_handle, &mut out_mode) };
    out_mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    unsafe { SetConsoleMode(out_handle, out_mode) };

    Ok(original)
}
```

### Terminal Size

```rust
#[cfg(unix)]
pub fn terminal_size() -> Result<(u16, u16)> {
    use std::os::unix::io::AsRawFd;
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    let fd = std::io::stdout().as_raw_fd();

    if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut size) } == 0 {
        Ok((size.ws_col, size.ws_row))
    } else {
        Err(Error::Terminal("Failed to get terminal size"))
    }
}

#[cfg(windows)]
pub fn terminal_size() -> Result<(u16, u16)> {
    use windows_sys::Win32::System::Console::*;

    let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };

    if unsafe { GetConsoleScreenBufferInfo(handle, &mut info) } != 0 {
        let width = (info.srWindow.Right - info.srWindow.Left + 1) as u16;
        let height = (info.srWindow.Bottom - info.srWindow.Top + 1) as u16;
        Ok((width, height))
    } else {
        Err(Error::Terminal("Failed to get terminal size"))
    }
}
```

## Buffer System

### Cell Structure

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub symbol: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            attrs: Attributes::empty(),
        }
    }
}
```

### Buffer Structure

```rust
pub struct Buffer {
    cells: Vec<Cell>,
    width: u16,
    height: u16,
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            cells: vec![Cell::default(); (width * height) as usize],
            width,
            height,
        }
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        &self.cells[(y * self.width + x) as usize]
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        &mut self.cells[(y * self.width + x) as usize]
    }

    pub fn set_string(&mut self, x: u16, y: u16, s: &str, style: Style) {
        let mut x = x;
        for c in s.chars() {
            if x >= self.width {
                break;
            }
            let cell = self.get_mut(x, y);
            cell.symbol = c;
            cell.fg = style.fg;
            cell.bg = style.bg;
            cell.attrs = style.attrs;
            x += 1;
        }
    }
}
```

### Double Buffering

```rust
pub struct DoubleBuffer {
    current: Buffer,
    previous: Buffer,
}

impl DoubleBuffer {
    pub fn diff(&self) -> Vec<(u16, u16, &Cell)> {
        let mut changes = Vec::new();
        for y in 0..self.current.height {
            for x in 0..self.current.width {
                let curr = self.current.get(x, y);
                let prev = self.previous.get(x, y);
                if curr != prev {
                    changes.push((x, y, curr));
                }
            }
        }
        changes
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.current, &mut self.previous);
        self.current.clear();
    }
}
```

## Rendering

### Renderer

```rust
pub struct Renderer {
    buffer: DoubleBuffer,
    output: BufWriter<Stdout>,
}

impl Renderer {
    pub fn render(&mut self) -> Result<()> {
        let changes = self.buffer.diff();

        if changes.is_empty() {
            return Ok(());
        }

        let mut last_style = Style::default();
        let mut last_pos = (u16::MAX, u16::MAX);

        for (x, y, cell) in changes {
            // Move cursor if not sequential
            if (x, y) != (last_pos.0 + 1, last_pos.1) {
                write!(self.output, "\x1b[{};{}H", y + 1, x + 1)?;
            }

            // Apply style changes
            let style = Style::from_cell(cell);
            if style != last_style {
                write!(self.output, "{}", style.to_ansi())?;
                last_style = style;
            }

            // Write character
            write!(self.output, "{}", cell.symbol)?;

            last_pos = (x, y);
        }

        self.output.flush()?;
        self.buffer.swap();
        Ok(())
    }
}
```

## Input Handling

### Event Types

```rust
pub enum Event {
    Key(KeyEvent),
    Resize(u16, u16),
    Mouse(MouseEvent),  // Optional: mouse support
}

pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    F(u8),
}

bitflags! {
    pub struct Modifiers: u8 {
        const SHIFT = 0b001;
        const CTRL  = 0b010;
        const ALT   = 0b100;
    }
}
```

### Input Parsing

```rust
pub fn parse_input(bytes: &[u8]) -> Option<(Event, usize)> {
    match bytes {
        // Single characters
        [b, ..] if *b < 0x1b => {
            let code = match b {
                0x0d => KeyCode::Enter,
                0x7f => KeyCode::Backspace,
                0x09 => KeyCode::Tab,
                _ => KeyCode::Char((*b + b'a' - 1) as char),  // Ctrl+letter
            };
            Some((Event::Key(KeyEvent { code, modifiers: Modifiers::CTRL }), 1))
        }

        // Escape sequences
        [0x1b, b'[', rest @ ..] => parse_csi_sequence(rest),

        // Alt + key
        [0x1b, b, ..] if *b != b'[' => {
            Some((Event::Key(KeyEvent {
                code: KeyCode::Char(*b as char),
                modifiers: Modifiers::ALT,
            }), 2))
        }

        // Plain escape
        [0x1b] => Some((Event::Key(KeyEvent {
            code: KeyCode::Escape,
            modifiers: Modifiers::empty(),
        }), 1)),

        // Regular character (UTF-8)
        _ => {
            let s = std::str::from_utf8(bytes).ok()?;
            let c = s.chars().next()?;
            let len = c.len_utf8();
            Some((Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: Modifiers::empty(),
            }), len))
        }
    }
}

fn parse_csi_sequence(bytes: &[u8]) -> Option<(Event, usize)> {
    match bytes {
        [b'A', ..] => Some((Event::Key(KeyEvent { code: KeyCode::Up, modifiers: Modifiers::empty() }), 3)),
        [b'B', ..] => Some((Event::Key(KeyEvent { code: KeyCode::Down, modifiers: Modifiers::empty() }), 3)),
        [b'C', ..] => Some((Event::Key(KeyEvent { code: KeyCode::Right, modifiers: Modifiers::empty() }), 3)),
        [b'D', ..] => Some((Event::Key(KeyEvent { code: KeyCode::Left, modifiers: Modifiers::empty() }), 3)),
        [b'H', ..] => Some((Event::Key(KeyEvent { code: KeyCode::Home, modifiers: Modifiers::empty() }), 3)),
        [b'F', ..] => Some((Event::Key(KeyEvent { code: KeyCode::End, modifiers: Modifiers::empty() }), 3)),
        // ... more sequences
        _ => None,
    }
}
```

## Widgets

### Widget Trait

```rust
pub trait Widget {
    fn render(&self, area: Rect, buf: &mut Buffer);
}

pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
```

### List Widget

```rust
pub struct List<'a> {
    items: Vec<ListItem<'a>>,
    selected: Option<usize>,
    offset: usize,
    style: Style,
    highlight_style: Style,
}

impl Widget for List<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        for (i, item) in self.items.iter()
            .skip(self.offset)
            .take(area.height as usize)
            .enumerate()
        {
            let y = area.y + i as u16;
            let style = if Some(self.offset + i) == self.selected {
                self.highlight_style
            } else {
                self.style
            };
            buf.set_string(area.x, y, &item.content, style);
        }
    }
}
```

### Block Widget (Borders)

```rust
pub struct Block<'a> {
    title: Option<&'a str>,
    borders: Borders,
    border_style: Style,
}

impl Widget for Block<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Border characters (Unicode box drawing)
        const TOP_LEFT: char = '┌';
        const TOP_RIGHT: char = '┐';
        const BOTTOM_LEFT: char = '└';
        const BOTTOM_RIGHT: char = '┘';
        const HORIZONTAL: char = '─';
        const VERTICAL: char = '│';

        // Draw borders...
    }
}
```

## Color System

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Indexed(u8),      // 256-color
    Rgb(u8, u8, u8),  // True color
}

impl Color {
    pub fn to_ansi_fg(&self) -> String {
        match self {
            Color::Reset => "\x1b[39m".to_string(),
            Color::Black => "\x1b[30m".to_string(),
            Color::Red => "\x1b[31m".to_string(),
            // ... other basic colors
            Color::Indexed(n) => format!("\x1b[38;5;{}m", n),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        }
    }
}
```
