use crate::tui::{Buffer, Rect, Style};
use super::Widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Borders(u8);

impl Borders {
    pub const NONE: Self = Self(0);
    pub const TOP: Self = Self(1);
    pub const RIGHT: Self = Self(2);
    pub const BOTTOM: Self = Self(4);
    pub const LEFT: Self = Self(8);
    pub const ALL: Self = Self(15);

    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Block<'a> {
    title: Option<&'a str>,
    borders: Borders,
    border_style: Style,
    title_style: Style,
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    pub fn inner(&self, area: Rect) -> Rect {
        let mut x = area.x;
        let mut y = area.y;
        let mut width = area.width;
        let mut height = area.height;

        if self.borders.contains(Borders::LEFT) {
            x = x.saturating_add(1);
            width = width.saturating_sub(1);
        }
        if self.borders.contains(Borders::TOP) {
            y = y.saturating_add(1);
            height = height.saturating_sub(1);
        }
        if self.borders.contains(Borders::RIGHT) {
            width = width.saturating_sub(1);
        }
        if self.borders.contains(Borders::BOTTOM) {
            height = height.saturating_sub(1);
        }

        Rect::new(x, y, width, height)
    }
}

impl Widget for Block<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        // Box drawing characters (rounded corners)
        const HORIZONTAL: &str = "─";
        const VERTICAL: &str = "│";
        const TOP_LEFT: &str = "╭";
        const TOP_RIGHT: &str = "╮";
        const BOTTOM_LEFT: &str = "╰";
        const BOTTOM_RIGHT: &str = "╯";

        // Draw borders
        if self.borders.contains(Borders::TOP) {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, area.y).set_symbol(HORIZONTAL).set_style(self.border_style);
            }
        }

        if self.borders.contains(Borders::BOTTOM) {
            let y = area.y + area.height - 1;
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_symbol(HORIZONTAL).set_style(self.border_style);
            }
        }

        if self.borders.contains(Borders::LEFT) {
            for y in area.y..area.y + area.height {
                buf.get_mut(area.x, y).set_symbol(VERTICAL).set_style(self.border_style);
            }
        }

        if self.borders.contains(Borders::RIGHT) {
            let x = area.x + area.width - 1;
            for y in area.y..area.y + area.height {
                buf.get_mut(x, y).set_symbol(VERTICAL).set_style(self.border_style);
            }
        }

        // Corners
        if self.borders.contains(Borders::TOP) && self.borders.contains(Borders::LEFT) {
            buf.get_mut(area.x, area.y).set_symbol(TOP_LEFT).set_style(self.border_style);
        }

        if self.borders.contains(Borders::TOP) && self.borders.contains(Borders::RIGHT) {
            buf.get_mut(area.x + area.width - 1, area.y).set_symbol(TOP_RIGHT).set_style(self.border_style);
        }

        if self.borders.contains(Borders::BOTTOM) && self.borders.contains(Borders::LEFT) {
            buf.get_mut(area.x, area.y + area.height - 1).set_symbol(BOTTOM_LEFT).set_style(self.border_style);
        }

        if self.borders.contains(Borders::BOTTOM) && self.borders.contains(Borders::RIGHT) {
            buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_symbol(BOTTOM_RIGHT).set_style(self.border_style);
        }

        // Title
        if let Some(title) = self.title {
            if self.borders.contains(Borders::TOP) && area.width > 4 {
                let title_x = area.x + 2;
                let max_width = area.width.saturating_sub(4);
                buf.set_string_truncated(title_x, area.y, title, max_width, self.title_style);
            }
        }
    }
}
