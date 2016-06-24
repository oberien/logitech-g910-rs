use std::time::Duration;
use std::collections::{HashMap, VecDeque};
use std::u8;
use libusb::{DeviceHandle, Result as UsbResult, Context};
use handle::{Handle, ControlPacket};
use color::*;
use keys::*;
use parser::*;
use event::Handler;

pub trait Keyboard {
    fn send_color(&mut self, color_packet: ColorPacket) -> UsbResult<()>;
    fn flush_color(&mut self) -> UsbResult<()>;
    fn set_color(&mut self, color_packet: ColorPacket) -> UsbResult<()>;
    fn set_all_colors(&mut self, color: Color) -> UsbResult<()>;
}

pub struct KeyboardInternal<'a> {
    handle: Handle<'a>,
    control_packet_queue: VecDeque<ControlPacket>,
    sending_control: bool,
}

impl<'a> KeyboardInternal<'a> {
    pub fn new(handle: Handle<'a>) -> KeyboardInternal<'a> {
        KeyboardInternal {
            handle: handle,
            control_packet_queue: VecDeque::new(),
            sending_control: false,
        }
    }

    pub fn queue_control_packet(&mut self, packet: ControlPacket) -> UsbResult<()> {
        self.control_packet_queue.push_back(packet);
        if !self.sending_control {
            self.send_next_control()
        } else {
            Ok(())
        }
    }

    pub fn send_next_control(&mut self) -> UsbResult<()> {
        if self.control_packet_queue.len() == 0 {
            self.sending_control = false;
            return Ok(());
        }
        if !self.sending_control {
            self.sending_control = true;
        }
        self.handle.send_control(self.control_packet_queue.pop_front().unwrap())
    }
}

impl<'a> Keyboard for KeyboardInternal<'a> {
    fn send_color(&mut self, color_packet: ColorPacket) -> UsbResult<()> {
        let packet: [u8; 64] = color_packet.into();
        let mut to_send = Vec::new();
        to_send.extend_from_slice(&packet);
        self.queue_control_packet(ControlPacket::new(0x80, to_send, 0x21, 9, 0x0212,
                                             0x0001, Duration::from_secs(10)))
    }

    fn flush_color(&mut self) -> UsbResult<()> {
        let flush: [u8; 20] = FlushPacket::new().into();
        let mut to_send = Vec::new();
        to_send.extend_from_slice(&flush);
        self.queue_control_packet(ControlPacket::new(0x80, to_send, 0x21, 9, 0x0212,
                                             0x0001, Duration::from_secs(10)))
    }

    fn set_color(&mut self, color_packet: ColorPacket) -> UsbResult<()> {
        try!(self.send_color(color_packet));
        self.flush_color()
    }

    fn set_all_colors(&mut self, color: Color) -> UsbResult<()> {
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
}

pub struct KeyboardImpl<'a> {
    keyboard_internal: KeyboardInternal<'a>,
    parser_index: u32,
    parsers: HashMap<u32, Parser>,
    handler_index: u32,
    handlers: HashMap<u32, Handler>,
}

impl<'a> KeyboardImpl<'a> {
    pub fn new(context: &'a Context, handle: &'a DeviceHandle<'a>) -> UsbResult<KeyboardImpl<'a>> {
        let mut keyboard = KeyboardImpl {
            keyboard_internal: KeyboardInternal::new(try!(Handle::new(context, handle))),
            parser_index: 0,
            parsers: HashMap::new(),
            handler_index: 0,
            handlers: HashMap::new(),
        };
        keyboard.add_parser(KeyParser::new().into());
        keyboard.add_parser(ControlParser::new().into());
        Ok(keyboard)
    }

    pub fn add_handler(&mut self, handler: Handler) -> u32 {
        let index = self.handler_index;
        self.handlers.insert(index, handler);
        self.handler_index += 1;
        index
    }

    pub fn remove_handler(&mut self, index: u32) -> Option<Handler> {
        self.handlers.remove(&index)
    }

    fn add_parser(&mut self, parser: Parser) -> u32 {
        let index = self.parser_index;
        self.parsers.insert(index, parser);
        self.parser_index += 1;
        index
    }

    // FIXME: Currently unused, but maybe needed later
    //fn remove_parser(&mut self, index: u32) -> Option<Parser> {
        //self.parsers.remove(&index)
    //}

    fn handle(&mut self) -> UsbResult<()> {
        let (endpoint_direction, buf) = try!(self.keyboard_internal.handle.recv());
        let packet = Packet::new(endpoint_direction, &buf);
        let mut handled = false;
        let &mut KeyboardImpl {
            ref mut keyboard_internal,
            parser_index: _,
            ref mut parsers,
            handler_index: _,
            ref mut handlers,
        } = self;
        for (_, parser) in parsers.iter_mut() {
            match parser {
                &mut Parser::ParseKey(ref mut p) if p.accept(&packet) => {
                    let key_events = try!(p.parse(&packet, keyboard_internal));
                    for key_event in key_events {
                        for (_, handler) in handlers.iter_mut() {
                            match handler {
                                &mut Handler::HandleKey(ref mut h) if h.accept(&key_event) => {
                                    handled = true;
                                    try!(h.handle(&key_event, keyboard_internal));
                                },
                                _ => {}
                            }
                        }
                    }
                },
                &mut Parser::ParseControl(ref mut p) if p.accept(&packet) => {
                    handled = true;
                    try!(p.parse(&packet, keyboard_internal));
                },
                _ => {}
            }
        }
        if !handled {
            println!("Packet not handled: {:?}", packet);
        }
        Ok(())
    }

    pub fn start_handle_loop(&mut self) -> UsbResult<()> {
        loop {
            try!(self.handle());
        }
    }
}

impl<'a> Keyboard for KeyboardImpl<'a> {
    fn send_color(&mut self, color_packet: ColorPacket) -> UsbResult<()> {
        self.keyboard_internal.send_color(color_packet)
    }
    fn flush_color(&mut self) -> UsbResult<()> {
        self.keyboard_internal.flush_color()
    }
    fn set_color(&mut self, color_packet: ColorPacket) -> UsbResult<()> {
        self.keyboard_internal.set_color(color_packet)
    }
    fn set_all_colors(&mut self, color: Color) -> UsbResult<()> {
        self.keyboard_internal.set_all_colors(color)
    }
}

