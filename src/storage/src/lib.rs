mod chunk;
mod column;
mod db;
pub mod error;
mod table;
mod util;

use crate::chunk::ScanChunk;
use crate::db::Shard;
use crate::error::{ScanError, WriteError};
use crate::util::{hash_combine, jump_consistent_hash, HashReduce};
use arrow2::datatypes::{DataType, Field, Schema};
use common::time::Instant;
use common::{Label, LabelType, LabelValue, Scalar, ScalarValue};
use context::Context;
use futures::channel::oneshot;
use ql::rosetta::{Matcher, MatcherRef, Range};
use runtime::Runtime;
use std::mem;
use std::sync::Arc;
use tracing::error;

#[derive(Debug)]
struct ScanRequest {
    table_name: String,
    projections: Option<Vec<String>>,
    filters: Vec<Matcher>,
    range: Range,
    limit: Option<usize>,
    ret: async_channel::Sender<Result<Vec<ScanChunk>, ScanError>>,
}

#[derive(Debug)]
enum Request<'a> {
    Write {
        table_name: &'a str,
        labels: Vec<Label<'a>>,
        scalars: Vec<(Instant, Vec<Scalar>)>,
        ret: oneshot::Sender<Result<(), WriteError>>,
    },
    Scan {
        inner: Arc<ScanRequest>,
    },
}

#[derive(Debug)]
pub struct StorageServer {
    runtime: Runtime<Request<'static>>,
    cores: usize,
    context: Arc<Context>,
}

impl StorageServer {
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
                    Request::Write {
                        table_name,
                        labels,
                        scalars,
                        ret,
                    } => {
                        let result =
                            db_shard.write(table_name.as_ref(), labels.as_ref(), scalars.as_ref());
                        ret.send(result).unwrap();
                    }
                    Request::Scan { inner } => {
                        let mut filter_refs = Vec::with_capacity(inner.filters.len());
                        let mut filter_values = Vec::with_capacity(inner.filters.len());
                        for filter in &inner.filters {
                            let value = match &filter.value {
                                None => None,
                                Some(value) => match value {
                                    LabelType::String(s) => Some(LabelType::String(s.as_ref())),
                                },
                            };
                            filter_values.push(value);
                        }

                        for (id, value) in filter_values.iter().enumerate() {
                            let filter = &inner.filters[id];
                            filter_refs.push(MatcherRef {
                                name: &filter.name,
                                op: filter.op,
                                value: value.as_ref(),
                            });
                        }
                        let result = db_shard
                            .scan(
                                &inner.table_name,
                                inner.projections.as_deref(),
                                &filter_refs,
                                inner.range,
                                inner.limit,
                            )
                            .await;
                        if let Err(error) = inner.ret.send(result).await {
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

    pub async fn inner_write(
        &self,
        table_name: &str,
        labels: Vec<Label<'_>>,
        scalars: Vec<(Instant, Vec<Scalar>)>,
    ) -> Result<(), WriteError> {
        let table_name = unsafe { mem::transmute::<&str, &'static str>(table_name) };
        let labels = unsafe { mem::transmute::<Vec<Label<'_>>, Vec<Label<'static>>>(labels) };

        let shard_id = jump_consistent_hash(Self::hash_labels(&labels), self.cores);
        let (ret, ret_recv) = oneshot::channel();
        self.runtime
            .send(
                shard_id as usize,
                Request::Write {
                    table_name,
                    labels,
                    scalars,
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
        table_name: &str,
        projections: Option<Vec<&str>>,
        filters: &[Matcher],
        range: Range,
        limit: Option<usize>,
    ) -> Result<(Schema, Vec<ScanChunk>), ScanError> {
        let schema = self
            .context
            .get_schema(table_name.as_ref())
            .ok_or_else(|| ScanError::NoSuchTable {
                name: table_name.into(),
            })?;
        let mut arrow_fields =
            Vec::with_capacity(schema.label_arrows.len() + schema.scalar_arrows.len() + 1);
        arrow_fields.push(Field::new("start_at", DataType::Int64, false));
        arrow_fields.extend_from_slice(&schema.label_arrows);
        match projections {
            None => arrow_fields.extend_from_slice(&schema.scalar_arrows),
            Some(ref projections) => {
                for projection in projections {
                    arrow_fields.push(
                        schema.scalar_arrows[schema.scalars.get_id(*projection).ok_or_else(
                            || ScanError::NoSuchScalar {
                                name: projection.to_string(),
                            },
                        )?]
                        .clone(),
                    );
                }
            }
        }

        let (ret, ret_recv) = async_channel::bounded(self.cores);
        let request = Arc::new(ScanRequest {
            table_name: table_name.into(),
            projections: projections.map(|ps| ps.iter().map(|p| p.to_string()).collect()),
            filters: filters.to_vec(),
            range,
            limit,
            ret,
        });
        for shard_id in 0..self.cores {
            self.runtime
                .send(
                    shard_id,
                    Request::Scan {
                        inner: Arc::clone(&request),
                    },
                )
                .await
                .unwrap();
        }

        let mut chunks = Vec::new();
        for _ in 0..self.cores {
            chunks.append(&mut ret_recv.recv().await.unwrap()?);
        }
        ret_recv.close();

        let mut arrow_schema = Schema::from(arrow_fields);
        arrow_schema.metadata.insert(
            String::from("time_interval"),
            schema.meta.time_interval.as_millis().to_string(),
        );
        Ok((arrow_schema, chunks))
    }

    pub async fn write(&self, request: flat::write::WriteRequest<'_>) {
        for timeseries in request.timeseries() {
            let mut name = None;
            let mut labels = Vec::with_capacity(timeseries.labels().len() - 1);

            for label in timeseries.labels() {
                if label.name() == "__name__" {
                    name = Some(label.value());
                } else {
                    labels.push(Label {
                        name: label.name(),
                        value: LabelValue::String(label.value()),
                    })
                }
            }

            let name = name.unwrap();

            let scalars = timeseries
                .samples()
                .iter()
                .map(|sample| {
                    let timestamp = Instant::from_millis(sample.timestamp());
                    let scalars = vec![Scalar {
                        name: String::from("value"),
                        value: ScalarValue::Float(sample.value()),
                    }];
                    (timestamp, scalars)
                })
                .collect();

            let result = self.inner_write(name, labels, scalars).await;
            if let Err(error) = result {
                error!("timeseries write error: {:?}", error);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::StorageServer;
    use common::time::Instant;
    use common::{Label, LabelValue, Scalar, ScalarValue};
    use context::Context;
    use ql::rosetta::Range;
    use std::sync::Arc;

    #[test]
    fn storage_scan() {
        let storage = StorageServer::new(&[0], Arc::new(Context::new()));
        let labels = vec![Label {
            name: "label1",
            value: LabelValue::String("value1"),
        }];
        let scalars = vec![Scalar {
            name: String::from("scalar1"),
            value: ScalarValue::Float(1.0),
        }];
        futures_lite::future::block_on(storage.inner_write(
            "test",
            labels,
            vec![(Instant::now(), scalars)],
        ))
        .unwrap();

        let result = futures_lite::future::block_on(storage.scan(
            "test",
            None,
            &[],
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
