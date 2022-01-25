#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MetricMetadata {
    /// Represents the metric type, these match the set from Prometheus.
    /// Refer to pkg/textparse/interface.go for details.
    #[prost(enumeration = "metric_metadata::MetricType", tag = "1")]
    pub r#type: i32,
    #[prost(string, tag = "2")]
    pub metric_family_name: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub help: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub unit: ::prost::alloc::string::String,
}
/// Nested message and enum types in `MetricMetadata`.
pub mod metric_metadata {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum MetricType {
        Unknown = 0,
        Counter = 1,
        Gauge = 2,
        Histogram = 3,
        Gaugehistogram = 4,
        Summary = 5,
        Info = 6,
        Stateset = 7,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Sample {
    #[prost(double, tag = "1")]
    pub value: f64,
    /// timestamp is in ms format, see pkg/timestamp/timestamp.go for
    /// conversion from time.Time to Prometheus timestamp.
    #[prost(int64, tag = "2")]
    pub timestamp: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Exemplar {
    /// Optional, can be empty.
    #[prost(message, repeated, tag = "1")]
    pub labels: ::prost::alloc::vec::Vec<Label>,
    #[prost(double, tag = "2")]
    pub value: f64,
    /// timestamp is in ms format, see pkg/timestamp/timestamp.go for
    /// conversion from time.Time to Prometheus timestamp.
    #[prost(int64, tag = "3")]
    pub timestamp: i64,
}
/// TimeSeries represents samples and labels for a single time series.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TimeSeries {
    /// For a timeseries to be valid, and for the samples and exemplars
    /// to be ingested by the remote system properly, the labels field is required.
    #[prost(message, repeated, tag = "1")]
    pub labels: ::prost::alloc::vec::Vec<Label>,
    #[prost(message, repeated, tag = "2")]
    pub samples: ::prost::alloc::vec::Vec<Sample>,
    #[prost(message, repeated, tag = "3")]
    pub exemplars: ::prost::alloc::vec::Vec<Exemplar>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Label {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub value: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Labels {
    #[prost(message, repeated, tag = "1")]
    pub labels: ::prost::alloc::vec::Vec<Label>,
}
/// Matcher specifies a rule, which can match or set of labels or not.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LabelMatcher {
    #[prost(enumeration = "label_matcher::Type", tag = "1")]
    pub r#type: i32,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub value: ::prost::alloc::string::String,
}
/// Nested message and enum types in `LabelMatcher`.
pub mod label_matcher {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        Eq = 0,
        Neq = 1,
        Re = 2,
        Nre = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadHints {
    /// Query step size in milliseconds.
    #[prost(int64, tag = "1")]
    pub step_ms: i64,
    /// String representation of surrounding function or aggregation.
    #[prost(string, tag = "2")]
    pub func: ::prost::alloc::string::String,
    /// Start time in milliseconds.
    #[prost(int64, tag = "3")]
    pub start_ms: i64,
    /// End time in milliseconds.
    #[prost(int64, tag = "4")]
    pub end_ms: i64,
    /// List of label names used in aggregation.
    #[prost(string, repeated, tag = "5")]
    pub grouping: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Indicate whether it is without or by.
    #[prost(bool, tag = "6")]
    pub by: bool,
    /// Range vector selector range in milliseconds.
    #[prost(int64, tag = "7")]
    pub range_ms: i64,
}
/// Chunk represents a TSDB chunk.
/// Time range [min, max] is inclusive.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Chunk {
    #[prost(int64, tag = "1")]
    pub min_time_ms: i64,
    #[prost(int64, tag = "2")]
    pub max_time_ms: i64,
    #[prost(enumeration = "chunk::Encoding", tag = "3")]
    pub r#type: i32,
    #[prost(bytes = "vec", tag = "4")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// Nested message and enum types in `Chunk`.
pub mod chunk {
    /// We require this to match chunkenc.Encoding.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Encoding {
        Unknown = 0,
        Xor = 1,
    }
}
/// ChunkedSeries represents single, encoded time series.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChunkedSeries {
    /// Labels should be sorted.
    #[prost(message, repeated, tag = "1")]
    pub labels: ::prost::alloc::vec::Vec<Label>,
    /// Chunks will be in start time order and may overlap.
    #[prost(message, repeated, tag = "2")]
    pub chunks: ::prost::alloc::vec::Vec<Chunk>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteRequest {
    #[prost(message, repeated, tag = "1")]
    pub timeseries: ::prost::alloc::vec::Vec<TimeSeries>,
    #[prost(message, repeated, tag = "3")]
    pub metadata: ::prost::alloc::vec::Vec<MetricMetadata>,
}
/// ReadRequest represents a remote read request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadRequest {
    #[prost(message, repeated, tag = "1")]
    pub queries: ::prost::alloc::vec::Vec<Query>,
    /// accepted_response_types allows negotiating the content type of the response.
    ///
    /// Response types are taken from the list in the FIFO order. If no response type in `accepted_response_types` is
    /// implemented by server, error is returned.
    /// For request that do not contain `accepted_response_types` field the SAMPLES response type will be used.
    #[prost(enumeration = "read_request::ResponseType", repeated, tag = "2")]
    pub accepted_response_types: ::prost::alloc::vec::Vec<i32>,
}
/// Nested message and enum types in `ReadRequest`.
pub mod read_request {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ResponseType {
        /// Server will return a single ReadResponse message with matched series that includes list of raw samples.
        /// It's recommended to use streamed response types instead.
        ///
        /// Response headers:
        /// Content-Type: "application/x-protobuf"
        /// Content-Encoding: "snappy"
        Samples = 0,
        /// Server will stream a delimited ChunkedReadResponse message that contains XOR encoded chunks for a single series.
        /// Each message is following varint size and fixed size bigendian uint32 for CRC32 Castagnoli checksum.
        ///
        /// Response headers:
        /// Content-Type: "application/x-streamed-protobuf; proto=prometheus.ChunkedReadResponse"
        /// Content-Encoding: ""
        StreamedXorChunks = 1,
    }
}
/// ReadResponse is a response when response_type equals SAMPLES.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadResponse {
    /// In same order as the request's queries.
    #[prost(message, repeated, tag = "1")]
    pub results: ::prost::alloc::vec::Vec<QueryResult>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Query {
    #[prost(int64, tag = "1")]
    pub start_timestamp_ms: i64,
    #[prost(int64, tag = "2")]
    pub end_timestamp_ms: i64,
    #[prost(message, repeated, tag = "3")]
    pub matchers: ::prost::alloc::vec::Vec<LabelMatcher>,
    #[prost(message, optional, tag = "4")]
    pub hints: ::core::option::Option<ReadHints>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryResult {
    /// Samples within a time series must be ordered by time.
    #[prost(message, repeated, tag = "1")]
    pub timeseries: ::prost::alloc::vec::Vec<TimeSeries>,
}
/// ChunkedReadResponse is a response when response_type equals STREAMED_XOR_CHUNKS.
/// We strictly stream full series after series, optionally split by time. This means that a single frame can contain
/// partition of the single series, but once a new series is started to be streamed it means that no more chunks will
/// be sent for previous one. Series are returned sorted in the same way TSDB block are internally.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChunkedReadResponse {
    #[prost(message, repeated, tag = "1")]
    pub chunked_series: ::prost::alloc::vec::Vec<ChunkedSeries>,
    /// query_index represents an index of the query from ReadRequest.queries these chunks relates to.
    #[prost(int64, tag = "2")]
    pub query_index: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WriteResponse {}
#[doc = r" Generated client implementations."]
pub mod remote_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct RemoteClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl RemoteClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> RemoteClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> RemoteClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            RemoteClient::new(InterceptedService::new(inner, interceptor))
        }
        #[doc = r" Compress requests with `gzip`."]
        #[doc = r""]
        #[doc = r" This requires the server to support it otherwise it might respond with an"]
        #[doc = r" error."]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        #[doc = r" Enable decompressing responses with `gzip`."]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn write(
            &mut self,
            request: impl tonic::IntoRequest<super::WriteRequest>,
        ) -> Result<tonic::Response<super::WriteResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/prometheus.Remote/Write");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
#[doc = r" Generated server implementations."]
pub mod remote_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with RemoteServer."]
    #[async_trait]
    pub trait Remote: Send + Sync + 'static {
        async fn write(
            &self,
            request: tonic::Request<super::WriteRequest>,
        ) -> Result<tonic::Response<super::WriteResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct RemoteServer<T: Remote> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Remote> RemoteServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for RemoteServer<T>
    where
        T: Remote,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/prometheus.Remote/Write" => {
                    #[allow(non_camel_case_types)]
                    struct WriteSvc<T: Remote>(pub Arc<T>);
                    impl<T: Remote> tonic::server::UnaryService<super::WriteRequest> for WriteSvc<T> {
                        type Response = super::WriteResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::WriteRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).write(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = WriteSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: Remote> Clone for RemoteServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Remote> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Remote> tonic::transport::NamedService for RemoteServer<T> {
        const NAME: &'static str = "prometheus.Remote";
    }
}
