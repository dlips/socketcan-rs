use crate::j1939::filter::J1939Filter;
use crate::{IoError, IoResult};
use libc::{
    c_int, j1939_filter, socklen_t, SOL_CAN_J1939, SOL_SOCKET, SO_J1939_FILTER, SO_J1939_PROMISC,
};
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::time::Duration;

pub(crate) mod private {
    /// Access the underlying `socket2::Socket`
    ///
    /// This trait is used internally to set socket options like broadcast,
    /// filters and timeouts.
    pub trait AsRawSocket {
        /// Returns a reference to the internal socket
        fn as_raw_socket(&self) -> &socket2::Socket;
        /// Returns a mutable reference to the internal socket
        fn as_raw_socket_mut(&mut self) -> &mut socket2::Socket;
    }
}

/// Trait for setting configuration options on a J19393 socket
///
/// These are blocking calls, even when implemented on asynchronous sockets.
pub trait SocketOptions: AsRawFd + private::AsRawSocket {
    /// Sets an option on the socket.
    ///
    /// The libc `setsockopt` function is used to set various options on a socket.
    /// `set_socket_option` offers a somewhat type-safe wrapper that does not
    /// require messing around with `*const c_void`s.
    ///
    /// A proper `std::io::Error` will be returned on failure.
    ///
    /// Example use:
    ///
    /// ```text
    /// sock.set_socket_option(SOL_SOCKET, SO_J1939_PROMISC, 1 as c_int)
    /// ```
    ///
    /// Note that the `val` parameter must be specified correctly; if an option
    /// expects an integer, it is advisable to pass in a `c_int`, not the default
    /// of `i32`.
    ///
    /// # Safety
    ///
    /// Because `val` is passed to [libc::setsockopt] internally, the caller must ensure
    /// that the referenced type has a C ABI compatible memory layout.
    unsafe fn set_socket_option<T>(&mut self, level: c_int, name: c_int, val: &T) -> IoResult<()> {
        let ret = unsafe {
            libc::setsockopt(
                self.as_raw_fd(),
                level,
                name,
                ptr::from_ref(val).cast(),
                size_of::<T>() as socklen_t,
            )
        };
        match ret {
            0 => Ok(()),
            _ => Err(IoError::last_os_error()),
        }
    }

    /// Returns an option on the socket.
    ///
    /// The libc `getsockopt` function is used to set various options on a socket.
    /// `get_socket_option` offers a somewhat type-safe wrapper that does not
    /// require messing around with `*const c_void`s.
    ///
    /// A proper `std::io::Error` will be returned on failure.
    ///
    /// Example use:
    ///
    /// ```text
    /// let mut val: c_int = 0;
    /// sock.get_socket_option(SOL_SOCKET, SO_J1939_PROMISC, &mut val)
    /// ```
    ///
    /// Note that the `val` parameter must be specified correctly; if an option
    /// expects an integer, it is advisable to pass in a `c_int`, not the default
    /// of `i32`.
    ///
    /// # Safety
    ///
    /// Because `val` is passed to [libc::getsockopt] internally, the caller must ensure
    /// that the referenced type has a C ABI compatible memory layout.
    unsafe fn get_socket_option<T>(&self, level: c_int, name: c_int, val: &mut T) -> IoResult<()> {
        // See https://man7.org/linux/man-pages/man2/getsockopt.2.html
        // ```text
        // For getsockopt(), optlen is a value-result argument, initially
        // containing the size of the buffer pointed to by optval, and
        // modified on return to indicate the actual size of the value
        // returned.
        // ```
        let mut optlen: socklen_t = size_of::<T>() as socklen_t;

        let ret = unsafe {
            libc::getsockopt(
                self.as_raw_fd(),
                level,
                name,
                ptr::from_mut(val).cast(),
                ptr::from_mut(&mut optlen).cast(),
            )
        };

        match ret {
            0 => Ok(()),
            _ => Err(IoError::last_os_error()),
        }
    }

    /// Sets a collection of multiple socket options with one call.
    ///  
    /// # Safety
    ///
    /// Because `val` is passed to [libc::setsockopt] internally, the caller must ensure
    /// that the referenced type has a C ABI compatible memory layout.
    unsafe fn set_socket_option_mult<T>(
        &mut self,
        level: c_int,
        name: c_int,
        values: &[T],
    ) -> IoResult<()> {
        let ret = unsafe {
            if values.is_empty() {
                // can't pass in a ptr to a 0-len slice, pass a null ptr instead
                libc::setsockopt(self.as_raw_fd(), level, name, ptr::null(), 0)
            } else {
                libc::setsockopt(
                    self.as_raw_fd(),
                    level,
                    name,
                    values.as_ptr().cast(),
                    size_of_val(values) as socklen_t,
                )
            }
        };

        match ret {
            0 => Ok(()),
            _ => Err(IoError::last_os_error()),
        }
    }

