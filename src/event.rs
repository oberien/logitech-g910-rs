use keys::*;
use color::*;
use keyboard::Keyboard;
use libusb::Result as UsbResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEvent {
    KeyPressed(Key),
    KeyReleased(Key),
}

pub enum Handler {
    HandleKey(Box<HandleKey>),
}

pub trait HandleKey {
    fn accept(&self, evt: &KeyEvent) -> bool;
    fn handle(&mut self, evt: &KeyEvent, keyboard: &mut Keyboard) -> UsbResult<()>;
}

pub struct FlashHandler;

impl FlashHandler {
    pub fn new() -> FlashHandler {
        FlashHandler { }
    }
}

impl HandleKey for FlashHandler {
    #[allow(unused_variables)]
    fn accept(&self, evt: &KeyEvent) -> bool {
        true
    }

    fn handle(&mut self, evt: &KeyEvent, keyboard: &mut Keyboard) -> UsbResult<()> {
        match evt {
            &KeyEvent::KeyPressed(ref k) => {
                println!("Key pressed: {:?}", k);
                keyboard.set_all_colors(Color::new(255, 0, 0))
            },
            &KeyEvent::KeyReleased(ref k) => {
                println!("Key released: {:?}", k);
                keyboard.set_all_colors(Color::new(0, 0, 255))
            },
        }
    }
}

impl From<FlashHandler> for Handler {
    fn from(handler: FlashHandler) -> Handler {
        Handler::HandleKey(Box::new(handler))
    }
}

