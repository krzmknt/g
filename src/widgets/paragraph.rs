use super::{Block, Widget};
use crate::tui::{Buffer, Rect, Style};

#[derive(Debug, Clone)]
pub struct Paragraph<'a> {
    text: &'a str,
    block: Option<Block<'a>>,
    style: Style,
    wrap: bool,
    scroll: (u16, u16),
}

impl<'a> Paragraph<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            block: None,
            style: Style::default(),
            wrap: false,
            scroll: (0, 0),
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

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn scroll(mut self, offset: (u16, u16)) -> Self {
        self.scroll = offset;
        self
    }
}

impl Widget for Paragraph<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Render block if present
        let text_area = if let Some(ref block) = self.block {
            block.render(area, buf);
            block.inner(area)
        } else {
            area
        };

        if text_area.width < 1 || text_area.height < 1 {
            return;
        }

        let lines: Vec<&str> = self.text.lines().collect();
        let (scroll_y, scroll_x) = self.scroll;

        for (i, line) in lines
            .iter()
            .skip(scroll_y as usize)
            .take(text_area.height as usize)
            .enumerate()
        {
            let y = text_area.y + i as u16;
            let content = if scroll_x > 0 && (scroll_x as usize) < line.len() {
                &line[scroll_x as usize..]
            } else {
                *line
            };

            buf.set_string_truncated(text_area.x, y, content, text_area.width, self.style);
        }
    }
}
