use crate::tui::{Buffer, Color, Rect, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub foreground: Color,
    pub border: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub selection: Color,
    pub diff_add: Color,
    pub diff_remove: Color,
    pub diff_add_bg: Color,
    pub diff_remove_bg: Color,
    pub diff_hunk: Color,
    pub staged: Color,
    pub unstaged: Color,
    pub untracked: Color,
    pub branch_current: Color,
    pub branch_local: Color,
    pub branch_remote: Color,
    // Commit-specific colors
    pub commit_hash: Color,
    pub commit_author: Color,
    pub commit_message: Color,
    pub commit_time: Color,
    pub commit_refs: Color,
}

/// Glyph drawn in the gutter next to the selected row. The row's content
/// cells are underlined in the same color so the ribbon's bottom edge
/// continues to the right as a bottom border.
pub const SELECTION_RIBBON: &str = "▌";

pub const HIGHLIGHT_COLORS: &[(Color, &str)] = &[
    (Color::Rgb(255, 140, 0), "orange"),
    (Color::Rgb(137, 180, 250), "blue"),
    (Color::Rgb(166, 227, 161), "green"),
    (Color::Rgb(243, 139, 168), "red"),
    (Color::Rgb(203, 166, 247), "purple"),
    (Color::Rgb(250, 179, 135), "peach"),
    (Color::Rgb(137, 220, 235), "cyan"),
    (Color::Rgb(249, 226, 175), "yellow"),
    (Color::Rgb(242, 205, 205), "pink"),
    (Color::White, "white"),
];

impl Theme {
    pub fn default() -> Self {
        Self {
            foreground: Color::Rgb(205, 214, 244),     // #cdd6f4
            border: Color::Rgb(108, 112, 134),         // #6c7086
            border_focused: Color::Rgb(137, 180, 250), // #89b4fa
            border_unfocused: Color::Rgb(69, 71, 90),  // #45475a (dimmer)
            selection: Color::Rgb(255, 140, 0),        // Orange background
            diff_add: Color::Rgb(166, 227, 161),       // #a6e3a1
            diff_remove: Color::Rgb(243, 139, 168),    // #f38ba8
            diff_add_bg: Color::Rgb(30, 60, 30),       // Dark green background
            diff_remove_bg: Color::Rgb(60, 30, 30),    // Dark red background
            diff_hunk: Color::Rgb(137, 220, 235),      // #89dceb
            staged: Color::Rgb(166, 227, 161),         // #a6e3a1
            unstaged: Color::Rgb(147, 153, 178),       // #9399b2 (gray)
            untracked: Color::Rgb(108, 112, 134),      // #6c7086
            branch_current: Color::Rgb(166, 227, 161), // #a6e3a1
            branch_local: Color::Rgb(137, 180, 250),   // #89b4fa
            branch_remote: Color::Rgb(203, 166, 247),  // #cba6f7
            // Commit-specific colors
            commit_hash: Color::Rgb(137, 220, 235), // #89dceb (cyan)
            commit_author: Color::Rgb(108, 112, 134), // #6c7086 (dim gray, same as time)
            commit_message: Color::Rgb(205, 214, 244), // #cdd6f4 (foreground)
            commit_time: Color::Rgb(108, 112, 134), // #6c7086 (dim)
            commit_refs: Color::Rgb(166, 227, 161), // #a6e3a1 (green)
        }
    }

    /// Mark a row as selected: a ribbon is drawn in the one-column `gutter`
    /// and `row` (the row's content cells) gets a bottom border in the
    /// selection color. The two join at the bottom-left corner. Text colors
    /// are untouched.
    pub fn mark_selected_row(&self, buf: &mut Buffer, gutter: Rect, row: Rect) {
        if gutter.width == 0 || row.y < gutter.y || row.y >= gutter.bottom() {
            return;
        }
        let border = Style::new().underline().underline_color(self.selection);
        buf.set_string(gutter.x, row.y, SELECTION_RIBBON, border.fg(self.selection));
        buf.set_style(Rect::new(row.x, row.y, row.width, 1), border);
    }

