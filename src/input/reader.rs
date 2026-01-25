use super::event::{Event, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::error::Result;
use std::io::Read;
use std::time::Duration;

pub struct EventReader {
    buffer: [u8; 32],
    buffer_len: usize,
}

impl EventReader {
    pub fn new() -> Self {
        Self {
            buffer: [0; 32],
            buffer_len: 0,
        }
    }

    pub fn read_event(&mut self, timeout: Duration) -> Result<Event> {
        // If we have bytes in the buffer, try to parse them first
        if self.buffer_len > 0 {
            let (event, consumed) = self.parse_event();
            if consumed > 0 {
                self.buffer.copy_within(consumed..self.buffer_len, 0);
                self.buffer_len -= consumed;
                if !matches!(event, Event::None) {
                    return Ok(event);
                }
            }
        }

        // Try to read from stdin with timeout
        if !self.poll_stdin(timeout)? {
            return Ok(Event::None);
        }

        // Read available bytes
        let n = std::io::stdin().read(&mut self.buffer[self.buffer_len..])?;
        if n == 0 {
            return Ok(Event::None);
        }
        self.buffer_len += n;

        // Parse event
        let (event, consumed) = self.parse_event();

        // Remove consumed bytes
        if consumed > 0 {
            self.buffer.copy_within(consumed..self.buffer_len, 0);
            self.buffer_len -= consumed;
        }

        Ok(event)
    }

    #[cfg(unix)]
    fn poll_stdin(&self, timeout: Duration) -> Result<bool> {
        use std::os::unix::io::AsRawFd;

        unsafe {
            let mut fds: libc::fd_set = std::mem::zeroed();
            libc::FD_ZERO(&mut fds);
            libc::FD_SET(libc::STDIN_FILENO, &mut fds);

            let mut tv = libc::timeval {
                tv_sec: timeout.as_secs() as libc::time_t,
                tv_usec: timeout.subsec_micros() as libc::suseconds_t,
            };

            let result = libc::select(
                libc::STDIN_FILENO + 1,
                &mut fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            );

            Ok(result > 0)
        }
    }

    #[cfg(windows)]
    fn poll_stdin(&self, timeout: Duration) -> Result<bool> {
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Console::*;

        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            let timeout_ms = timeout.as_millis() as u32;

            let result =
                windows_sys::Win32::System::Threading::WaitForSingleObject(handle as _, timeout_ms);

            Ok(result == WAIT_OBJECT_0)
        }
    }

    fn parse_event(&self) -> (Event, usize) {
        if self.buffer_len == 0 {
            return (Event::None, 0);
        }

        let bytes = &self.buffer[..self.buffer_len];

        // Check for escape sequences
        if bytes[0] == 0x1b {
            if self.buffer_len == 1 {
                // Just escape
                return (
                    Event::Key(KeyEvent::new(KeyCode::Escape, Modifiers::NONE)),
                    1,
                );
            }

            if bytes.len() >= 2 && bytes[1] == b'[' {
                return self.parse_csi_sequence(&bytes[2..]);
            }

            if bytes.len() >= 2 && bytes[1] == b'O' {
                return self.parse_ss3_sequence(&bytes[2..]);
            }

            // Alt + key
            if bytes.len() >= 2 {
                let (inner_event, inner_consumed) = self.parse_single_byte(bytes[1]);
                if let Event::Key(mut key) = inner_event {
                    key.modifiers = key.modifiers.union(Modifiers::ALT);
                    return (Event::Key(key), 1 + inner_consumed);
                }
            }

            return (
                Event::Key(KeyEvent::new(KeyCode::Escape, Modifiers::NONE)),
                1,
            );
        }

        self.parse_single_byte(bytes[0])
    }

    fn parse_single_byte(&self, byte: u8) -> (Event, usize) {
        let event = match byte {
            0 => KeyEvent::new(KeyCode::Null, Modifiers::NONE),
            // Tab (0x09) - must be before Ctrl range
            9 => KeyEvent::new(KeyCode::Tab, Modifiers::NONE),
            // Enter (0x0A, 0x0D) - must be before Ctrl range
            10 | 13 => KeyEvent::new(KeyCode::Enter, Modifiers::NONE),
            // Ctrl+A through Ctrl+Z (excluding Tab=9, LF=10, CR=13)
            1..=8 | 11..=12 | 14..=26 => {
                let c = (byte - 1 + b'a') as char;
                KeyEvent::new(KeyCode::Char(c), Modifiers::CTRL)
            }
            27 => KeyEvent::new(KeyCode::Escape, Modifiers::NONE),
            127 => KeyEvent::new(KeyCode::Backspace, Modifiers::NONE),
            32..=126 => KeyEvent::new(KeyCode::Char(byte as char), Modifiers::NONE),
            _ => {
                // Try to parse as UTF-8
                if let Some((c, len)) = self.parse_utf8() {
                    return (
                        Event::Key(KeyEvent::new(KeyCode::Char(c), Modifiers::NONE)),
                        len,
                    );
                }
                return (Event::None, 1);
            }
        };

        (Event::Key(event), 1)
    }

    fn parse_utf8(&self) -> Option<(char, usize)> {
        let bytes = &self.buffer[..self.buffer_len];
        let s = std::str::from_utf8(bytes).ok()?;
        let c = s.chars().next()?;
        Some((c, c.len_utf8()))
    }

    fn parse_csi_sequence(&self, bytes: &[u8]) -> (Event, usize) {
        // CSI sequences: ESC [ ...
        // Base offset is 2 (ESC [)

        if bytes.is_empty() {
            return (
                Event::Key(KeyEvent::new(KeyCode::Escape, Modifiers::NONE)),
                1,
            );
        }

        // Check for SGR mouse encoding: ESC [ < Cb ; Cx ; Cy M/m
        if bytes[0] == b'<' {
            return self.parse_sgr_mouse(&bytes[1..]);
        }

        // Check for normal mouse encoding: ESC [ M Cb Cx Cy
        if bytes[0] == b'M' && bytes.len() >= 4 {
            return self.parse_normal_mouse(&bytes[1..]);
        }

        // Simple arrow keys and navigation
        match bytes[0] {
            b'A' => return (Event::Key(KeyEvent::new(KeyCode::Up, Modifiers::NONE)), 3),
            b'B' => return (Event::Key(KeyEvent::new(KeyCode::Down, Modifiers::NONE)), 3),
            b'C' => {
                return (
                    Event::Key(KeyEvent::new(KeyCode::Right, Modifiers::NONE)),
                    3,
                )
            }
            b'D' => return (Event::Key(KeyEvent::new(KeyCode::Left, Modifiers::NONE)), 3),
            b'H' => return (Event::Key(KeyEvent::new(KeyCode::Home, Modifiers::NONE)), 3),
            b'F' => return (Event::Key(KeyEvent::new(KeyCode::End, Modifiers::NONE)), 3),
            b'Z' => {
                return (
                    Event::Key(KeyEvent::new(KeyCode::BackTab, Modifiers::SHIFT)),
                    3,
                )
            }
            _ => {}
        }

        // Extended sequences: ESC [ number ~
        // or ESC [ number ; modifier ~
        let mut num = 0u32;
        let mut modifier = 0u32;
        let mut i = 0;
        let mut saw_semicolon = false;

        while i < bytes.len() {
            match bytes[i] {
                b'0'..=b'9' => {
                    let digit = (bytes[i] - b'0') as u32;
                    if saw_semicolon {
                        modifier = modifier * 10 + digit;
                    } else {
                        num = num * 10 + digit;
                    }
                }
                b';' => saw_semicolon = true,
                b'~' => {
                    let code = match num {
                        1 => KeyCode::Home,
                        2 => KeyCode::Insert,
                        3 => KeyCode::Delete,
                        4 => KeyCode::End,
                        5 => KeyCode::PageUp,
                        6 => KeyCode::PageDown,
                        7 => KeyCode::Home,
                        8 => KeyCode::End,
                        11..=15 => KeyCode::F((num - 10) as u8),
                        17..=21 => KeyCode::F((num - 11) as u8),
                        23..=24 => KeyCode::F((num - 12) as u8),
                        _ => return (Event::None, i + 3),
                    };
                    let modifiers = self.parse_modifier(modifier);
                    return (Event::Key(KeyEvent::new(code, modifiers)), i + 3);
                }
                b'A'..=b'D' => {
                    // Arrow keys with modifiers: ESC [ 1 ; modifier A/B/C/D
                    let code = match bytes[i] {
                        b'A' => KeyCode::Up,
                        b'B' => KeyCode::Down,
                        b'C' => KeyCode::Right,
                        b'D' => KeyCode::Left,
                        _ => unreachable!(),
                    };
                    let modifiers = self.parse_modifier(modifier);
                    return (Event::Key(KeyEvent::new(code, modifiers)), i + 3);
                }
                _ => break,
            }
            i += 1;
        }

        (Event::None, 2)
    }

    fn parse_sgr_mouse(&self, bytes: &[u8]) -> (Event, usize) {
        // SGR mouse: ESC [ < Cb ; Cx ; Cy M/m
        // Format: <button_code;x;y[Mm]
        // M = button press, m = button release

        let mut nums: [u32; 3] = [0, 0, 0];
        let mut num_idx = 0;
        let mut i = 0;

        while i < bytes.len() && num_idx < 3 {
            match bytes[i] {
                b'0'..=b'9' => {
                    nums[num_idx] = nums[num_idx] * 10 + (bytes[i] - b'0') as u32;
                }
                b';' => {
                    num_idx += 1;
                }
                b'M' | b'm' => {
                    let is_release = bytes[i] == b'm';
                    let cb = nums[0];
                    let cx = nums[1].saturating_sub(1) as u16; // 1-based to 0-based
                    let cy = nums[2].saturating_sub(1) as u16;

                    let button = match cb & 0b11 {
                        0 => MouseButton::Left,
                        1 => MouseButton::Middle,
                        2 => MouseButton::Right,
                        _ => MouseButton::Left,
                    };

                    let is_drag = (cb & 32) != 0;
                    let is_scroll = (cb & 64) != 0;

                    let kind = if is_scroll {
                        if (cb & 0b1) == 0 {
                            MouseEventKind::ScrollUp
                        } else {
                            MouseEventKind::ScrollDown
                        }
                    } else if is_release {
                        MouseEventKind::Up(button)
                    } else if is_drag {
                        MouseEventKind::Drag(button)
                    } else {
                        MouseEventKind::Down(button)
                    };

                    let event = MouseEvent {
                        kind,
                        column: cx,
                        row: cy,
                    };
                    // Total consumed: ESC [ < (3) + parsed bytes + terminator
                    return (Event::Mouse(event), 3 + i + 1);
                }
                _ => {
                    // Invalid character in SGR sequence - skip the entire sequence
                    // Find the next M or m terminator to consume all garbage
                    while i < bytes.len() {
                        if bytes[i] == b'M' || bytes[i] == b'm' {
                            return (Event::None, 3 + i + 1);
                        }
                        i += 1;
                    }
                    // No terminator found - sequence is incomplete, wait for more bytes
                    return (Event::None, 0);
                }
            }
            i += 1;
        }

        // Sequence is incomplete (no M/m terminator found yet)
        // Don't consume anything, wait for more bytes
        (Event::None, 0)
    }

    fn parse_normal_mouse(&self, bytes: &[u8]) -> (Event, usize) {
        // Normal mouse: ESC [ M Cb Cx Cy (each is a single byte + 32 offset)
        if bytes.len() < 3 {
            return (Event::None, 3);
        }

        let cb = bytes[0].wrapping_sub(32);
        let cx = bytes[1].wrapping_sub(32).saturating_sub(1) as u16;
        let cy = bytes[2].wrapping_sub(32).saturating_sub(1) as u16;

        let button = match cb & 0b11 {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 => {
                // Button release (no specific button in normal mode)
                let event = MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Left),
                    column: cx,
                    row: cy,
                };
                return (Event::Mouse(event), 6);
            }
            _ => MouseButton::Left,
        };

        let is_drag = (cb & 32) != 0;
        let is_scroll = (cb & 64) != 0;

        let kind = if is_scroll {
            if (cb & 0b1) == 0 {
                MouseEventKind::ScrollUp
            } else {
                MouseEventKind::ScrollDown
            }
        } else if is_drag {
            MouseEventKind::Drag(button)
        } else {
            MouseEventKind::Down(button)
        };

        let event = MouseEvent {
            kind,
            column: cx,
            row: cy,
        };
        (Event::Mouse(event), 6)
    }

    fn parse_ss3_sequence(&self, bytes: &[u8]) -> (Event, usize) {
        // SS3 sequences: ESC O ...
        if bytes.is_empty() {
            return (
                Event::Key(KeyEvent::new(KeyCode::Escape, Modifiers::NONE)),
                1,
            );
        }

        let event = match bytes[0] {
            b'A' => KeyEvent::new(KeyCode::Up, Modifiers::NONE),
            b'B' => KeyEvent::new(KeyCode::Down, Modifiers::NONE),
            b'C' => KeyEvent::new(KeyCode::Right, Modifiers::NONE),
            b'D' => KeyEvent::new(KeyCode::Left, Modifiers::NONE),
            b'H' => KeyEvent::new(KeyCode::Home, Modifiers::NONE),
            b'F' => KeyEvent::new(KeyCode::End, Modifiers::NONE),
            b'P' => KeyEvent::new(KeyCode::F(1), Modifiers::NONE),
            b'Q' => KeyEvent::new(KeyCode::F(2), Modifiers::NONE),
            b'R' => KeyEvent::new(KeyCode::F(3), Modifiers::NONE),
            b'S' => KeyEvent::new(KeyCode::F(4), Modifiers::NONE),
            _ => return (Event::None, 3),
        };

        (Event::Key(event), 3)
    }

    fn parse_modifier(&self, modifier: u32) -> Modifiers {
        // xterm modifier encoding: modifier = 1 + (shift) + 2*(alt) + 4*(ctrl)
        let m = modifier.saturating_sub(1);
        let mut result = Modifiers::NONE;
        if m & 1 != 0 {
            result = result.union(Modifiers::SHIFT);
        }
        if m & 2 != 0 {
            result = result.union(Modifiers::ALT);
        }
        if m & 4 != 0 {
            result = result.union(Modifiers::CTRL);
        }
        result
    }
}
