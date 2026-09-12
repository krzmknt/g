use super::Widget;
use crate::tui::{Buffer, Rect, Style};

/// A one-column vertical rule used to divide adjacent panel columns.
///
/// Rows where a horizontal title rule touches the separator from the left
/// or right are joined with the matching tee glyph so the layout reads as a
/// single grid instead of overlapping lines.
#[derive(Debug, Clone, Default)]
pub struct VerticalSeparator<'a> {
    style: Style,
    left_rules: &'a [u16],
    right_rules: &'a [u16],
}

impl<'a> VerticalSeparator<'a> {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            left_rules: &[],
            right_rules: &[],
        }
    }

    /// Absolute rows where a horizontal rule ends on the left side.
    pub fn left_rules(mut self, rows: &'a [u16]) -> Self {
        self.left_rules = rows;
        self
    }

    /// Absolute rows where a horizontal rule starts on the right side.
    pub fn right_rules(mut self, rows: &'a [u16]) -> Self {
        self.right_rules = rows;
        self
    }

    fn symbol_for(&self, y: u16, top: u16) -> &'static str {
        let left = self.left_rules.contains(&y);
        let right = self.right_rules.contains(&y);
        match (left, right) {
            (true, true) if y == top => "┬",
            (true, true) => "┼",
            (true, false) => "┤",
            (false, true) => "├",
            (false, false) => "│",
        }
    }
}

impl Widget for VerticalSeparator<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        for y in area.y..area.bottom() {
            buf.get_mut(area.x, y)
                .set_symbol(self.symbol_for(y, area.y))
                .set_style(self.style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::Rect;

    fn symbols(buf: &Buffer, x: u16, height: u16) -> Vec<String> {
        (0..height).map(|y| buf.get(x, y).symbol.clone()).collect()
    }

    #[test]
    fn renders_plain_vertical_line() {
        let area = Rect::new(2, 0, 1, 3);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 3));
        VerticalSeparator::new(Style::default()).render(area, &mut buf);
        assert_eq!(symbols(&buf, 2, 3), vec!["│", "│", "│"]);
    }

    #[test]
    fn uses_tee_at_top_row_when_both_sides_have_rules() {
        let area = Rect::new(2, 0, 1, 3);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 3));
        VerticalSeparator::new(Style::default())
            .left_rules(&[0])
            .right_rules(&[0])
            .render(area, &mut buf);
        assert_eq!(symbols(&buf, 2, 3), vec!["┬", "│", "│"]);
    }

    #[test]
    fn joins_rules_from_either_side() {
        let area = Rect::new(2, 0, 1, 4);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 4));
        VerticalSeparator::new(Style::default())
            .left_rules(&[1, 3])
            .right_rules(&[2, 3])
            .render(area, &mut buf);
        assert_eq!(symbols(&buf, 2, 4), vec!["│", "┤", "├", "┼"]);
    }

    #[test]
    fn ignores_rules_outside_area() {
        let area = Rect::new(0, 1, 1, 2);
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 4));
        VerticalSeparator::new(Style::default())
            .left_rules(&[0, 3])
            .render(area, &mut buf);
        assert_eq!(buf.get(0, 0).symbol, " ");
        assert_eq!(buf.get(0, 3).symbol, " ");
        assert_eq!(symbols(&buf, 0, 4), vec![" ", "│", "│", " "]);
    }

    #[test]
    fn renders_nothing_for_empty_area() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
        VerticalSeparator::new(Style::default()).render(Rect::new(1, 0, 0, 3), &mut buf);
        assert_eq!(symbols(&buf, 1, 3), vec![" ", " ", " "]);
    }
}
