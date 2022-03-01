use prost::Message;
use std::io;
use std::io::{Cursor, ErrorKind};
use std::sync::Arc;
use storage::StorageServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tracing::{debug, warn};

const MAGIC_CODE: u64 = 0x9d2bd00b191c59e9;

const MAX_MESSAGE_SIZE: u64 = 1 << 16;

#[derive(Debug)]
pub struct Server {
    listener: TcpListener,
    server: StorageServer,
}

impl Server {
    pub async fn bind<A: ToSocketAddrs + Send>(addr: A, server: StorageServer) -> io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
            server,
        })
    }

    pub async fn serve(self: Arc<Self>) -> io::Result<()> {
        loop {
            debug!("socket start listen");
            let mut socket = self.listener.accept().await?.0;
            debug!("accept socket");
            let server = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(err) = server.handle(&mut socket).await {
                    if err.kind() == ErrorKind::UnexpectedEof {
                        return;
                    }
                    warn!("tcp handle error: {:?}", err);
                }
                if let Err(err) = socket.shutdown().await {
                    warn!("tcp shutdown error: {:?}", err);
                }
            });
        }
    }

    pub async fn handle(&self, socket: &mut TcpStream) -> io::Result<()> {
        socket.set_nodelay(true).unwrap();
        if socket.read_u64().await? != MAGIC_CODE {
            return Ok(());
        }
        let mut buf = Vec::with_capacity(MAX_MESSAGE_SIZE as usize);
        loop {
            let len = socket.read_u64().await?;
            if len > MAX_MESSAGE_SIZE {
                warn!("receive message larger than 64KB");
                return Ok(());
            }
            buf.resize(len as usize, 0);
            socket.read_exact(&mut buf).await?;
            let request = flat::write::root_as_write_request(&buf)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, format!("{:?}", err)))?;
            self.server.grpc_write(request).await;
            buf.clear();
        }
    }
}
