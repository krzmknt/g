#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub const fn area(&self) -> u16 {
        self.width * self.height
    }

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub const fn left(&self) -> u16 {
        self.x
    }

    pub const fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub const fn top(&self) -> u16 {
        self.y
    }

    pub const fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub fn inner(&self, margin: u16) -> Self {
        if self.width < margin * 2 || self.height < margin * 2 {
            Rect::default()
        } else {
            Rect {
                x: self.x + margin,
                y: self.y + margin,
                width: self.width - margin * 2,
                height: self.height - margin * 2,
            }
        }
    }

    pub fn intersection(&self, other: Rect) -> Rect {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        Rect {
            x: x1,
            y: y1,
            width: x2.saturating_sub(x1),
            height: y2.saturating_sub(y1),
        }
    }

    pub fn union(&self, other: Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self.right().max(other.right());
        let y2 = self.bottom().max(other.bottom());

        Rect {
            x: x1,
            y: y1,
            width: x2.saturating_sub(x1),
            height: y2.saturating_sub(y1),
        }
    }

    /// Split horizontally at the given position (from top)
    pub fn split_horizontal(&self, at: u16) -> (Rect, Rect) {
        let at = at.min(self.height);
        (
            Rect::new(self.x, self.y, self.width, at),
            Rect::new(self.x, self.y + at, self.width, self.height.saturating_sub(at)),
        )
    }

    /// Split vertically at the given position (from left)
    pub fn split_vertical(&self, at: u16) -> (Rect, Rect) {
        let at = at.min(self.width);
        (
            Rect::new(self.x, self.y, at, self.height),
            Rect::new(self.x + at, self.y, self.width.saturating_sub(at), self.height),
        )
    }
}
