mod event;
mod reader;

pub use event::{Event, KeyEvent, KeyCode, Modifiers, MouseEvent, MouseEventKind, MouseButton};
pub use reader::EventReader;
