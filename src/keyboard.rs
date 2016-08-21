use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use libusb::{Result as UsbResult, Error as UsbError};
use nix::sys::signal::{SigAction, sigaction, SaFlags, SigSet, SigHandler, SIGINT, SIGTERM};
use nix::Result as NixResult;
use handle::{Handle, ControlPacket, ToControlPacket};
use color::*;
use keys::*;
use parser::*;
use event::{GenericHandler, Handler};

pub trait Keyboard {
    fn set_key_colors(&mut self, key_colors: Vec<KeyColor>) -> UsbResult<()>;
    fn set_color(&mut self, key_color: KeyColor) -> UsbResult<()>;
    fn set_all_colors(&mut self, color: Color) -> UsbResult<()>;
    fn set_reconnect_interval(&mut self, interval: Duration);
    fn set_reconnect_attempts(&mut self, attempts: i32);
    fn set_auto_reconnect(&mut self, enabled: bool);
    fn reconnect(&mut self) -> UsbResult<()>;
    unsafe fn enable_signal_handling(&mut self) -> NixResult<()>;
    fn disable_signal_handling(&mut self) -> NixResult<()>;
}

pub struct KeyboardInternal {
    handle: Handle,
    control_packet_queue: VecDeque<ControlPacket>,
    sending_control: bool,
    reconnect_interval: Duration,
    reconnect_attempts: i32,
    auto_reconnect: bool,
}

