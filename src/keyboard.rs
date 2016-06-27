use std::collections::{HashMap, VecDeque};
use libusb::{Context, DeviceHandle, Result as UsbResult, Error as UsbError};
use handle::{Handle, ControlPacket, ToControlPacket};
use color::*;
use keys::*;
use parser::*;
use event::Handler;

pub trait Keyboard {
    fn set_key_colors(&mut self, key_colors: Vec<KeyColor>) -> UsbResult<()>;
    fn set_color(&mut self, key_color: KeyColor) -> UsbResult<()>;
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

    fn send_color<T: KeyType>(&mut self, color_packet: ColorPacket<T>) -> UsbResult<()> {
        self.queue_control_packet(color_packet.to_control_packet())
    }

    fn flush_color(&mut self) -> UsbResult<()> {
        self.queue_control_packet(FlushPacket::new().to_control_packet())
    }
}

impl<'a> Keyboard for KeyboardInternal<'a> {
    fn set_key_colors(&mut self, key_colors: Vec<KeyColor>) -> UsbResult<()> {
        let mut standard_packet = ColorPacket::new();
        let mut gaming_packet = ColorPacket::new();
        let mut logo_packet = ColorPacket::new();


        for key_color in key_colors {
            match key_color.key {
                Key::Standard(s) => {
                    match standard_packet.add(s, key_color.color) {
                        Some(p) => try!(self.send_color(p)),
                        None => {}
                    }
                },
                Key::Gaming(g) => {
                    match gaming_packet.add(g, key_color.color) {
                        Some(p) => try!(self.send_color(p)),
                        None => {}
                    }
                },
                Key::Logo(l) => {
                    match logo_packet.add(l, key_color.color) {
                        Some(p) => try!(self.send_color(p)),
                        None => {}
                    }
                },
                Key::Media(_) => return Err(UsbError::InvalidParam)
            }
        }
        if standard_packet.len() > 0 {
            try!(self.send_color(standard_packet));
        }
        if gaming_packet.len() > 0 {
            try!(self.send_color(gaming_packet));
        }
        if logo_packet.len() > 0 {
            try!(self.send_color(logo_packet));
        }
        self.flush_color()
    }

    fn set_color(&mut self, key_color: KeyColor) -> UsbResult<()> {
        let key_colors = vec![key_color];
        self.set_key_colors(key_colors)
    }

    fn set_all_colors(&mut self, color: Color) -> UsbResult<()> {
        let mut values = Key::values();
        let key_colors = values.drain(..)
            .filter(|k| match k {
                // we can't set the color of media keys
                &Key::Media(_) => false,
                _ => true
            }).map(|k| KeyColor::new(k, color.clone()))
            .collect();
        self.set_key_colors(key_colors)
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

    pub fn add_handler(&mut self, mut handler: Handler) -> UsbResult<u32> {
        match &mut handler {
            &mut Handler::HandleKey(ref mut handler) => try!(handler.init(self)),
        }
        let index = self.handler_index;
        self.handlers.insert(index, handler);
        self.handler_index += 1;
        Ok(index)
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
        let mut parsed = false;
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
                    parsed = true;
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
                    parsed = true;
                    handled = true;
                    try!(p.parse(&packet, keyboard_internal));
                },
                _ => {}
            }
        }
        if !parsed {
            println!("Packet not parsed: {:?}", packet);
        } else if !handled {
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
    fn set_key_colors(&mut self, key_colors: Vec<KeyColor>) -> UsbResult<()> {
        self.keyboard_internal.set_key_colors(key_colors)
    }
    fn set_color(&mut self, key_color: KeyColor) -> UsbResult<()> {
        self.keyboard_internal.set_color(key_color)
    }
    fn set_all_colors(&mut self, color: Color) -> UsbResult<()> {
        self.keyboard_internal.set_all_colors(color)
    }
}

