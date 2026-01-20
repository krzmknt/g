use crate::tui::{Buffer, Rect, Style};

pub struct Scrollbar {
    pub total_items: usize,
    pub visible_items: usize,
    pub scroll_position: usize,
}

impl Scrollbar {
    pub fn new(total_items: usize, visible_items: usize, scroll_position: usize) -> Self {
        Self {
            total_items,
            visible_items,
            scroll_position,
        }
    }

    /// Render a vertical scrollbar on the right edge of the given area
    pub fn render(&self, area: Rect, buf: &mut Buffer, style: Style) {
        if self.total_items == 0 || self.visible_items >= self.total_items {
            // No scrollbar needed - just draw empty track
            let x = area.x;
            for i in 0..area.height as usize {
                let y = area.y + i as u16;
                buf.set_string(x, y, " ", style);
            }
            return;
        }

        let track_height = area.height as usize;
        if track_height == 0 {
            return;
        }

        // Calculate thumb size and position
        let thumb_height = (self.visible_items * track_height / self.total_items).max(1);
        let max_scroll = self.total_items.saturating_sub(self.visible_items);
        let thumb_position = if max_scroll > 0 {
            self.scroll_position * (track_height - thumb_height) / max_scroll
        } else {
            0
        };

        let x = area.x;

        // Draw track and thumb using ASCII characters
        for i in 0..track_height {
            let y = area.y + i as u16;
            let is_thumb = i >= thumb_position && i < thumb_position + thumb_height;
            // Use '#' for thumb, '|' for track
            let symbol = if is_thumb { "#" } else { "|" };
            buf.set_string(x, y, symbol, style);
        }
    }
}