impl KeyboardInternal {
    pub fn new() -> UsbResult<KeyboardInternal> {
        let handle = try!(Handle::new());
        Ok(KeyboardInternal {
            handle: handle,
            control_packet_queue: VecDeque::new(),
            sending_control: false,
            reconnect_interval: Duration::from_secs(1),
            reconnect_attempts: 10,
            auto_reconnect: true,
        })
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

impl Keyboard for KeyboardInternal {
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

    fn set_reconnect_interval(&mut self, interval: Duration) {
        self.reconnect_interval = interval;
    }

    fn set_reconnect_attempts(&mut self, attempts: i32) {
        self.reconnect_attempts = attempts;
    }
    fn set_auto_reconnect(&mut self, enabled: bool) {
        self.auto_reconnect = enabled;
    }

    fn reconnect(&mut self) -> UsbResult<()> {
        println!("Connection lost. Starting reconnect...");
        let mut last_err = UsbError::NoDevice;
        for _ in 0..self.reconnect_attempts {
            match self.handle.reconnect() {
                Ok(_) => {
                    println!("Reconnected");
                    return Ok(());
                },
                Err(e) => {
                    println!("Reconnecting failed: {:?}", e);
                    last_err = e;
                }
            }
            ::std::thread::sleep(self.reconnect_interval);
        }
        Err(last_err)
    }

    unsafe fn enable_signal_handling(&mut self) -> NixResult<()> {
        let sig_action = SigAction::new(SigHandler::Handler(panic_on_sig), SaFlags::empty(), SigSet::empty());
        try!(sigaction(SIGINT, &sig_action).map(|_| ()));
        sigaction(SIGTERM, &sig_action).map(|_| ())
    }

    fn disable_signal_handling(&mut self) -> NixResult<()> {
        let sig_action = SigAction::new(SigHandler::Handler(exit_on_sig), SaFlags::empty(), SigSet::empty());
        unsafe {
            try!(sigaction(SIGINT, &sig_action).map(|_| ()));
            sigaction(SIGTERM, &sig_action).map(|_| ())
        }
    }
}

extern fn panic_on_sig(_: i32) {
    panic!("Got Sigint: Panicking to drop UsbWrapper to reattach kernel driver");
}

extern fn exit_on_sig(_: i32) {
    ::std::process::exit(1);
}

pub struct KeyboardImpl {
    keyboard_internal: KeyboardInternal,
    parser_index: u32,
    parsers: HashMap<u32, Parser>,
    handler_index: u32,
    handlers: HashMap<u32, Box<GenericHandler>>,
}

impl KeyboardImpl {
    pub fn new() -> UsbResult<KeyboardImpl> {
        let mut keyboard = KeyboardImpl {
            keyboard_internal: try!(KeyboardInternal::new()),
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
        self.handlers.insert(index, handler.into());
        self.handler_index += 1;
        index
    }

    pub fn remove_handler(&mut self, index: u32) -> Option<Box<GenericHandler>> {
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
        let &mut KeyboardImpl {
            ref mut keyboard_internal,
            parser_index: _,
            ref mut parsers,
            handler_index: _,
            ref mut handlers,
        } = self;

        let endpoint_direction;
        let buf;
        loop {
            let timeout = match handlers.iter().filter_map(|(_,h)| h.sleep_duration()).min() {
                Some(d) => d,
                None => Duration::from_secs(3600*24*365)
            };
            let res = keyboard_internal.handle.recv(timeout);
            match res {
                Some(Ok((e, b))) => {
                    endpoint_direction = e;
                    buf = b;
                    break
                },
                Some(Err(err)) => return Err(err),
                None => {
                    for handler in handlers.iter_mut().filter_map(|(_,h)| match h.sleep_duration() {
                        Some(dur) if dur == Duration::from_secs(0) => Some(h),
                        _ => None
                    }) {
                        try!(handler.handle_time(keyboard_internal));
                    }
                }
            }
        }
        let packet = Packet::new(endpoint_direction, &buf);
        let mut handled = false;
        let mut parsed = false;
        for (_, parser) in parsers.iter_mut() {
            match parser {
                &mut Parser::ParseKey(ref mut p) if p.accept(&packet) => {
                    parsed = true;
                    let key_events = try!(p.parse(&packet, keyboard_internal));
                    for key_event in key_events {
                        for (_, handler) in handlers.iter_mut() {
                            if handler.accept_key(&key_event) {
                                handled = true;
                                try!(handler.handle_key(&key_event, keyboard_internal));
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
        // init handlers
        {
            let &mut KeyboardImpl {
                ref mut keyboard_internal,
                parser_index: _,
                parsers: _,
                handler_index: _,
                ref mut handlers,
            } = self;
            for (_, handler) in handlers {
                try!(handler.init(keyboard_internal));
            }
        }
        loop {
            match self.handle() {
                Ok(()) => {},
                Err(UsbError::NoDevice) | Err(UsbError::Io) | Err(UsbError::Busy) => try!(self.reconnect()),
                Err(e) => return Err(e),
            }
        }
    }
}

impl Keyboard for KeyboardImpl {
    fn set_key_colors(&mut self, key_colors: Vec<KeyColor>) -> UsbResult<()> {
        self.keyboard_internal.set_key_colors(key_colors)
    }
    fn set_color(&mut self, key_color: KeyColor) -> UsbResult<()> {
        self.keyboard_internal.set_color(key_color)
    }
    fn set_all_colors(&mut self, color: Color) -> UsbResult<()> {
        self.keyboard_internal.set_all_colors(color)
    }
    fn set_reconnect_interval(&mut self, interval: Duration) {
        self.keyboard_internal.set_reconnect_interval(interval)
    }
    fn set_reconnect_attempts(&mut self, attempts: i32) {
        self.keyboard_internal.set_reconnect_attempts(attempts)
    }
    fn set_auto_reconnect(&mut self, enabled: bool) {
        self.keyboard_internal.set_auto_reconnect(enabled)
    }
    fn reconnect(&mut self) -> UsbResult<()> {
        self.keyboard_internal.reconnect()
    }
    unsafe fn enable_signal_handling(&mut self) -> NixResult<()> {
        self.keyboard_internal.enable_signal_handling()
    }
    fn disable_signal_handling(&mut self) -> NixResult<()> {
        self.keyboard_internal.disable_signal_handling()
    }
}

