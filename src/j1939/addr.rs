use crate::{
    as_bytes, as_bytes_mut,
    j1939::protocol::{Addr, Name, Pgn},
};
use libc::{
    sa_family_t, sockaddr_can, sockaddr_storage, socklen_t, AF_CAN, J1939_NO_ADDR, J1939_NO_NAME,
    J1939_NO_PGN,
};
use nix::net::if_::if_nametoindex;
use socket2::SockAddr;
use std::{fmt, mem, os::raw::c_int};
use thiserror::Error;

/// Error related to the handling of [J1939SockAddr]
#[derive(Error, Debug, Copy, Clone)]
pub enum J1939SockAddrError {
    /// Found invalid address family
    #[error("Invalid address family. Found address family other than AF_CAN.")]
    InvalidAddressFamily,
}

/// A specific socket address for a J1939 socket, see [Socket address structures](https://man7.org/linux/man-pages/man7/socket.7.html).
#[derive(Clone, Copy)]
pub struct J1939SockAddr(sockaddr_can);

impl J1939SockAddr {
    /// Creates a new [J1939SockAddr] with the given interface index.
    pub fn new(ifindex: u32) -> Self {
        let mut socket_addr = Self::default();
        socket_addr.0.can_ifindex = ifindex as c_int;
        socket_addr
    }

    /// Creates a new [J1939SockAddr] with an interface index, J1939 NAME,
    /// PGN, and control function address.
    pub fn from_parts(ifindex: u32, name: Name, pgn: Pgn, addr: Addr) -> Self {
        let mut socket_addr = Self::default();
        socket_addr.0.can_ifindex = ifindex as c_int;
        socket_addr.0.can_addr.j1939.name = name.into();
        socket_addr.0.can_addr.j1939.pgn = pgn.into();
        socket_addr.0.can_addr.j1939.addr = addr.into();
        socket_addr
    }

    /// Creates a new [J1939SockAddr] from a given interface name.
    pub fn with_ifname(ifname: &str) -> std::io::Result<Self> {
        let ifindex = if_nametoindex(ifname)?;
        Ok(Self::new(ifindex))
    }

    /// Creates a new [J1939SockAddr] with a interface name, J1939 NAME,
    /// PGN, and control function address.
    pub fn from_parts_with_ifname(
        ifname: &str,
        name: Name,
        pgn: Pgn,
        addr: Addr,
    ) -> std::io::Result<Self> {
        let ifindex = if_nametoindex(ifname)?;
        Ok(Self::from_parts(ifindex, name, pgn, addr))
    }

    /// Returns the J1939 NAME, see https://elearning.vector.com/mod/page/view.php?id=259
    pub fn name(&self) -> Name {
        // SAFETY: Implementation gurantees that the inner field is always initialized
        Name::from(unsafe { self.0.can_addr.j1939.name })
    }

    /// Sets the J1939 NAME.
    pub fn set_name(&mut self, name: Name) {
        self.0.can_addr.j1939.name = name.into();
    }

    /// Returns the control function address.
    pub fn addr(&self) -> Addr {
        // SAFETY: Implementation gurantees that the inner field is always initialized
        Addr::from(unsafe { self.0.can_addr.j1939.addr })
    }

    /// Sets the control function address.
    pub fn set_addr(&mut self, addr: Addr) {
        self.0.can_addr.j1939.addr = addr.into()
    }

    /// Returns the Parameter Group Number
    pub fn pgn(&self) -> Pgn {
        // SAFETY: Implementation gurantees that the inner field is always initialized
        Pgn::from(unsafe { self.0.can_addr.j1939.pgn })
    }

    /// Sets the Parameter Group Number
    pub fn set_pgn(&mut self, pgn: Pgn) {
        self.0.can_addr.j1939.pgn = pgn.into();
    }

