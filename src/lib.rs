//#![warn(missing_docs)]

extern crate libusb;
extern crate byteorder;

pub use color::{Color, KeyColor};
pub use keys::{Key, KeyType, StandardKey, MediaKey, GamingKey, Logo};
pub use utils::UsbWrapper;
pub use keyboard::{Keyboard, KeyboardImpl};
pub use event::{KeyEvent, HandlerBuilder, Handler};

mod consts;
mod color;
mod keys;
mod utils;
mod handle;
mod keyboard;
mod parser;
mod event;
