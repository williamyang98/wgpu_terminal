mod process;
mod common;
#[cfg(windows)]
mod win32;
#[cfg(unix)]
mod unix;

pub use process::TerminalProcess;
pub use common::RawProcess;
#[cfg(windows)]
pub use win32::ConptyProcess;
#[cfg(unix)]
pub use unix::TermiosProcess;

