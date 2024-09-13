use crate::addr::J1939SockAddr;
pub use crate::options::SocketOptions;
use crate::socket::{J1939Socket, Linked, Peer, Unlinked};
use crate::IoResult;
use futures::{ready, task::Context};
use std::{
    io::{Read, Write},
    os::fd::AsRawFd,
    pin::Pin,
    task::Poll,
};
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// An asynchronous J1939 socket
pub struct AsyncJ1939Socket<S: Peer>(AsyncFd<J1939Socket<S>>);

impl AsyncJ1939Socket<Unlinked> {
    /// Opens a new J1939 socket with a given address.
    pub fn open(addr: &J1939SockAddr) -> IoResult<AsyncJ1939Socket<Unlinked>> {
        let mut socket = J1939Socket::open(addr)?;
        socket.set_nonblocking(true)?;
        Ok(AsyncJ1939Socket(AsyncFd::new(socket)?))
    }
}

impl<S: Peer> AsyncJ1939Socket<S> {
    /// Rebinds the socket with a new address.
    pub fn rebind(&self, addr: &J1939SockAddr) -> IoResult<()> {
        self.0.get_ref().rebind(addr)
    }

    /// Connects the socket to a specific peer.
    ///
    /// The peer is given by the destination address. This function must be called
    /// before the `read` and `write` functions can be used. Otherwise, this will
    /// result in a error, see [J1939 kernel stack](https://docs.kernel.org/networking/j1939.html#api-calls).
    pub fn connect(self, dest: &J1939SockAddr) -> IoResult<AsyncJ1939Socket<Linked>> {
        // The J1939 kernel stack just stores the destination address of the peer
        // on a call to `connect(2)`. Therefore, the J1939 kernel stack will not return
        // EINPROGRESS, which is different to e.g. a TcpSocket.
        // See for reference, https://github.com/torvalds/linux/blob/0c3836482481200ead7b416ca80c68a29cfdaabd/net/can/j1939/socket.c#L534
        let AsyncJ1939Socket(inner) = self;
        let socket = inner.into_inner();
        Ok(AsyncJ1939Socket(AsyncFd::new(socket.connect(dest)?)?))
    }

    /// Sends a message to a given address.
    ///
    /// If the message in the buffer is larger than 8 bytes, the J1939 kernel stack will
    /// automatically switch to the (extended) transfer protocol. The function returns,
    /// the number of bytes that were written to the socket.
    pub async fn send_to(&self, addr: &J1939SockAddr, buf: &[u8]) -> IoResult<usize> {
        self.0
            .async_io(Interest::WRITABLE, |socket| socket.send_to(addr, buf))
            .await
    }

    /// Receives a message from the socket.
    ///
    /// Returns the number of bytes written to `buf` and the address from which the
    /// message was received.
    pub async fn recv_from(&self, buf: &mut [u8]) -> IoResult<(usize, J1939SockAddr)> {
        self.0
            .async_io(Interest::READABLE, |socket| socket.recv_from(buf))
            .await
    }
}

impl<S: Peer> crate::options::private::AsRawSocket for AsyncJ1939Socket<S> {
    fn as_raw_socket(&self) -> &socket2::Socket {
        self.0.get_ref().as_raw_socket()
    }

    fn as_raw_socket_mut(&mut self) -> &mut socket2::Socket {
        self.0.get_mut().as_raw_socket_mut()
    }
}

impl<S: Peer> AsRawFd for AsyncJ1939Socket<S> {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.0.as_raw_fd()
    }
}

impl<S: Peer> SocketOptions for AsyncJ1939Socket<S> {}

// This implementation for `AsyncRead` and `AsyncWrite` follows the examples for the AsyncFd
// in the tokio crate documentation.
// See https://docs.rs/tokio/latest/tokio/io/unix/struct.AsyncFd.html#examples
impl AsyncRead for AsyncJ1939Socket<Linked> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        loop {
            let mut guard = ready!(self.0.poll_read_ready_mut(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| inner.get_mut().read(unfilled)) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for AsyncJ1939Socket<Linked> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        loop {
            let mut guard = ready!(self.0.poll_write_ready_mut(cx))?;

            match guard.try_io(|inner| inner.get_mut().write(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        // Different to a TcpSocket, which requires an explicit shutdown, a J1939 socket
        // doesn't perform an action on shutdown. Therefore we can just drop the socket
        // without doing anything extra.
        // See for reference https://github.com/torvalds/linux/blob/0c3836482481200ead7b416ca80c68a29cfdaabd/net/can/j1939/socket.c#L1319
        Poll::Ready(Ok(()))
    }
}
