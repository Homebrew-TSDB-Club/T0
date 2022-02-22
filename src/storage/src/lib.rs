mod chunk;
mod column;
mod db;
pub mod error;
mod table;
mod util;

use crate::chunk::ScanChunk;
use crate::db::Shard;
use crate::error::{ScanError, WriteError};
use crate::util::{hash_combine, HashReduce};
use arrow2::datatypes::{DataType, Field, Schema};
use async_trait::async_trait;
use common::time::Instant;
use common::{Label, LabelValue, Scalar, ScalarValue};
use context::Context;
use futures::channel::oneshot;
use proto::prometheus::remote_server::Remote;
use proto::prometheus::{WriteRequest as PromWriteRequest, WriteResponse};
use proto::{Request as GrpcRequest, Response as GrpcResponse, Status};
use ql::rosetta::{Matcher, Range};
use runtime::Runtime;
use std::sync::Arc;
use tracing::{debug, error};

#[derive(Debug)]
struct WriteRequest {
    table_name: Arc<str>,
    labels: Arc<Vec<Label>>,
    scalars: Vec<(Instant, Vec<Scalar>)>,
}

#[derive(Debug)]
struct ScanRequest {
    table_name: String,
    projections: Option<Vec<String>>,
    filters: Vec<Matcher>,
    range: Range,
    limit: Option<usize>,
}

#[derive(Debug)]
enum Request {
    Write {
        inner: Arc<WriteRequest>,
        ret: oneshot::Sender<Result<(), WriteError>>,
    },
    Scan {
        inner: Arc<ScanRequest>,
        ret: oneshot::Sender<Result<Vec<ScanChunk>, ScanError>>,
    },
}

#[derive(Debug)]
pub struct Storage {
    runtime: Runtime<Request>,
    cores: usize,
    context: Arc<Context>,
}

impl Storage {
    pub fn new(cores: &[usize], context: Arc<Context>) -> Self {
        let mut storage = Self {
            runtime: Runtime::new(cores).unwrap(),
            cores: cores.len(),
            context: Arc::clone(&context),
        };
        storage.runtime.run(move |_, recv| async move {
            let mut db_shard = Shard::new(Arc::clone(&context));
            while let Ok(request) = recv.recv().await {
                match request {
                    Request::Write { inner, ret } => {
                        let request = inner.as_ref();
                        let result = db_shard.write(
                            request.table_name.as_ref(),
                            request.labels.as_ref(),
                            request.scalars.as_ref(),
                        );
                        ret.send(result).unwrap();
                    }
                    Request::Scan { inner, ret } => {
                        let request = inner.as_ref();
                        let result = db_shard
                            .scan(
                                &request.table_name,
                                request.projections.as_deref(),
                                &request.filters,
                                request.range,
                                request.limit,
                            )
                            .await;
                        if let Err(error) = ret.send(result) {
                            error!("storage send response error: {:?}", error)
                        }
                    }
                }
            }
        });
        storage
    }

    fn hash_labels(labels: &[Label]) -> u64 {
        let mut label_vec = labels.iter().collect::<Vec<_>>();
        label_vec.sort_by_key(|label| &label.name);
        let mut hashes = label_vec.into_iter().map(|label| {
            let value_hash = match &label.value {
                LabelValue::String(value) => fxhash::hash64(value),
            };
            hash_combine(fxhash::hash64(&label.name), value_hash)
        });
        let mut hr = HashReduce::new(hashes.next().unwrap());
        for hash in hashes {
            hr.add(hash);
        }
        hr.finish()
    }

    pub async fn write(
        &self,
        table_name: Arc<str>,
        labels: Arc<Vec<Label>>,
        scalars: Vec<(Instant, Vec<Scalar>)>,
    ) -> Result<(), WriteError> {
        let shard_id = Self::hash_labels(&labels) as usize % self.cores;

        let (ret, ret_recv) = oneshot::channel();
        self.runtime
            .send(
                shard_id,
                Request::Write {
                    inner: Arc::new(WriteRequest {
                        table_name,
                        labels,
                        scalars,
                    }),
                    ret,
                },
            )
            .await
            .unwrap();
        ret_recv.await.unwrap()
    }

