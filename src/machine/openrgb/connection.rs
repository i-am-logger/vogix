//! The socket side of vogix's two SDK clients, the owner and `inspect`: a
//! blocking connect to the loopback endpoint, then non-blocking reads and
//! writes that move bytes between the socket and a pure state machine
//! ([`super::session::Session`] or [`super::session::Mirror`]).
//!
//! The connect is blocking because the endpoint is loopback (the machine
//! config accepts nothing else): it completes, or is refused, at once. After
//! it the socket is non-blocking and every wait is the caller's `poll(2)`.

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};

/// How large one read from the socket is.
pub const READ_CHUNK: usize = 64 * 1024;

/// Connect to the SDK server and make the socket non-blocking. Small request
/// frames are sent at once rather than coalesced (`TCP_NODELAY`).
pub fn connect(host: Ipv4Addr, port: u16) -> io::Result<TcpStream> {
    let stream = TcpStream::connect(SocketAddrV4::new(host, port))?;
    stream.set_nodelay(true)?;
    stream.set_nonblocking(true)?;
    Ok(stream)
}

/// Whether the peer is still there after a read.
#[derive(Debug)]
pub enum Peer {
    /// Everything available was read.
    Open,
    /// The peer closed the connection (`error` is `None` for an orderly
    /// close, the socket error for a reset).
    Closed { error: Option<io::Error> },
}

/// Read everything the socket holds now, handing each chunk to `sink`.
pub fn read_available<S: Read>(
    stream: &mut S,
    buf: &mut [u8],
    mut sink: impl FnMut(&[u8]),
) -> Peer {
    loop {
        match stream.read(buf) {
            Ok(0) => return Peer::Closed { error: None },
            Ok(n) => sink(&buf[..n]),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Peer::Open,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Peer::Closed { error: Some(e) },
        }
    }
}

/// Write as much of `pending` as the socket takes now; returns the count
/// written. An error means the connection is gone (a Rust `TcpStream` or
/// `UnixStream` writes with `MSG_NOSIGNAL`, so a closed peer is `EPIPE`, not
/// `SIGPIPE`).
pub fn write_available<S: Write>(stream: &mut S, pending: &[u8]) -> io::Result<usize> {
    let mut written = 0;
    while written < pending.len() {
        match stream.write(&pending[written..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => written += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;

    #[test]
    fn reads_until_the_socket_is_empty_then_reports_open() {
        let (mut a, mut b) = UnixStream::pair().unwrap();
        a.set_nonblocking(true).unwrap();
        b.write_all(b"hello, ").unwrap();
        b.write_all(b"world").unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 4];
        let peer = read_available(&mut a, &mut buf, |chunk| got.extend_from_slice(chunk));
        assert!(matches!(peer, Peer::Open));
        assert_eq!(got, b"hello, world");
    }

    #[test]
    fn an_orderly_close_is_reported_after_the_last_bytes() {
        let (mut a, mut b) = UnixStream::pair().unwrap();
        a.set_nonblocking(true).unwrap();
        b.write_all(b"last").unwrap();
        drop(b);
        let mut got = Vec::new();
        let mut buf = [0u8; 64];
        let peer = read_available(&mut a, &mut buf, |chunk| got.extend_from_slice(chunk));
        assert!(matches!(peer, Peer::Closed { error: None }));
        assert_eq!(got, b"last");
    }

    #[test]
    fn writes_stop_when_the_socket_is_full() {
        let (mut a, _b) = UnixStream::pair().unwrap();
        a.set_nonblocking(true).unwrap();
        let big = vec![7u8; 16 * 1024 * 1024];
        let written = write_available(&mut a, &big).unwrap();
        assert!(written > 0 && written < big.len(), "wrote {written}");
    }

    #[test]
    fn a_write_to_a_closed_peer_is_an_error_not_a_signal() {
        let (mut a, b) = UnixStream::pair().unwrap();
        a.set_nonblocking(true).unwrap();
        drop(b);
        let err = write_available(&mut a, b"more").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn a_refused_loopback_connect_fails_at_once() {
        // Bind then drop a listener: nothing listens on that port now.
        let port = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = connect(Ipv4Addr::LOCALHOST, port).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::ConnectionRefused);
    }
}
