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
    pressed_keys: HashSet<Key>,
    pressed_rollover_keys: HashSet<Key>,
    pressed_media_keys: HashSet<Key>,
}

impl KeyParser {
    pub fn new() -> KeyParser {
        KeyParser {
            pressed_keys: HashSet::new(),
            pressed_rollover_keys: HashSet::new(),
            pressed_media_keys: HashSet::new(),
        }
    }
}

impl ParseKey for KeyParser {
    fn accept(&self, packet: &Packet) -> bool {
        // normal key
        packet.buf.len() == 8 && packet.endpoint == 1 && packet.buf[1] == 0x00
        // rollover
        || packet.buf.len() == 21 && packet.endpoint == 2 && packet.buf[0] == 0x01
        //  media key
        || packet.buf.len() == 2 && packet.endpoint == 2 && packet.buf[0] == 0x02
    }

    #[allow(unused_variables)]
    fn parse(&mut self, packet: &Packet, keyboard_internal: &mut KeyboardInternal) -> UsbResult<Vec<KeyEvent>> {
        let mut state = HashSet::new();

        let media = packet.endpoint == 2 && packet.buf[0] == 0x02;
        let standard = packet.endpoint == 1;
        let rollover = packet.endpoint == 2 && packet.buf[0] == 0x01;

        // media keys
        if media {
            for key in MediaKey::values() {
                if packet.buf[1] & key as u8 == key as u8 {
                    state.insert(key.into());
                }
            }
        }

        // standard keys and rollover
        if standard || rollover {
            for k in &packet.buf[1..] {
                match StandardKey::from(*k) {
                    StandardKey::None => {},
                    s => { state.insert(s.into()); }
                }
            }
        }

        // modifier keys
        if standard {
            // byte 0 has modifier keys as bytemap:
            // 0b0 0 0 0 0 0 0 0
            //   R R R R L L L L
            //   W A S C W A S C
            //   I L H T I L H T
            //   N T I R N T I R
            //     G F L     F L
            //     R T       T
            if packet.buf[0] & 0x01 == 0x01 {
                state.insert(StandardKey::LeftControl.into());
            }
            if packet.buf[0] & 0x02 == 0x02 {
                state.insert(StandardKey::LeftShift.into());
            }
            if packet.buf[0] & 0x04 == 0x04 {
                state.insert(StandardKey::LeftAlt.into());
            }
            if packet.buf[0] & 0x08 == 0x08 {
                state.insert(StandardKey::LeftWindows.into());
            }
            if packet.buf[0] & 0x10 == 0x10 {
                state.insert(StandardKey::RightControl.into());
            }
            if packet.buf[0] & 0x20 == 0x20 {
                state.insert(StandardKey::RightShift.into());
            }
            if packet.buf[0] & 0x40 == 0x40 {
                state.insert(StandardKey::RightAlt.into());
            }
            if packet.buf[0] & 0x80 == 0x80 {
                state.insert(StandardKey::RightWindows.into());
            }
        }

        let old_state = if standard {
            &mut self.pressed_keys
        } else if rollover {
            &mut self.pressed_rollover_keys
        } else if media {
            &mut self.pressed_media_keys
        } else {
            unreachable!()
        };
        let mut added: Vec<_>;
        let mut removed: Vec<_>;
        added = state.difference(old_state).cloned().collect();
        removed = old_state.difference(&state).cloned().collect();
        *old_state = state;
        let res = added.drain(..).map(|e| KeyEvent::KeyPressed(e))
            .chain(removed.drain(..).map(|e| KeyEvent::KeyReleased(e)))
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
        (packet.buf.len() == 20 || packet.buf.len() == 64) && packet.endpoint == 0
        || packet.buf.len() == 20 && packet.endpoint == 2 && packet.buf[0] == 0x11
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
        // wait for the acknoledgement of the control packet on iface 2 before
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

