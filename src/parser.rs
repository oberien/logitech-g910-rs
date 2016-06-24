use std::collections::HashSet;
use keys::*;
use event::KeyEvent;
use keyboard::KeyboardInternal;
use libusb::{Result as UsbResult, Error as UsbError};

#[derive(Debug, Clone, PartialEq)]
pub struct Packet<'a> {
    pub endpoint: u8,
    pub buf: &'a [u8],
}

impl<'a> Packet<'a> {
    pub fn new(endpoint_direction: u8, buf: &'a [u8]) -> Packet {
        Packet {
            endpoint: endpoint_direction & 0x7f,
            buf: buf
        }
    }
}

pub enum Parser {
    ParseKey(Box<ParseKey>),
    ParseControl(Box<ParseControl>),
}

pub trait ParseKey {
    fn accept(&self, packet: &Packet) -> bool;
    fn parse(&mut self, packet: &Packet, keyboard_internal: &mut KeyboardInternal) -> UsbResult<Vec<KeyEvent>>;
}

pub trait ParseControl {
    fn accept(&self, packet: &Packet) -> bool;
    fn parse(&mut self, packet: &Packet, keyboard_internal: &mut KeyboardInternal) -> UsbResult<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyParser {
    pressed_keys1: HashSet<Key>,
    pressed_keys2: HashSet<Key>,
}

impl KeyParser {
    pub fn new() -> KeyParser {
        KeyParser {
            pressed_keys1: HashSet::new(),
            pressed_keys2: HashSet::new(),
        }
    }
}

impl ParseKey for KeyParser {
    fn accept(&self, packet: &Packet) -> bool {
        // interface 1 - normal key || interface 2 - rollover
        packet.buf.len() >= 8
            && packet.endpoint == 1
            || (packet.endpoint == 2 && packet.buf[0] == 0x01)
    }

    #[allow(unused_variables)]
    fn parse(&mut self, packet: &Packet, keyboard_internal: &mut KeyboardInternal) -> UsbResult<Vec<KeyEvent>> {
        let mut state = HashSet::new();
        // TODO: add modifier keys
        for k in &packet.buf[1..] {
            match StandardKey::from(*k) {
                StandardKey::None => {},
                s => { state.insert(s.into()); }
            }
        }

        let mut added: Vec<_>;
        let mut removed: Vec<_>;
        if packet.endpoint == 1 {
            added = state.difference(&self.pressed_keys1).cloned().collect();
            removed = self.pressed_keys1.difference(&state).cloned().collect();
            self.pressed_keys1 = state;
        } else {
            added = state.difference(&self.pressed_keys2).cloned().collect();
            removed = self.pressed_keys2.difference(&state).cloned().collect();
            self.pressed_keys2 = state;
        }
        let added_len = added.len();
        let removed_len = removed.len();
        let res = added.drain(0..added_len).map(|e| KeyEvent::KeyPressed(e))
            .chain(removed.drain(0..removed_len).map(|e| KeyEvent::KeyReleased(e)))
            .collect();
        Ok(res)
    }
}

impl From<KeyParser> for Parser {
    fn from(parser: KeyParser) -> Parser {
        Parser::ParseKey(Box::new(parser))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlParser;

impl ControlParser {
    pub fn new() -> ControlParser {
        ControlParser { }
    }
}

impl ParseControl for ControlParser {
    fn accept(&self, packet: &Packet) -> bool {
        packet.endpoint == 0 || (packet.endpoint == 2 && packet.buf[0] == 0x11)
    }

    fn parse(&mut self, packet: &Packet, keyboard_internal: &mut KeyboardInternal) -> UsbResult<()> {
        if packet.buf.len() == 0 {
            println!("buf empty");
            Ok(())
        } else if packet.endpoint == 0
             && !(packet.buf[0] == 0x11 || packet.buf[0] == 0x12) {
            println!("Trying to parse unknown packet from iface 0: {:?}", packet);
            Err(UsbError::NotSupported)
        } else if packet.endpoint == 2
            && !(packet.buf[0] == 0x11) {
            println!("Trying to parse unknown packet from iface 2: {:?}", packet);
            Err(UsbError::NotSupported)
        // wait for the acknoledgement of the control packet before
        // sending the next one
        } else if packet.endpoint == 2 {
            keyboard_internal.send_next_control()
        } else {
            Ok(())
        }
    }
}

impl From<ControlParser> for Parser {
    fn from(parser: ControlParser) -> Parser {
        Parser::ParseControl(Box::new(parser))
    }
}

