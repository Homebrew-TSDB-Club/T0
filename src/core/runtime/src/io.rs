use crate::executor::CONTEXT;
use futures_lite::Stream;
use polling::Event;
use socket2::{Domain, Protocol, Socket, Type};
use std::io::{Error, IoSlice, Read, Write};
use std::net::Shutdown;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::AsRawFd;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Async<T: AsRawFd> {
    io: T,
    id: usize,
}

impl<T: AsRawFd> Async<T> {
    pub fn new(io: T) -> Self {
        let id = CONTEXT.with(|context| context.get().unwrap().polling.borrow_mut().add(&io));
        Self { io, id }
    }
}

impl Async<TcpListener> {
    pub fn connect(addr: SocketAddr, cpu_id: usize) -> Self {
        let sock = Socket::new(
            match addr {
                SocketAddr::V4(_) => Domain::IPV4,
                SocketAddr::V6(_) => Domain::IPV6,
            },
            Type::STREAM,
            Some(Protocol::TCP),
        )
        .unwrap();
        sock.set_reuse_address(true).unwrap();
        sock.set_reuse_port(true).unwrap();
        sock.set_nonblocking(true).unwrap();
        sock.set_nodelay(true).unwrap();
        Self::set_core_affinity(&sock, cpu_id);
        sock.bind(&addr.into()).unwrap();
        sock.listen(libc::S_IFREG as libc::c_int).unwrap();

        Async::new(sock.into())
    }

    #[cfg(target_os = "linux")]
    fn set_core_affinity(sock: &Socket, cpu_id: usize) {
        sock.set_cpu_affinity(cpu_id).unwrap();
    }

    #[cfg(not(target_os = "linux"))]
    fn set_core_affinity(_: &Socket, _: usize) {}
}

impl Stream for Async<TcpListener> {
    type Item = std::io::Result<Async<TcpStream>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        return match self.as_ref().io.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(true)
                    .expect("open socket with nonblocking error");
                stream.set_nodelay(true).expect("set tcp_nodelay failed");
                Poll::Ready(Some(Ok(Async::<TcpStream>::new(stream))))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                CONTEXT.with(|context| {
                    let context = context.get().unwrap();
                    context.polling.borrow_mut().modify(
                        self.id,
                        &self.io,
                        Event::readable,
                        cx.waker().clone(),
                    )
                });
                Poll::Pending
            }
            Err(e) => std::task::Poll::Ready(Some(Err(e))),
        };
    }
}

impl tokio::io::AsyncRead for Async<TcpStream> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        unsafe {
            let b = &mut *(buf.unfilled_mut() as *mut [std::mem::MaybeUninit<u8>] as *mut [u8]);
            match self.io.read(b) {
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    CONTEXT.with(|context| {
                        let context = context.get().unwrap();
                        context.polling.borrow_mut().modify(
                            self.id,
                            &self.io,
                            Event::readable,
                            cx.waker().clone(),
                        )
                    });
                    Poll::Pending
                }
                Ok(n) => {
                    buf.assume_init(n);
                    buf.advance(n);
                    Poll::Ready(Ok(()))
                }
                Err(e) => Poll::Ready(Err(e)),
            }
        }
    }
}

impl tokio::io::AsyncWrite for Async<TcpStream> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.io.write(buf) {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                CONTEXT.with(|context| {
                    let context = context.get().unwrap();
                    context.polling.borrow_mut().modify(
                        self.id,
                        &self.io,
                        Event::writable,
                        cx.waker().clone(),
                    )
                });
                Poll::Pending
            }
            x => Poll::Ready(x),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(self.io.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(self.io.shutdown(Shutdown::Both))
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.io.write_vectored(bufs) {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                CONTEXT.with(|context| {
                    let context = context.get().unwrap();
                    context.polling.borrow_mut().modify(
                        self.id,
                        &self.io,
                        Event::writable,
                        cx.waker().clone(),
                    )
                });
                Poll::Pending
            }
            x => Poll::Ready(x),
        }
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
}

impl<T: AsRawFd> Drop for Async<T> {
    fn drop(&mut self) {
        CONTEXT.with(|context| {
            context
                .get()
                .unwrap()
                .polling
                .borrow_mut()
                .delete(&self.io, self.id)
        });
    }
}