    #[tracing::instrument]
    pub async fn scan(
        &self,
        table_name: String,
        projections: Option<Vec<String>>,
        filters: Vec<Matcher>,
        range: Range,
        limit: Option<usize>,
    ) -> Result<(Schema, Vec<ScanChunk>), ScanError> {
        let schema = self
            .context
            .get_schema(table_name.as_ref())
            .ok_or_else(|| ScanError::NoSuchTable {
                name: table_name.clone(),
            })?;
        let mut arrow_fields =
            Vec::with_capacity(schema.label_arrows.len() + schema.scalar_arrows.len() + 1);
        arrow_fields.push(Field::new("metadata", DataType::Utf8, false));
        arrow_fields.extend_from_slice(&schema.label_arrows);
        match &projections {
            None => arrow_fields.extend_from_slice(&schema.scalar_arrows),
            Some(projections) => {
                for projection in projections {
                    arrow_fields.push(
                        schema.scalar_arrows[schema.scalars.get_id(projection).ok_or_else(
                            || ScanError::NoSuchScalar {
                                name: projection.to_owned(),
                            },
                        )?]
                        .clone(),
                    );
                }
            }
        }

        let mut tasks = Vec::with_capacity(self.cores);
        let request = Arc::new(ScanRequest {
            table_name,
            projections,
            filters,
            range,
            limit,
        });
        for shard_id in 0..self.cores {
            let (ret, ret_recv) = oneshot::channel();
            self.runtime
                .send(
                    shard_id,
                    Request::Scan {
                        inner: Arc::clone(&request),
                        ret,
                    },
                )
                .await
                .unwrap();
            tasks.push(ret_recv);
        }

        let mut chunks = Vec::new();
        for task in tasks {
            chunks.append(&mut task.await.unwrap()?);
        }

        Ok((Schema::from(arrow_fields), chunks))
    }

    pub fn to_grpc_server(self: Arc<Self>) -> StorageServer {
        StorageServer {
            inner: Arc::clone(&self),
        }
    }
}

#[derive(Debug)]
pub struct StorageServer {
    inner: Arc<Storage>,
}

impl StorageServer {
    pub async fn grpc_write(&self, request: PromWriteRequest) {
        for timeseries in request.timeseries {
            let mut name = None;
            let mut labels = Vec::with_capacity(timeseries.labels.len() - 1);

            for label in timeseries.labels {
                if label.name == "__name__" {
                    name = Some(label.value);
                } else {
                    labels.push(Label {
                        name: label.name,
                        value: LabelValue::String(label.value),
                    })
                }
            }

            let name = Arc::from(name.unwrap().as_ref());
            let labels = Arc::new(labels);

            let scalars = timeseries
                .samples
                .into_iter()
                .map(|sample| {
                    let timestamp = Instant::from_millis(sample.timestamp);
                    let scalars = vec![Scalar {
                        name: String::from("value"),
                        value: ScalarValue::Float(sample.value),
                    }];
                    (timestamp, scalars)
                })
                .collect();

            let result = self
                .inner
                .write(Arc::clone(&name), Arc::clone(&labels), scalars)
                .await;
            if let Err(error) = result {
                error!("timeseries write error: {:?}", error);
            }
        }
    }
}

#[async_trait]
impl Remote for StorageServer {
    async fn write(
        &self,
        request: GrpcRequest<PromWriteRequest>,
    ) -> Result<GrpcResponse<WriteResponse>, Status> {
        debug!("request start");
        self.grpc_write(request.into_inner()).await;
        Ok(GrpcResponse::new(WriteResponse {}))
    }
}

#[cfg(test)]
mod test {
    use crate::Storage;
    use common::time::Instant;
    use common::{Label, LabelValue, Scalar, ScalarValue};
    use context::Context;
    use ql::rosetta::Range;
    use std::sync::Arc;

    #[test]
    fn storage_scan() {
        let storage = Storage::new(&[0], Arc::new(Context::new()));
        let labels = vec![Label {
            name: String::from("label1"),
            value: LabelValue::String(String::from("value1")),
        }];
        let scalars = vec![Scalar {
            name: String::from("scalar1"),
            value: ScalarValue::Float(1.0),
        }];
        futures_lite::future::block_on(storage.write(
            Arc::from("test"),
            Arc::new(labels),
            vec![(Instant::now(), scalars)],
        ))
        .unwrap();

        let result = futures_lite::future::block_on(storage.scan(
            String::from("test"),
            None,
            vec![],
            Range {
                start: None,
                end: None,
            },
            None,
        ))
        .unwrap();
        println!("{:?}", result);
    }
}
