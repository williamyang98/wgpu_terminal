use nix::{
    errno::Errno,
    sys::termios::{tcflush, tcgetattr, tcsetattr, FlushArg, InputFlags, SetArg},
    unistd::{read, write},
    fcntl::{fcntl, FcntlArg, OFlag},
};
use std::{
    ffi::CStr,
    io::{Read, Write, IsTerminal},
    os::fd::{OwnedFd, RawFd, BorrowedFd, AsRawFd, IntoRawFd, AsFd},
};
use cgmath::Vector2;

#[derive(Debug)]
pub struct MasterPty(OwnedFd);

impl From<OwnedFd> for MasterPty {
    fn from(fd: OwnedFd) -> Self {
        Self(fd)
    }
}

impl From<MasterPty> for OwnedFd {
    fn from(pty: MasterPty) -> Self {
        pty.0
    }
}

impl From<MasterPty> for std::fs::File {
    fn from(pty: MasterPty) -> Self {
        Self::from(pty.0)
    }
}

impl AsFd for MasterPty {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for MasterPty {
    fn as_raw_fd(&self)  -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for MasterPty {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl Write for MasterPty {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        write(self, buf).map_err(|e| e.into())
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        // https://man7.org/linux/man-pages/man3/tcflush.3p.html
        tcflush(self, FlushArg::TCOFLUSH).map_err(|e| e.into())
    }
}

impl Read for MasterPty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        read(self.as_raw_fd(), buf).map_err(|e| e.into())
    }
}

impl MasterPty {
    pub fn try_clone(&self) -> Result<Self, std::io::Error> {
        self.0.try_clone().map(Self)
    }

    pub fn is_terminal(&self) -> bool {
        self.0.is_terminal()
    }

    pub fn grantpt(&self) -> Result<(), Errno> {
        // https://man7.org/linux/man-pages/man3/grantpt.3.html
        match unsafe { libc::grantpt(self.as_raw_fd()) } {
             0 => Ok(()),
            -1 => Err(Errno::last()),
            code => {
                log::warn!("Got unexpected failure code: {}", code);
                Err(Errno::last())
            },
        }
    }

    pub fn unlockpt(&self) -> Result<(), Errno> {
        // https://man7.org/linux/man-pages/man3/unlockpt.3.html
        match unsafe { libc::unlockpt(self.as_raw_fd()) } {
             0 => Ok(()),
            -1 => Err(Errno::last()),
            code => {
                log::warn!("Got unexpected failure code: {}", code);
                Err(Errno::last())
            },
        }
    }
 

    pub fn set_utf8(&self, is_utf8: bool) -> Result<(), Errno> {
        let mut termios = tcgetattr(self)?;
        termios.input_flags.set(InputFlags::IUTF8, is_utf8);
        tcsetattr(self, SetArg::TCSANOW, &termios)?;
        Ok(())
    }

    pub fn set_blocking(&self, is_blocking: bool) -> Result<(), Errno> {
        let flag = fcntl(self.as_raw_fd(), FcntlArg::F_GETFL)?;
        let mut flag = OFlag::from_bits(flag).ok_or(Errno::EINVAL)?;
        flag.set(OFlag::O_NONBLOCK, !is_blocking);
        let _ = fcntl(self.as_raw_fd(), FcntlArg::F_SETFL(flag))?;
        Ok(())
    }

    pub fn ptsname(&self) -> Result<String, Errno> {
        // https://man7.org/linux/man-pages/man3/ptsname.3.html
        let name = unsafe { libc::ptsname(self.as_raw_fd()) };
        if name.is_null() {
            return Err(Errno::last());
        }
        let name = unsafe { CStr::from_ptr(name) };
        // copy string since it is in static storage and will be overwritten by subsequent calls
        let name = name.to_string_lossy().into_owned();
        Ok(name)
    }

    pub fn ptsname_r(&self) -> Result<String, Errno> {
        // https://man7.org/linux/man-pages/man3/ptsname.3.html
        const BUFFER_SIZE: usize = 128;
        let mut buf = vec![0 as libc::c_char; BUFFER_SIZE];
        let code = unsafe { libc::ptsname_r(self.as_raw_fd(), buf.as_mut_ptr(), buf.len()) };
        if code != 0 {
            return Err(Errno::from_raw(code));
        }
        let name = unsafe { CStr::from_ptr(buf.as_ptr()) };
        let name = name.to_string_lossy().into_owned();
        Ok(name)
    }

    pub fn get_window_size(&self) -> Result<Vector2<u16>, Errno> {
        let mut window_size: libc::winsize = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        let res = unsafe { libc::ioctl(self.as_raw_fd(), libc::TIOCGWINSZ, &mut window_size) };
        if res != 0 {
            if res != -1 {
                log::warn!("Got unexpected failure code: {}", res);
            }
            return Err(Errno::last());
        }
        let window_size = Vector2::new(window_size.ws_col, window_size.ws_row);
        Ok(window_size)
    }

    pub fn set_window_size(&self, size: Vector2<u16>) -> Result<(), Errno> {
        let window_size = libc::winsize {
            ws_row: size.y,
            ws_col: size.x,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let res = unsafe { libc::ioctl(self.as_raw_fd(), libc::TIOCSWINSZ, &window_size) };
        if res != 0 {
            if res != -1 {
                log::warn!("Got unexpected failure code: {}", res);
            }
            return Err(Errno::last());
        }
        Ok(())
    }
}
