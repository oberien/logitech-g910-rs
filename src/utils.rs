use libusb::{
    LogLevel,
    Context,
    DeviceHandle,
    AsyncGroup,
    Result as UsbResult,
    Error,
};

use consts;

pub struct UsbWrapper {
    context: &'static Context,
    pub handle: &'static DeviceHandle<'static>,
    has_kernel_driver0: bool,
    has_kernel_driver1: bool,
    pub async_group: &'static mut AsyncGroup<'static>,
}

impl UsbWrapper {
    pub fn new() -> UsbResult<UsbWrapper> {
        // We must leak both context and handle and async_group, as rust does not allow sibling structs.
        // Leaking them gives us a &'static reference, which we can then use without
        // lifetime bounds, as it outlives everything.
        // We must make sure though, that the leaked memory is freed afterwards,
        // which is done in Drop.
        let context = try!(get_context());
        let context_ptr = Box::into_raw(Box::new(context));
        let context_ref = unsafe { &*context_ptr as &'static Context };
        let (handle, driver0, driver1) = try!(get_handle(context_ref));
        let async_group = AsyncGroup::new(context_ref);
        let handle_ptr = Box::into_raw(Box::new(handle));
        let async_ptr = Box::into_raw(Box::new(async_group));
        unsafe {
            Ok(UsbWrapper {
                context: context_ref,
                handle: &mut *handle_ptr as &'static mut DeviceHandle<'static>,
                has_kernel_driver0: driver0,
                has_kernel_driver1: driver1,
                async_group: &mut *async_ptr as &'static mut AsyncGroup<'static>,
            })
        }
    }
}

macro_rules! unwrap_safe {
    ($e:expr) => {
        match $e {
            Ok(_) => {},
            Err(e) => println!("Error while dropping UsbWrapper during another panic: {:?}", e),
        }
    }
}

impl Drop for UsbWrapper {
    fn drop(&mut self) {
        // make sure handle_mut is dropped before dropping it's refering content
        // this assures that there will be no dangling pointers
        {
            let handle_mut = unsafe { &mut *(self.handle as *const _ as *mut DeviceHandle<'static>) };
            unwrap_safe!(handle_mut.release_interface(1));
            unwrap_safe!(handle_mut.release_interface(0));
            if self.has_kernel_driver1 {
                unwrap_safe!(handle_mut.attach_kernel_driver(1));
            }
            if self.has_kernel_driver0 {
                unwrap_safe!(handle_mut.attach_kernel_driver(0));
            }
        }
        // first, drop async_group to release all captured references to the DeviceHandle
        let async_ptr = &mut *self.async_group as *mut AsyncGroup<'static>;
        drop(unsafe { Box::from_raw(async_ptr) });
        // then, drop the DeviceHandle to release Context
        let handle_ptr = &*self.handle as *const _ as *mut DeviceHandle<'static>;
        drop(unsafe { Box::from_raw(handle_ptr) });
        let context_ptr = self.context as *const _ as *mut Context;
        drop(unsafe { Box::from_raw(context_ptr) });
    }
}

fn get_context() -> UsbResult<Context> {
    let mut context = try!(Context::new());
    context.set_log_level(LogLevel::Debug);
    context.set_log_level(LogLevel::Info);
    context.set_log_level(LogLevel::Warning);
    context.set_log_level(LogLevel::Error);
    context.set_log_level(LogLevel::None);
    Ok(context)
}

fn get_handle<'a>(context: &'a Context) -> UsbResult<(DeviceHandle<'a>, bool, bool)> {
    let devices = try!(context.devices());
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
            return Ok((handle, has_kernel_driver0, has_kernel_driver1));
        }
    }
    Err(Error::NoDevice)
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