    pub fn highlight_color_index(&self) -> usize {
        HIGHLIGHT_COLORS
            .iter()
            .position(|(c, _)| *c == self.selection)
            .unwrap_or(0)
    }

    pub fn cycle_highlight_color(&mut self) -> (Color, &'static str) {
        let current = self.highlight_color_index();
        let next = (current + 1) % HIGHLIGHT_COLORS.len();
        let (color, name) = HIGHLIGHT_COLORS[next];
        self.selection = color;
        self.border_focused = color;
        (color, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_highlight_is_orange() {
        let theme = Theme::default();
        assert_eq!(theme.selection, Color::Rgb(255, 140, 0));
        assert_eq!(theme.highlight_color_index(), 0);
    }

    #[test]
    fn test_cycle_highlight_color_advances() {
        let mut theme = Theme::default();
        let (color, name) = theme.cycle_highlight_color();
        assert_eq!(color, Color::Rgb(137, 180, 250));
        assert_eq!(name, "blue");
        assert_eq!(theme.selection, Color::Rgb(137, 180, 250));
        assert_eq!(theme.border_focused, Color::Rgb(137, 180, 250));
    }

    #[test]
    fn test_cycle_highlight_color_wraps_around() {
        let mut theme = Theme::default();
        for _ in 0..HIGHLIGHT_COLORS.len() {
            theme.cycle_highlight_color();
        }
        assert_eq!(theme.selection, Color::Rgb(255, 140, 0));
        assert_eq!(theme.highlight_color_index(), 0);
    }

    #[test]
    fn test_highlight_color_index_unknown_color_defaults_to_zero() {
        let mut theme = Theme::default();
        theme.selection = Color::Rgb(1, 2, 3);
        assert_eq!(theme.highlight_color_index(), 0);
    }

    #[test]
    fn mark_selected_row_draws_ribbon_and_underlines_the_row() {
        use crate::tui::{Modifier, Style};
        let theme = Theme::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 3));
        buf.set_string(1, 1, "ab", Style::new().fg(theme.commit_hash));
        let gutter = Rect::new(0, 0, 1, 3);
        let row = Rect::new(1, 1, 3, 1);

        theme.mark_selected_row(&mut buf, gutter, row);

        let ribbon = buf.get(0, 1);
        assert_eq!(ribbon.symbol, SELECTION_RIBBON);
        assert_eq!(ribbon.fg, Some(theme.selection));
        assert!(ribbon.modifier.contains(Modifier::UNDERLINE));
        assert_eq!(ribbon.underline_color, Some(theme.selection));

        for x in 1..4 {
            let cell = buf.get(x, 1);
            assert!(cell.modifier.contains(Modifier::UNDERLINE), "x={}", x);
            assert_eq!(cell.underline_color, Some(theme.selection));
            assert_eq!(cell.bg, None);
        }
        assert_eq!(buf.get(1, 1).fg, Some(theme.commit_hash));
        assert_eq!(buf.get(4, 1).modifier, Modifier::empty());
        assert_eq!(buf.get(1, 0).modifier, Modifier::empty());
        assert_eq!(buf.get(0, 0).symbol, " ");
    }

    #[test]
    fn selection_ribbon_is_a_single_narrow_cell() {
        use crate::tui::str_display_width;
        assert_eq!(SELECTION_RIBBON.chars().count(), 1);
        assert_eq!(str_display_width(SELECTION_RIBBON), 1);
    }

    #[test]
    fn mark_selected_row_ignores_empty_gutter_and_rows_outside_it() {
        let theme = Theme::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 3));

        theme.mark_selected_row(&mut buf, Rect::new(0, 0, 0, 3), Rect::new(0, 1, 4, 1));
        theme.mark_selected_row(&mut buf, Rect::new(0, 0, 1, 2), Rect::new(1, 2, 3, 1));

        assert!(buf.cells.iter().all(|c| c.symbol == " "));
    }
}
