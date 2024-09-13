//! Wrapper for the J1939 linux kernel stack.
//! For more details, see https://www.kernel.org/doc/html/latest/networking/j1939.html
mod addr;
mod filter;
mod options;
mod protocol;
pub mod socket;
#[cfg(feature = "tokio")]
pub mod tokio;

pub use addr::{J1939SockAddr, J1939SockAddrError};
pub use filter::*;
pub use options::SocketOptions;
pub use protocol::{Addr, Name, Pgn};

pub(crate) type IoError = std::io::Error;
pub(crate) type IoResult<T> = std::io::Result<T>;