    /// Sets receive filters for the J1939 socket.
    ///
    /// The `filter` module provides multiple filter types, including [NameFilter](crate::filter::NameFilter),
    /// [PgnFilter](crate::filter::PgnFilter), and [AddrFilter](crate::filter::AddrFilter). These are convience
    /// wrappers around the more generic `J1939Filter`, which allows to set an arbitrary combination of
    /// the before mentioned filter types.
    fn set_filters(&mut self, filters: impl IntoIterator<Item = J1939Filter>) -> IoResult<()> {
        // SAFETY: libc::j1939_filter is a C struct that has the correct memory layout expected by
        // set_socket_option_mult.
        let filters: Vec<j1939_filter> = filters.into_iter().map(|f| f.into()).collect();
        unsafe { self.set_socket_option_mult(SOL_CAN_J1939, SO_J1939_FILTER, filters.as_slice()) }
    }

    /// Returns if the socket is configured in promisc mode.
    ///
    /// Promisc mode diables all filters set by the `bind()` and `connect()` calls,
    /// and the `SOCK_BROADCAST` option. For more details, see the RX path description
    /// of the [J1939 kernel stack](https://docs.kernel.org/networking/j1939.html#api-calls).
    fn promisc(&self) -> IoResult<bool> {
        // SAFETY: The option value for SO_J1939_PROMISC is a C integer, which has the correct
        // memory layout which expected by [SocketOption::get_socket_option]
        let mut promisc: c_int = 0;
        unsafe { self.get_socket_option(SOL_SOCKET, SO_J1939_PROMISC, &mut promisc)? };
        Ok(promisc != 0)
    }

    /// Sets the promisc mode of the socket. For a description of promisc mode,
    /// see [SocketOptions::promisc].
    ///
    /// If `value` is `true`, then the promisc mode will be activated, and if `value`
    /// is `false` then the promisc mode will be disabled.    
    fn set_promisc(&mut self, promisc: bool) -> IoResult<()> {
        // SAFETY: The option value for SO_J1939_PROMISC is a C integer, which has the correct
        // memory layout which expected by [SocketOption::set_socket_option]
        let promisc: c_int = match promisc {
            true => 1,
            false => 0,
        };
        unsafe { self.set_socket_option(SOL_SOCKET, SO_J1939_PROMISC, &promisc) }
    }

    /// Returns if the socket is configured in non-blocking mode.
    ///
    /// If true is returned, then the socket is configured as nonblocking, and if false
    /// is return, then the socket is configured as blocking.
    fn nonblocking(&self) -> IoResult<bool> {
        self.as_raw_socket().nonblocking()
    }

    /// Sets non-blocking mode for the socket.
    ///
    /// If `nonblocking` is true,
    /// then the socket will be configured as nonblocking.
    fn set_nonblocking(&mut self, nonblocking: bool) -> IoResult<()> {
        self.as_raw_socket().set_nonblocking(nonblocking)
    }

    /// Returns the currently set timeout for read operations.
    ///
    /// If no timeout is set, then `None` is returned.
    fn read_timeout(&self) -> IoResult<Option<Duration>> {
        self.as_raw_socket().read_timeout()
    }

    /// Sets the timeout for read operations on the socket.
    ///
    /// If `duration` is `None`, then an already set timeout will be cleared from
    /// the socket.
    fn set_read_timeout(&mut self, duration: Option<Duration>) -> IoResult<()> {
        self.as_raw_socket().set_read_timeout(duration)
    }

    /// Returns the currently set timeout for write operations.
    ///
    /// If no timeout is set, then `None` is returned.
    fn write_timeout(&self) -> IoResult<Option<Duration>> {
        self.as_raw_socket().write_timeout()
    }

    /// Sets the timeout for write operations on the socket.
    ///
    /// If `duration` is `None`, then an already set timeout will be cleared from
    /// the socket.
    fn set_write_timeout(&mut self, duration: Option<Duration>) -> IoResult<()> {
        self.as_raw_socket().set_write_timeout(duration)
    }

    /// Returns if the socket is currently set to broadcast mode.
    ///
    /// If broadcast mode is enabled, then all messages send to the global J1939
    /// address (0xFF) can be received by the socket. If broadcast mode is disabled,
    /// then no broadcast messages can be send with this socket.
    fn broadcast(&self) -> IoResult<bool> {
        self.as_raw_socket().broadcast()
    }

    /// Set the brodcast mode of the J1939 socket. For a description of the broadcast mode
    /// see [SocketOptions::broadcast].
    ///
    /// If `value` is `true`, then the broadcast mode will be activated, and if `value`
    /// is `false` then the broadcast mode will be disabled.
    fn set_broadcast(&mut self, broadcast: bool) -> IoResult<()> {
        self.as_raw_socket().set_broadcast(broadcast)
    }
}
