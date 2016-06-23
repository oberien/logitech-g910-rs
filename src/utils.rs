use std::borrow::Borrow;
use std::ops::Deref;
use libusb::{
    LogLevel,
    Context,
    DeviceHandle,
    Result as UsbResult,
    Error,
};

use consts;

pub struct DeviceHandleWrapper<'a> {
    handle: DeviceHandle<'a>,
    has_kernel_driver0: bool,
    has_kernel_driver1: bool,
}

impl<'a> Drop for DeviceHandleWrapper<'a> {
    fn drop(&mut self) {
        self.handle.release_interface(1).unwrap();
        self.handle.release_interface(0).unwrap();
        if self.has_kernel_driver1 {
            self.handle.attach_kernel_driver(1).unwrap();
        }
        if self.has_kernel_driver0 {
            self.handle.attach_kernel_driver(0).unwrap();
        }
    }
}

impl<'a> Borrow<DeviceHandle<'a>> for DeviceHandleWrapper<'a> {
    fn borrow(&self) -> &DeviceHandle<'a> {
        &self.handle
    }
}

impl<'a> Deref for DeviceHandleWrapper<'a> {
    type Target = DeviceHandle<'a>;
    fn deref(&self) -> &DeviceHandle<'a> {
        &self.handle
    }
}

pub fn get_context() -> Context {
    let mut context = match Context::new() {
        Ok(c) => c,
        Err(e) => panic!("Context::new(): {}", e)
    };
    context.set_log_level(LogLevel::Debug);
    context.set_log_level(LogLevel::Info);
    context.set_log_level(LogLevel::Warning);
    context.set_log_level(LogLevel::Error);
    context.set_log_level(LogLevel::None);
    return context;
}

pub fn get_handle<'a>(context: &'a Context) -> UsbResult<DeviceHandleWrapper<'a>> {
    let devices = match context.devices() {
        Ok(devices) => devices,
        Err(e) => return Err(e),
    };
    for d in devices.iter() {
        let dd = match d.device_descriptor() {
            Ok(dd) => dd,
            Err(_) => continue
        };
        if dd.vendor_id() == consts::VENDOR_ID && dd.product_id() == consts::PRODUCT_ID {
            let mut handle = try!(d.open());
            // for some reason we cannot claim interface 2 as it doesn't exist
            // but we will be able to read from it, if we claim interface 1
            // detch kernel driver
            let has_kernel_driver0 = try!(detach(&mut handle, 0));
            let has_kernel_driver1 = try!(detach(&mut handle, 1));
            // claim interfaces
            try!(handle.claim_interface(0));
            try!(handle.claim_interface(1));
            // reset keyboard to get clean status
            try!(handle.reset());
            return Ok(DeviceHandleWrapper {
                handle: handle,
                has_kernel_driver0: has_kernel_driver0,
                has_kernel_driver1: has_kernel_driver1,
            });
        }
    }
    return Err(Error::NoDevice);
}

fn detach(handle: &mut DeviceHandle, iface: u8) -> UsbResult<bool> {
    match handle.kernel_driver_active(iface) {
        Ok(true) => {
            try!(handle.detach_kernel_driver(iface));
            Ok(true)
        },
        _ => Ok(false)
    }

}

