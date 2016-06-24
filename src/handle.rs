use libusb::{Context, DeviceHandle, AsyncGroup, Transfer, Result as UsbResult};

use std::time::Duration;

pub struct Handle<'a> {
    handle: &'a DeviceHandle<'a>,
    async_group: AsyncGroup<'a>,
}

pub struct ControlPacket {
    endpoint_direction: u8,
    buf: Vec<u8>,
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    timeout: Duration,
}

impl ControlPacket {
    pub fn new(endpoint_direction: u8, buf: Vec<u8>, request_type: u8, request: u8,
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

impl<'a> Handle<'a> {
    pub fn new(context: &'a Context, handle: &'a DeviceHandle<'a>) -> UsbResult<Handle<'a>> {
        let mut handle = Handle {
            handle: handle,
            async_group: AsyncGroup::new(&context),
        };
        try!(handle.listen_iface1(Duration::from_secs(3600*24)));
        try!(handle.listen_iface2(Duration::from_secs(3600*24)));
        Ok(handle)
    }

    pub fn send_control(&mut self, packet: ControlPacket) ->  UsbResult<()> {
        self.async_group.submit(Transfer::control(
                self.handle,
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
        self.async_group.submit(Transfer::interrupt(
                self.handle,
                endpoint_direction,
                buf,
                timeout
        ))
    }

    pub fn recv(&mut self) -> UsbResult<(u8, Vec<u8>)> {
        let mut transfer = try!(self.async_group.wait_any());
        let buf = transfer.actual().iter().cloned().collect();
        let endpoint_direction = transfer.endpoint();
        // don't resubmit control packets
        if endpoint_direction != 0x80 {
            try!(self.async_group.submit(transfer));
        }
        Ok((endpoint_direction, buf))
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

