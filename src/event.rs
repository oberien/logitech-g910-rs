use std::time::Duration;
use keys::Key;
use keyboard::Keyboard;
use libusb::Result as UsbResult;

pub struct Handler(Box<GenericHandler>);

impl From<Handler> for Box<GenericHandler> {
    fn from(handler: Handler) -> Box<GenericHandler> {
        handler.0
    }
}

pub trait GenericHandler {
    fn init(&mut self, &mut Keyboard) -> UsbResult<()>;
    fn accept_key(&self, &KeyEvent) -> bool;
    fn handle_key(&mut self, &KeyEvent, &mut Keyboard) -> UsbResult<()>;
    fn handle_time(&mut self, Duration, &mut Keyboard) -> UsbResult<()>;
    fn sleep_duration(&self) -> Option<Duration>;
}

pub struct HandlerBuilder<T: Sized> {
    user_data: T,
    init_fn: Option<Box<Fn(&mut T, &mut Keyboard) -> UsbResult<()>>>,
    accept_key_fn: Option<Box<Fn(&T, &KeyEvent) -> bool>>,
    handle_key_fn: Option<Box<Fn(&mut T, &KeyEvent, &mut Keyboard) -> UsbResult<()>>>,
    handle_time_fn: Option<(Box<Fn(&mut T, Duration, &mut Keyboard) -> UsbResult<()>>, Duration)>,
}

impl<T: 'static + Sized> HandlerBuilder<T> {
    pub fn new(user_data: T) -> HandlerBuilder<T> {
        HandlerBuilder {
            user_data: user_data,
            init_fn: None,
            accept_key_fn: None,
            handle_key_fn: None,
            handle_time_fn: None,
        }
    }

    pub fn init_fn<F>(mut self, f: F) -> Self
            where F: 'static + Fn(&mut T, &mut Keyboard) -> UsbResult<()> {
        self.init_fn = Some(Box::new(f));
        self
    }
    pub fn accept_key_fn<F>(mut self, f: F) -> Self
            where F: 'static + Fn(&T, &KeyEvent) -> bool {
        self.accept_key_fn = Some(Box::new(f));
        self
    }
    pub fn handle_key_fn<F>(mut self, f: F) -> Self
            where F: 'static + Fn(&mut T, &KeyEvent, &mut Keyboard) -> UsbResult<()> {
        self.handle_key_fn = Some(Box::new(f));
        self
    }
    pub fn handle_time_fn<F>(mut self, f: F, sleep_duration: Duration) -> Self
            where F: 'static + Fn(&mut T, Duration, &mut Keyboard) -> UsbResult<()> {
        self.handle_time_fn = Some((Box::new(f), sleep_duration));
        self
    }
    pub fn build(self) -> Handler{
        Handler(Box::new(self))
    }
}

impl<T: Sized> GenericHandler for HandlerBuilder<T> {
    fn init(&mut self, keyboard: &mut Keyboard) -> UsbResult<()> {
        match &self.init_fn {
            &Some(ref f) => f(&mut self.user_data, keyboard),
            &None => Ok(())
        }
    }
    fn accept_key(&self, evt: &KeyEvent) -> bool {
        match &self.accept_key_fn {
            &Some(ref f) => f(&self.user_data, evt),
            &None => false
        }
    }
    fn handle_key(&mut self, evt: &KeyEvent, keyboard: &mut Keyboard) -> UsbResult<()> {
        match &self.handle_key_fn {
            &Some(ref f) => f(&mut self.user_data, evt, keyboard),
            &None => Ok(())
        }
    }
    fn handle_time(&mut self, delta: Duration, keyboard: &mut Keyboard) -> UsbResult<()> {
        match &self.handle_time_fn {
            &Some((ref f, _)) => f(&mut self.user_data, delta, keyboard),
            &None => Ok(())
        }
    }
    fn sleep_duration(&self) -> Option<Duration> {
        self.handle_time_fn.as_ref().map(|s| s.1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEvent {
    KeyPressed(Key),
    KeyReleased(Key),
}

