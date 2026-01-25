use super::buffer::{Buffer, Cell};
use super::render::Rect;
use super::style::Style;
use crate::error::{Error, Result};
use std::io::{self, BufWriter, Stdout, Write};

pub struct Terminal {
    stdout: BufWriter<Stdout>,
    current_buffer: Buffer,
    previous_buffer: Buffer,
    first_draw: bool,
    #[cfg(unix)]
    original_termios: Option<libc::termios>,
    #[cfg(windows)]
    original_mode: Option<(u32, u32)>,
}

impl Terminal {
    pub fn new() -> Result<Self> {
        let stdout = BufWriter::with_capacity(8192, io::stdout());
        let size = Self::size_static()?;
        let area = Rect::new(0, 0, size.0, size.1);

        Ok(Self {
            stdout,
            current_buffer: Buffer::empty(area),
            previous_buffer: Buffer::empty(area),
            first_draw: true,
            #[cfg(unix)]
            original_termios: None,
            #[cfg(windows)]
            original_mode: None,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.enable_raw_mode()?;
        self.enter_alternate_screen()?;
        self.hide_cursor()?;
        self.enable_mouse()?;
        self.clear()?;
        Ok(())
    }

    pub fn restore(&mut self) -> Result<()> {
        self.disable_mouse()?;
        self.show_cursor()?;
        self.leave_alternate_screen()?;
        self.disable_raw_mode()?;
        Ok(())
    }

    fn enable_mouse(&mut self) -> Result<()> {
        // Enable mouse tracking:
        // 1000 = normal tracking (press/release)
        // 1002 = button-event tracking (press/release/drag)
        // 1003 = any-event tracking (all mouse events)
        // 1006 = SGR extended mode (better coordinate handling)
        write!(self.stdout, "\x1b[?1000h\x1b[?1002h\x1b[?1006h")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn disable_mouse(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b[?1006l\x1b[?1002l\x1b[?1000l")?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn size(&self) -> Result<(u16, u16)> {
        Self::size_static()
    }

    #[cfg(unix)]
    fn size_static() -> Result<(u16, u16)> {
        unsafe {
            let mut size: libc::winsize = std::mem::zeroed();
            if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) == 0 {
                Ok((size.ws_col, size.ws_row))
            } else {
                Err(Error::Terminal("Failed to get terminal size".to_string()))
            }
        }
    }

    #[cfg(windows)]
    fn size_static() -> Result<(u16, u16)> {
        use windows_sys::Win32::System::Console::*;
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(handle, &mut info) != 0 {
                let width = (info.srWindow.Right - info.srWindow.Left + 1) as u16;
                let height = (info.srWindow.Bottom - info.srWindow.Top + 1) as u16;
                Ok((width, height))
            } else {
                Err(Error::Terminal("Failed to get terminal size".to_string()))
            }
        }
    }

    #[cfg(unix)]
    fn enable_raw_mode(&mut self) -> Result<()> {
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) != 0 {
                return Err(Error::Terminal(
                    "Failed to get terminal attributes".to_string(),
                ));
            }

            self.original_termios = Some(termios);

            // Disable canonical mode, echo, and signals
            termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN);
            // Disable input processing (but keep ICRNL for proper input)
            termios.c_iflag &= !(libc::IXON | libc::BRKINT | libc::INPCK | libc::ISTRIP);
            // Keep output processing enabled for proper terminal output
            // termios.c_oflag &= !libc::OPOST;  // Don't disable this
            // Set character size to 8 bits
            termios.c_cflag |= libc::CS8;
            // Read returns immediately
            termios.c_cc[libc::VMIN] = 0;
            termios.c_cc[libc::VTIME] = 1; // 100ms timeout

            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &termios) != 0 {
                return Err(Error::Terminal(
                    "Failed to set terminal attributes".to_string(),
                ));
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn disable_raw_mode(&mut self) -> Result<()> {
        if let Some(termios) = self.original_termios.take() {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &termios);
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn enable_raw_mode(&mut self) -> Result<()> {
        use windows_sys::Win32::System::Console::*;
        unsafe {
            let in_handle = GetStdHandle(STD_INPUT_HANDLE);
            let out_handle = GetStdHandle(STD_OUTPUT_HANDLE);

            let mut in_mode = 0u32;
            let mut out_mode = 0u32;
            GetConsoleMode(in_handle, &mut in_mode);
            GetConsoleMode(out_handle, &mut out_mode);

            self.original_mode = Some((in_mode, out_mode));

            // Disable line input and echo
            let new_in_mode = (in_mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            // Enable virtual terminal processing
            let new_out_mode = out_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;

            SetConsoleMode(in_handle, new_in_mode);
            SetConsoleMode(out_handle, new_out_mode);
        }
        Ok(())
    }

    #[cfg(windows)]
    fn disable_raw_mode(&mut self) -> Result<()> {
        use windows_sys::Win32::System::Console::*;
        if let Some((in_mode, out_mode)) = self.original_mode.take() {
            unsafe {
                let in_handle = GetStdHandle(STD_INPUT_HANDLE);
                let out_handle = GetStdHandle(STD_OUTPUT_HANDLE);
                SetConsoleMode(in_handle, in_mode);
                SetConsoleMode(out_handle, out_mode);
            }
        }
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b[?1049h")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b[?1049l")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b[?25l")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b[?25h")?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        write!(self.stdout, "\x1b[2J\x1b[H")?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        let area = Rect::new(0, 0, width, height);
        self.current_buffer.resize(area);
        self.previous_buffer.resize(area);
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.current_buffer
    }

    /// Force a full redraw on the next draw call
    pub fn force_full_redraw(&mut self) {
        self.first_draw = true;
    }

    /// Invalidate the previous buffer to force diff to detect all changes
    /// More efficient than full redraw for layout changes
    pub fn invalidate_buffer(&mut self) {
        self.previous_buffer.clear();
    }

    pub fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        // Check if terminal was resized
        let (width, height) = self.size()?;
        if width != self.current_buffer.area.width || height != self.current_buffer.area.height {
            self.resize(width, height);
            self.first_draw = true; // Force full redraw on resize
        }

        // Clear current buffer and draw
        self.current_buffer.clear();
        f(&mut self.current_buffer);

        // Render - full render on first draw, diff render otherwise
        if self.first_draw {
            self.flush_full()?;
            self.first_draw = false;
        } else {
            self.flush()?;
        }

        // Swap buffers
        std::mem::swap(&mut self.current_buffer, &mut self.previous_buffer);

        Ok(())
    }

    fn flush_full(&mut self) -> Result<()> {
        // Move to home position (no screen clear to avoid flicker)
        write!(self.stdout, "\x1b[H")?;

        let mut last_style = Style::default();
        let area = self.current_buffer.area;

        for y in area.y..area.y + area.height {
            // Move cursor to start of line
            write!(self.stdout, "\x1b[{};{}H", y + 1, 1)?;

            for x in area.x..area.x + area.width {
                let cell = self.current_buffer.get(x, y);

                // Skip continuation cells
                if cell.symbol.is_empty() {
                    continue;
                }

                // Apply style if changed
                let style = cell.style();
                if style != last_style {
                    write!(self.stdout, "{}", style.to_ansi())?;
                    last_style = style;
                }

                // Write character
                write!(self.stdout, "{}", cell.symbol)?;
            }
        }

        // Reset style at end
        write!(self.stdout, "\x1b[0m")?;
        self.stdout.flush()?;

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let updates = self.previous_buffer.diff(&self.current_buffer);

        if updates.is_empty() {
            return Ok(());
        }

        let mut last_style = Style::default();
        let mut last_pos: Option<(u16, u16)> = None;

        for (x, y, cell) in updates {
            // Skip continuation cells (empty symbol = second half of wide char)
            if cell.symbol.is_empty() {
                continue;
            }

            // Move cursor if not adjacent
            let need_move = match last_pos {
                Some((lx, ly)) => !(ly == y && lx + 1 == x),
                None => true,
            };

            if need_move {
                write!(self.stdout, "\x1b[{};{}H", y + 1, x + 1)?;
            }

            // Apply style if changed
            let style = cell.style();
            if style != last_style {
                write!(self.stdout, "{}", style.to_ansi())?;
                last_style = style;
            }

            // Write character
            write!(self.stdout, "{}", cell.symbol)?;

            // Update position tracking - wide chars advance by their width
            let char_width = if cell
                .symbol
                .chars()
                .next()
                .map(|c| !c.is_ascii())
                .unwrap_or(false)
            {
                2u16
            } else {
                1u16
            };
            last_pos = Some((x + char_width - 1, y));
        }

        // Reset style at end
        write!(self.stdout, "\x1b[0m")?;
        self.stdout.flush()?;

        Ok(())
    }

    pub fn area(&self) -> Rect {
        self.current_buffer.area
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
