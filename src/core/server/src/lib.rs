pub mod error;

use crate::error::Error as ServerError;
use hashbrown::HashMap;
use http::Response;
use hyper::server::conn::Http;
use hyper::{Body, Request};
use proto::BoxBody;
use proto::Never;
use runtime::io::Async;
use runtime::{Runtime, StreamExt};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use tower::util::BoxCloneService;
use tower_service::Service;

#[derive(Clone)]
struct HyperExecutor;
impl<F> hyper::rt::Executor<F> for HyperExecutor
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, fut: F) {
        runtime::spawn(fut).detach();
    }
}

pub type BoxFuture<T, E> = self::Pin<Box<dyn self::Future<Output = Result<T, E>> + Send + 'static>>;

type ServiceFactory<T, U, E> = Box<dyn Fn() -> BoxCloneService<T, U, E> + Send + Sync + 'static>;

type NamedServices<T, U, E> = HashMap<&'static str, ServiceFactory<T, U, E>>;

struct Router<T, U, E> {
    inner: Arc<RwLock<NamedServices<T, U, E>>>,
}

impl<T, U, E> Router<T, U, E> {
    fn new(svc: Vec<(&'static str, ServiceFactory<T, U, E>)>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(
                svc.into_iter()
                    .map(|(name, service)| (name, service))
                    .collect(),
            )),
        }
    }
}

impl<T, U, E> Clone for Router<T, U, E> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T, U, E> std::fmt::Debug for Router<T, U, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router").finish()
    }
}

impl Service<Request<Body>> for Router<Request<Body>, Response<BoxBody>, Never> {
    type Response = http::Response<BoxBody>;
    type Error = ServerError;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let router = self.inner.clone();
        let fut = async move {
            let (name, _) =
                req.uri()
                    .path()
                    .split_once('/')
                    .ok_or_else(|| ServerError::InvalidURI {
                        uri: req.uri().path().to_owned(),
                        desc: String::from("should have service name"),
                    })?;
            let mut service = {
                let services = router.write().unwrap();
                services
                    .get(name)
                    .ok_or_else(|| ServerError::ServiceNotExist {
                        name: name.to_owned(),
                        services: services.keys().cloned().collect(),
                    })?()
            };
            service
                .call(req)
                .await
                .map_err(|error| ServerError::Internal { err: error.into() })
        };
        Box::pin(fut)
    }
}

#[derive(Debug)]
pub struct GrpcServer {
    runtime: Runtime<()>,
    addr: SocketAddr,
    router: Router<Request<Body>, Response<BoxBody>, Never>,
}

type GrpcService = ServiceFactory<Request<Body>, Response<BoxBody>, Never>;

impl GrpcServer {
    pub fn new(
        addr: SocketAddr,
        cores: &[usize],
        services: Vec<(&'static str, GrpcService)>,
    ) -> Self {
        let runtime = Runtime::new(cores).unwrap();
        Self {
            runtime,
            addr,
            router: Router::new(services),
        }
    }

    pub fn run(&mut self) {
        let addr = self.addr;
        let router = self.router.clone();
        self.runtime.run(move |id, _| async move {
            let mut socket = Async::connect(addr, id);
            while let Ok(stream) = socket.next().await.unwrap() {
                let router = router.clone();
                runtime::spawn(async move {
                    let _ = Http::new()
                        .with_executor(HyperExecutor)
                        .serve_connection(stream, router)
                        .await;
                })
                .detach();
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::GrpcServer;
    use async_trait::async_trait;
    use proto::ping::ping_pong_server::{PingPong, PingPongServer};
    use proto::ping::{PingRequest, Pong};
    use proto::prometheus::remote_server::{Remote, RemoteServer};
    use proto::prometheus::{WriteRequest, WriteResponse};
    use proto::NamedService;
    use proto::{Request as GrpcRequest, Response, Status};
    use tower::util::BoxCloneService;

    #[derive(Clone)]
    struct RemoteWrite {}

    #[async_trait]
    impl Remote for RemoteWrite {
        async fn write(
            &self,
            _request: GrpcRequest<WriteRequest>,
        ) -> Result<Response<WriteResponse>, Status> {
            todo!()
        }
    }

    #[derive(Clone)]
    struct Ping {}

    #[async_trait]
    impl PingPong for Ping {
        async fn ping(&self, _: GrpcRequest<PingRequest>) -> Result<Response<Pong>, Status> {
            todo!()
        }
    }

    #[test]
    fn test_service() {
        let addr = "0.0.0.0:1107".parse().unwrap();
        let mut s = GrpcServer::new(
            addr,
            &[0],
            vec![
                (
                    RemoteServer::<RemoteWrite>::NAME,
                    Box::new(|| {
                        let service = RemoteServer::new(RemoteWrite {});
                        BoxCloneService::new(service)
                    }),
                ),
                (
                    PingPongServer::<Ping>::NAME,
                    Box::new(|| {
                        let service = RemoteServer::new(RemoteWrite {});
                        BoxCloneService::new(service)
                    }),
                ),
            ],
        );
        s.run();
    }
}
