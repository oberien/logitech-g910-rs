use std::time::Duration;
use libusb::{Transfer, Result as UsbResult};
use utils::UsbWrapper;

pub trait ToControlPacket {
    fn to_control_packet(self) -> ControlPacket;
}

pub struct ControlPacket {
    buf: Vec<u8>,
    endpoint_direction: u8,
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    timeout: Duration,
}

impl ControlPacket {
    pub fn new(buf: Vec<u8>, endpoint_direction: u8, request_type: u8, request: u8,
               value:u16, index: u16, timeout: Duration) -> ControlPacket {
        ControlPacket {
            endpoint_direction: endpoint_direction,
            buf: buf,
            request_type: request_type,
            request: request,
            value: value,
            index: index,
            timeout: timeout,
        }
    }
}

pub struct Handle {
    usb_wrapper: Option<UsbWrapper>,
}

impl Handle {
    pub fn new() -> UsbResult<Handle> {
        let usb_wrapper = try!(UsbWrapper::new());
        let mut handle = Handle {
            usb_wrapper: Some(usb_wrapper),
        };
        try!(handle.listen_iface1(Duration::from_secs(3600*24*365)));
        try!(handle.listen_iface2(Duration::from_secs(3600*24*365)));
        Ok(handle)
    }

    pub fn reconnect(&mut self) -> UsbResult<()> {
        // We must drop the old one before creating a new one, because all
        // handles and locks on that device must be released first.
        drop(::std::mem::replace(&mut self.usb_wrapper, None));
        self.usb_wrapper = Some(try!(UsbWrapper::new()));
        Ok(())
    }

    pub fn send_control(&mut self, packet: ControlPacket) ->  UsbResult<()> {
        let wrapper_ref = self.usb_wrapper.as_mut().unwrap();
        wrapper_ref.async_group.submit(Transfer::control(
                wrapper_ref.handle,
                packet.endpoint_direction,
                packet.buf,
                packet.request_type,
                packet.request,
                packet.value,
                packet.index,
                packet.timeout
        ))
    }

    pub fn send_interrupt(&mut self, endpoint_direction: u8, buf: Vec<u8>,
                          timeout: Duration) -> UsbResult<()> {
        let wrapper_ref = self.usb_wrapper.as_mut().unwrap();
        wrapper_ref.async_group.submit(Transfer::interrupt(
                wrapper_ref.handle,
                endpoint_direction,
                buf,
                timeout
        ))
    }

    pub fn recv(&mut self, timeout: Duration) -> Option<UsbResult<(u8, Vec<u8>)>> {
        let mut transfer = match self.usb_wrapper.as_mut().unwrap().async_group.try_wait_any(timeout) {
            Some(res) => match res {
                Ok(transfer) => transfer,
                Err(err) => return Some(Err(err))
            },
            None => return None
        };
        let buf = transfer.actual().iter().cloned().collect();
        let endpoint_direction = transfer.endpoint();
        // don't resubmit control packets
        if endpoint_direction != 0x80 {
            match self.usb_wrapper.as_mut().unwrap().async_group.submit(transfer) {
                Ok(_) => {},
                Err(err) => return Some(Err(err))
            }
        }
        Some(Ok((endpoint_direction, buf)))
    }

    pub fn listen_iface2(&mut self, timeout: Duration) -> UsbResult<()> {
        let mut vec = Vec::new();
        vec.resize(64, 0u8);
        self.send_interrupt(0x82, vec, timeout)
    }

    pub fn listen_iface1(&mut self, timeout: Duration) -> UsbResult<()> {
        let mut vec = Vec::new();
        vec.resize(8, 0u8);
        try!(self.send_interrupt(0x81, vec, timeout));
        Ok(())
    }
}

