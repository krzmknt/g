mod event;
mod reader;

pub use event::{Event, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind};
pub use reader::EventReader;