    /// Returns the underlying socket address as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        as_bytes(&self.0)
    }

    /// Converts the address into a [sockaddr_storage].
    /// This is a generic socket address container with enough space to hold
    /// any address type in the system.
    pub fn into_storage(self) -> (sockaddr_storage, socklen_t) {
        let can_addr = self.as_bytes();
        let len = can_addr.len();

        let mut storage: sockaddr_storage = unsafe { mem::zeroed() };
        let sock_addr = as_bytes_mut(&mut storage);

        sock_addr[..len].copy_from_slice(can_addr);
        (storage, len as socklen_t)
    }
}

impl Default for J1939SockAddr {
    fn default() -> Self {
        let mut addr: sockaddr_can = unsafe { mem::zeroed() };
        addr.can_family = AF_CAN as sa_family_t;
        addr.can_addr = libc::__c_anonymous_sockaddr_can_can_addr {
            j1939: libc::__c_anonymous_sockaddr_can_j1939 {
                #[cfg(target_pointer_width = "32")]
                name: J1939_NO_NAME as u64,
                #[cfg(target_pointer_width = "64")]
                name: J1939_NO_NAME,
                pgn: J1939_NO_PGN,
                addr: J1939_NO_ADDR,
            },
        };
        Self(addr)
    }
}

impl fmt::Debug for J1939SockAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // SAFETY: The J1939SockAddr struct takes care that all the fields of the
        // internal `socket_can` struct are always properly initialized. Structs
        // returned by the kernel are also guranteed to be intitialized.
        write!(
            f,
            "J1939SockAddr {{ can_family: {}, can_ifindex: {}, NAME: {:#018X}, PGN: {:#08X}, address: {:#04X} }}",
            self.0.can_family,
            self.0.can_ifindex,
            unsafe { self.0.can_addr.j1939.name },
            unsafe { self.0.can_addr.j1939.pgn },
            unsafe { self.0.can_addr.j1939.addr }
        )
    }
}

impl From<J1939SockAddr> for SockAddr {
    fn from(addr: J1939SockAddr) -> Self {
        let (storage, len) = addr.into_storage();
        // SAFETY: `SockAddr` creation is safe because the call to into_storage
        // correctly initializes the libc::sockaddr_storage from a libc::sockaddr_can.
        unsafe { SockAddr::new(storage, len) }
    }
}

impl TryFrom<SockAddr> for J1939SockAddr {
    type Error = J1939SockAddrError;
    fn try_from(addr: SockAddr) -> Result<Self, Self::Error> {
        let storage = addr.as_storage();
        if storage.ss_family != AF_CAN as sa_family_t {
            return Err(J1939SockAddrError::InvalidAddressFamily);
        }
        // SAFETY: `SockAddr` contains a `sockaddr_storage` that serves as a container for all
        // other types of socket address. The cast to `sockaddr_can` should be safe because
        // `sockaddr_storage` has the same or a larger size than `sockaddr_can`.
        unsafe {
            let can_addr_ptr = &storage as *const sockaddr_storage as *const sockaddr_can;
            Ok(Self(*can_addr_ptr))
        }
    }
}

impl PartialEq for J1939SockAddr {
    fn eq(&self, other: &Self) -> bool {
        self.0.can_family == other.0.can_family
            && self.0.can_ifindex == other.0.can_ifindex
            && self.name() == other.name()
            && self.pgn() == other.pgn()
            && self.addr() == other.addr()
    }
}

impl Eq for J1939SockAddr {}

#[cfg(test)]
mod tests {
    use super::J1939SockAddr;
    use crate::j1939::protocol::{Addr, Name, Pgn};

    #[test]
    fn test_correct_format_output() {
        let addr = J1939SockAddr::from_parts(
            0,
            Name::from(0x0000000001000004),
            Pgn::from(0x01F004),
            Addr::from(0x02),
        );
        let addr_fmt = format!("{:?}", addr);
        let addr_fmt_correct = String::from("J1939SockAddr { can_family: 29, can_ifindex: 0, NAME: 0x0000000001000004, PGN: 0x01F004, address: 0x02 }");
        assert_eq!(addr_fmt, addr_fmt_correct);
    }
}
