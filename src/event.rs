use keys::Key;
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
    #[allow(unused_variables)]
    fn init(&mut self, keyboard: &mut Keyboard) -> UsbResult<()> {
        Ok(())
    }
    fn accept(&self, evt: &KeyEvent) -> bool;
    fn handle(&mut self, evt: &KeyEvent, keyboard: &mut Keyboard) -> UsbResult<()>;
}

