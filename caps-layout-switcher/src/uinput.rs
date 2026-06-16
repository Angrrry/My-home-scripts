use crate::evdev::{EV_KEY, EV_SYN, InputEvent};
use std::fs::{File, OpenOptions};
use std::io;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong, c_void};
use std::thread;
use std::time::Duration;

const UI_DEV_CREATE: c_ulong = 0x5501;
const UI_DEV_DESTROY: c_ulong = 0x5502;
const UI_SET_EVBIT: c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: c_ulong = 0x4004_5565;
const KEY_MAX: c_int = 0x2ff;
const BUS_USB: u16 = 0x03;

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UInputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
}

pub struct VirtualKeyboard {
    file: File,
    created: bool,
}

impl VirtualKeyboard {
    pub fn create() -> io::Result<Self> {
        let file = OpenOptions::new().write(true).open("/dev/uinput")?;
        let mut keyboard = Self {
            file,
            created: false,
        };

        keyboard.ioctl_int(UI_SET_EVBIT, EV_SYN as c_int)?;
        keyboard.ioctl_int(UI_SET_EVBIT, EV_KEY as c_int)?;
        for key in 0..=KEY_MAX {
            keyboard.ioctl_int(UI_SET_KEYBIT, key)?;
        }

        let mut setup = UInputSetup {
            id: InputId {
                bustype: BUS_USB,
                vendor: 0x1d6b,
                product: 0x0104,
                version: 1,
            },
            name: [0; 80],
            ff_effects_max: 0,
        };

        let name = b"caps-layout-switcher virtual keyboard";
        setup.name[..name.len()].copy_from_slice(name);
        keyboard.ioctl_ptr(0x405c_5503, &setup)?;
        keyboard.ioctl_int(UI_DEV_CREATE, 0)?;
        keyboard.created = true;

        thread::sleep(Duration::from_millis(100));
        Ok(keyboard)
    }

    pub fn write_event(&mut self, event: &InputEvent) -> io::Result<()> {
        let expected = mem::size_of::<InputEvent>();
        let written = unsafe {
            write(
                self.file.as_raw_fd(),
                (event as *const InputEvent).cast::<c_void>(),
                expected,
            )
        };

        if written == -1 {
            return Err(io::Error::last_os_error());
        }
        if written as usize != expected {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!("short uinput write: {written} bytes"),
            ));
        }

        Ok(())
    }

    fn ioctl_int(&self, request: c_ulong, value: c_int) -> io::Result<()> {
        let result = unsafe { ioctl(self.file.as_raw_fd(), request, value) };
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn ioctl_ptr<T>(&self, request: c_ulong, value: &T) -> io::Result<()> {
        let result = unsafe { ioctl(self.file.as_raw_fd(), request, value as *const T) };
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for VirtualKeyboard {
    fn drop(&mut self) {
        if self.created {
            unsafe {
                ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY, 0);
            }
        }
    }
}
