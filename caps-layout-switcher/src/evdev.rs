use std::fs::{self, File, OpenOptions};
use std::io;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong, c_void};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const EV_KEY: u16 = 0x01;
pub const EV_SYN: u16 = 0x00;
pub const KEY_CAPSLOCK: u16 = 58;
pub const KEY_UP: i32 = 0;
pub const KEY_DOWN: i32 = 1;

const EVIOCGRAB: c_ulong = 0x4004_4590;
const POLLIN: i16 = 0x001;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InputEvent {
    time: TimeVal,
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub fn is_caps_lock(&self) -> bool {
        self.event_type == EV_KEY && self.code == KEY_CAPSLOCK
    }
}

impl InputEvent {
    pub fn timestamp(&self) -> Duration {
        Duration::from_secs(self.time.tv_sec.max(0) as u64)
            + Duration::from_micros(self.time.tv_usec.max(0) as u64)
    }
}

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: c_int) -> c_int;
}

pub struct Device {
    file: File,
    grabbed: bool,
}

impl Device {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).open(path)?;
        Ok(Self {
            file,
            grabbed: false,
        })
    }

    pub fn grab(&mut self) -> io::Result<()> {
        let result = unsafe { ioctl(self.file.as_raw_fd(), EVIOCGRAB, 1) };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        self.grabbed = true;
        Ok(())
    }

    pub fn read_event_timeout(&mut self, timeout: Duration) -> io::Result<Option<InputEvent>> {
        let timeout_ms = timeout.as_millis().min(c_int::MAX as u128) as c_int;
        let mut poll_fd = PollFd {
            fd: self.file.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        let ready = unsafe { poll(&mut poll_fd, 1, timeout_ms) };
        if ready == -1 {
            return Err(io::Error::last_os_error());
        }
        if ready == 0 {
            return Ok(None);
        }

        let mut event = mem::MaybeUninit::<InputEvent>::uninit();
        let expected = mem::size_of::<InputEvent>();
        let read_bytes = unsafe {
            read(
                self.file.as_raw_fd(),
                event.as_mut_ptr().cast::<c_void>(),
                expected,
            )
        };

        if read_bytes == -1 {
            return Err(io::Error::last_os_error());
        }
        if read_bytes as usize != expected {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("short input_event read: {read_bytes} bytes"),
            ));
        }

        Ok(Some(unsafe { event.assume_init() }))
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if self.grabbed {
            unsafe {
                ioctl(self.file.as_raw_fd(), EVIOCGRAB, 0);
            }
        }
    }
}

pub fn candidate_keyboard_devices() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir("/dev/input/by-id") {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.ends_with("event-kbd") {
                paths.push(path);
            }
        }
    }

    if paths.is_empty() {
        if let Ok(entries) = fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if name.starts_with("event") {
                    paths.push(path);
                }
            }
        }
    }

    paths.sort();
    paths
}
