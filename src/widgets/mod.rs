mod block;
mod list;
mod paragraph;
mod scrollbar;

pub use block::{Block, Borders};
pub use list::{List, ListItem, ListState};
pub use paragraph::Paragraph;
pub use scrollbar::Scrollbar;

use crate::tui::{Buffer, Rect};

pub trait Widget {
    fn render(&self, area: Rect, buf: &mut Buffer);
}

pub trait StatefulWidget {
    type State;
    fn render(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
