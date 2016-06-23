use std::time::Duration;
use std::collections::HashSet;
use libusb::{DeviceHandle, Result as UsbResult, Context};
use std::u8;
use color::*;
use keys::*;
use handle::Handle;

pub struct Keyboard<'a> {
    handle: Handle<'a>,
    pressed_keys1: HashSet<Key>,
    pressed_keys2: HashSet<Key>,
}

impl<'a> Keyboard<'a> {
    pub fn new(context: &'a Context, handle: &'a DeviceHandle<'a>) -> UsbResult<Keyboard<'a>> {
        Ok(Keyboard {
            handle: try!(Handle::new(context, handle)),
            pressed_keys1: HashSet::new(),
            pressed_keys2: HashSet::new(),
        })
    }

    pub fn send_color(&mut self, color_packet: ColorPacket) -> UsbResult<()> {
        //try!(self.handle.listen_iface2(Duration::from_secs(1)));

        let packet: [u8; 64] = color_packet.into();
        let mut to_send = Vec::new();
        to_send.extend_from_slice(&packet);
        try!(self.handle.send_control(0x80, to_send, 0x21, 9, 0x0212, 0x0001, Duration::from_secs(10)));
        match self.handle.recv() {
            Ok(buf) => println!("OK: {:?}", &buf),
            Err(e) => println!("Err: {}", e)
        }
        match self.handle.recv() {
            Ok(buf) => println!("OK: {:?}", &buf),
            Err(e) => println!("Err: {}", e)
        }
        Ok(())
    }

    pub fn flush_color(&mut self) -> UsbResult<()> {
        //try!(self.handle.listen_iface2(Duration::from_secs(1)));

        let flush: [u8; 20] = FlushPacket::new().into();
        let mut to_send = Vec::new();
        to_send.extend_from_slice(&flush);
        try!(self.handle.send_control(0x80, to_send, 0x21, 9, 0x0212, 0x0001, Duration::from_secs(10)));
        match self.handle.recv() {
            Ok(buf) => println!("OK: {:?}", &buf),
            Err(e) => println!("Err: {}", e)
        }
        match self.handle.recv() {
            Ok(buf) => println!("OK: {:?}", &buf),
            Err(e) => println!("Err: {}", e)
        }
        Ok(())
    }

    pub fn set_color(&mut self, color_packet: ColorPacket) -> UsbResult<()> {
        try!(self.send_color(color_packet));
        try!(self.flush_color());
        Ok(())
    }

    pub fn set_all_colors(&mut self, color: Color) -> UsbResult<()> {
        for chunk in (&StandardKey::values()[..]).chunks(14) {
            let mut packet = ColorPacket::new();
            for code in chunk {
                packet.add_key_color(KeyColor::new(*code, color)).unwrap();
            }
            try!(self.send_color(packet));
        }
        for chunk in (&GamingKey::values()[..]).chunks(14) {
            let mut packet = ColorPacket::new();
            for code in chunk {
                packet.add_key_color(KeyColor::new(*code, color)).unwrap();
            }
            try!(self.send_color(packet));
        }
        for chunk in (&Logo::values()[..]).chunks(14) {
            let mut packet = ColorPacket::new();
            for code in chunk {
                packet.add_key_color(KeyColor::new(*code, color)).unwrap();
            }
            try!(self.send_color(packet));
        }
        self.flush_color()
    }

    pub fn parse_buf(&mut self, buf: &[u8]) -> Result<KeyEvent, ()> {
        // iface1, normal key || iface2, rollover
        if buf[0] != 0x00 && buf[0] != 0x01 {
            return Err(());
        }
        let mut state = HashSet::new();
        for k in &buf[1..] {
            match StandardKey::from(*k) {
                StandardKey::None => {},
                s => { state.insert(s.into()); }
            }
        }

        let mut added: Vec<_>;
        let mut removed: Vec<_>;
        if buf[0] == 0x00 {
            added = state.difference(&self.pressed_keys1).cloned().collect();
            removed = self.pressed_keys1.difference(&state).cloned().collect();
            self.pressed_keys1 = state;
        } else {
            added = state.difference(&self.pressed_keys2).cloned().collect();
            removed = self.pressed_keys2.difference(&state).cloned().collect();
            self.pressed_keys2 = state;
        }
        assert!(1 == added.len() + removed.len());

        if added.len() == 1 {
            Ok(KeyEvent::KeyPressed(added.pop().unwrap()))
        } else {
            Ok(KeyEvent::KeyReleased(removed.pop().unwrap()))
        }
    }

    pub fn handle<F>(&mut self, mut f: F) -> UsbResult<()>
            where F: FnMut(KeyEvent, &mut Keyboard) -> bool {
        loop {
            let buf = try!(self.handle.recv());
            println!("buf: {:?}", buf);
            // TODO: fix unwrap as race conditions between setting colors and reading keys could
            // happen
            if !f(self.parse_buf(&buf).unwrap(), self) {
                break;
            }
        }
        Ok(())
    }
}

