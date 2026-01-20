use crate::tui::{Buffer, Rect, Style};
use super::{Widget, StatefulWidget, Block};

#[derive(Debug, Clone)]
pub struct ListItem<'a> {
    pub content: &'a str,
    pub style: Style,
}

impl<'a> ListItem<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub offset: usize,
    pub selected: Option<usize>,
}

impl ListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
}

#[derive(Debug, Clone)]
pub struct List<'a> {
    items: Vec<ListItem<'a>>,
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: Option<&'a str>,
}

impl<'a> List<'a> {
    pub fn new<I>(items: I) -> Self
    where
        I: IntoIterator<Item = ListItem<'a>>,
    {
        Self {
            items: items.into_iter().collect(),
            block: None,
            style: Style::default(),
            highlight_style: Style::default(),
            highlight_symbol: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = Some(symbol);
        self
    }
}

impl StatefulWidget for List<'_> {
    type State = ListState;

    fn render(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render block if present
        let list_area = if let Some(ref block) = self.block {
            block.render(area, buf);
            block.inner(area)
        } else {
            area
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        // Adjust offset to ensure selected item is visible
        if let Some(selected) = state.selected {
            let height = list_area.height as usize;
            if selected < state.offset {
                state.offset = selected;
            } else if selected >= state.offset + height {
                state.offset = selected - height + 1;
            }
        }

        let highlight_symbol_width = self.highlight_symbol.map(|s| s.chars().count()).unwrap_or(0);

        // Render items
        for (i, item) in self.items.iter().skip(state.offset).take(list_area.height as usize).enumerate() {
            let y = list_area.y + i as u16;
            let is_selected = state.selected == Some(state.offset + i);

            let style = if is_selected {
                self.highlight_style
            } else {
                item.style
            };

            // Clear the line with the style
            buf.set_style(Rect::new(list_area.x, y, list_area.width, 1), style);

            // Draw highlight symbol
            let content_x = if let Some(symbol) = self.highlight_symbol {
                if is_selected {
                    buf.set_string(list_area.x, y, symbol, style);
                }
                list_area.x + highlight_symbol_width as u16
            } else {
                list_area.x
            };

            // Draw content
            let max_width = list_area.width.saturating_sub(highlight_symbol_width as u16);
            buf.set_string_truncated(content_x, y, item.content, max_width, style);
        }
    }
}
