use bytes::BytesMut;
use prost::Message;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;
use storage::StorageServer;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tracing::{info, warn};

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
            info!("start");
            let socket = self.listener.accept().await?.0;
            info!("accept");
            let server = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(err) = server.handle(socket).await {
                    if err.kind() != ErrorKind::UnexpectedEof {
                        warn!("tcp handle error: {:?}", err);
                    }
                }
            });
        }
    }

    pub async fn handle(&self, mut socket: TcpStream) -> io::Result<()> {
        loop {
            let mut buf = BytesMut::with_capacity(socket.read_u64().await? as usize);
            socket.read_buf(&mut buf).await?;
            let request = Message::decode(&mut buf)?;
            self.server.grpc_write(request).await;
        }
    }
}
