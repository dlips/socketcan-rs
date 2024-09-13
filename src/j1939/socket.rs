//! This module contains an implementation for a J1939 socket.
//! The [J1939Socket] can always be used with the [J1939Socket::send_to] and
//! [J1939Socker::recv_from] functions. For using the [std::io::Read] and
//! [std::io::Write] implementation, the socket first need to be connected.
//! For more details, see [Peer].
use crate::j1939::addr::J1939SockAddr;
use crate::j1939::SocketOptions;
use crate::j1939::{IoError, IoResult};
use libc::{CAN_J1939, PF_CAN};
use socket2::SockAddr;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::RawFd;
use std::os::unix::io::AsRawFd;

// ===== Private local helper functions =====

/// Tries to open the CAN socket by the interface number.
fn raw_open_socket(addr: &J1939SockAddr) -> IoResult<socket2::Socket> {
    let pf_can = socket2::Domain::from(PF_CAN);
    let can_j1939 = socket2::Protocol::from(CAN_J1939);
    let sock = socket2::Socket::new_raw(pf_can, socket2::Type::DGRAM, Some(can_j1939))?;
    sock.bind(&SockAddr::from(*addr))?;
    Ok(sock)
}

/// This trait is used to indicate if a peer was set with the `.connect(2)` system
/// call. The documentation of the [J1939 Linux kernel stack](https://www.kernel.org/doc/html/latest/networking/j1939.html#api-calls),
/// explains the following
///
/// ```text
/// On the other hand connect(2) assigns the remote address, i.e. the destination address.
/// The PGN from connect(2) is used as the default PGN when sending packets. If ADDR or NAME
/// is set it will be used as the default destination ADDR or NAME. Further a set ADDR or NAME
/// during connect(2) is used as a receive filter. It will match the source NAME or ADDR of
/// the incoming packet.
///
/// Both write(2) and send(2) will send a packet with local address from bind(2) and the remote
/// address from connect(2). Use sendto(2) to overwrite the destination address.
/// ```
///
/// This means that the socket is linked to a specific peer whose address was set with the
/// `connect(2)` call. After a peer was linked, the function calls to `read(2)` and `write(2)`
/// will automatically use the linked peer as a receive filter and destination address, respectively.
///
/// If no peer address was set with a call to `connect(2)`, then the kernel stack will return an
/// error if `read(2)`, `write(2)` or `send(2)` are called on the socket.
///
/// To prevent such an error at runtime, we use the typestate pattern to convert it to a compile time
/// error. Accordingly, [Peer] will indicate if a peer was already linked with [J1939Socket::connect].
pub trait Peer {}

/// Type indicating that no peer address is linked to the socket.
#[derive(Debug, Clone, Copy)]
pub struct Unlinked;
impl Peer for Unlinked {}

/// Type indicating that a peer address is linked to the socket.
#[derive(Debug, Clone, Copy)]
pub struct Linked;
impl Peer for Linked {}

/// A socket which communicates with the J1939 protocol on the CAN bus.
#[derive(Debug)]
pub struct J1939Socket<S: Peer> {
    inner: socket2::Socket,
    _state: std::marker::PhantomData<S>,
}

impl J1939Socket<Unlinked> {
    /// Opens a new J1939 socket with a given address.
    pub fn open(addr: &J1939SockAddr) -> IoResult<J1939Socket<Unlinked>> {
        let socket = raw_open_socket(addr)?;
        Ok(J1939Socket {
            inner: socket,
            _state: std::marker::PhantomData,
        })
    }
}

impl<S> J1939Socket<S>
where
    S: Peer,
{
    /// Rebinds the socket with a new address.
    pub fn rebind(&self, addr: &J1939SockAddr) -> IoResult<()> {
        self.inner.bind(&SockAddr::from(*addr))
    }

    /// Connects the socket to a specific peer.
    ///
    /// The peer is given by the destination address. This function must be called
    /// before the `read` and `write` traits are available. Otherwise, this would
    /// result in a error, see [J1939 kernel stack](https://docs.kernel.org/networking/j1939.html#api-calls).
    /// The implementation gurantees with the type state pattern that the [Read](std::io::Read) and
    /// [Write](std::io::Write) are only available for a socket, which has been connected.
    pub fn connect(self, dest: &J1939SockAddr) -> IoResult<J1939Socket<Linked>> {
        let J1939Socket { inner, .. } = self;
        inner.connect(&SockAddr::from(*dest))?;
        Ok(J1939Socket {
            inner,
            _state: std::marker::PhantomData,
        })
    }

    /// Sends a message to a given address.
    ///
    /// If the message in the buffer is larger than 8 bytes, the J1939 kernel stack will
    /// automatically switch to the (extended) transfer protocol. The function returns,
    /// the number of bytes that were written to the socket.
    pub fn send_to(&self, addr: &J1939SockAddr, buf: &[u8]) -> IoResult<usize> {
        self.inner.send_to(buf, &SockAddr::from(*addr))
    }

    /// Receives a message from the socket.
    ///
    /// Returns the number of bytes written to `buf` and the address from which the
    /// message was received.
    pub fn recv_from(&self, buf: &mut [u8]) -> IoResult<(usize, J1939SockAddr)> {
        // SAFETY: the `recv_from` implementation of the `socket2` crate promises
        // not to write unitialized bytes to the `buf`fer.
        let buf = unsafe { &mut *(buf as *mut [u8] as *mut [MaybeUninit<u8>]) };

        let (bytes_read, addr) = self.inner.recv_from(buf)?;
        let sa_addr =
            J1939SockAddr::try_from(addr).map_err(|_e| IoError::other("Invalid source address"))?;
        Ok((bytes_read, sa_addr))
    }
}

impl<S: Peer> SocketOptions for J1939Socket<S> {}

impl<S: Peer> AsRawFd for J1939Socket<S> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<S: Peer> crate::j1939::options::private::AsRawSocket for J1939Socket<S> {
    fn as_raw_socket(&self) -> &socket2::Socket {
        &self.inner
    }

    fn as_raw_socket_mut(&mut self) -> &mut socket2::Socket {
        &mut self.inner
    }
}

impl Read for J1939Socket<Linked> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.inner.read(buf)
    }
}

impl Write for J1939Socket<Linked> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.inner.flush()
    }
}
