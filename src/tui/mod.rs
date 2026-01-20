mod terminal;
mod buffer;
mod style;
mod render;

pub use terminal::Terminal;
pub use buffer::{Buffer, Cell};
pub use style::{Style, Color, Modifier};
pub use render::Rect;
