//#![warn(missing_docs)]

extern crate libusb;
extern crate byteorder;

pub use color::{Color, KeyColor, KeyColorError, ColorPacket, FlushPacket};
pub use keys::{KeyEvent, Key, KeyType, StandardKey, GamingKey, Logo};
pub use utils::{get_context, get_handle};
pub use keyboard::Keyboard;

mod consts;
mod color;
mod keys;
mod utils;
mod handle;
mod keyboard;
