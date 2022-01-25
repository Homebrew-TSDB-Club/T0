pub mod error;

use crate::error::Error;
use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::ipc::write::{FileWriter, WriteOptions};
use async_trait::async_trait;
use proto::query::query_server::Query as QueryProto;
use proto::query::{PromQlQuery, PromQlResponse};
use proto::{Request, Response, Status};
use ql::promql::parse;
use ql::rosetta::Projection;
use std::sync::Arc;
use storage::Storage;

#[derive(Debug)]
pub struct Query {
    storage: Arc<Storage>,
}

impl Query {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    async fn prom_query(
        &self,
        promql: &str,
    ) -> Result<(Schema, Vec<Chunk<Arc<dyn Array>>>), Error> {
        let expr = parse(promql).map_err(|err| Error::ParseError { err })?;
        let mut projections = Vec::new();
        for projection in expr.projection {
            if let Projection::Specific { name, .. } = projection {
                projections.push(name)
            }
        }
        let (schema, chunks) = self
            .storage
            .scan(
                expr.resource.resource,
                Some(projections),
                expr.filters,
                expr.range,
                None,
            )
            .await
            .map_err(|err| Error::StorageError { err })?;
        let chunks = chunks
            .into_iter()
            .map(|chunk| chunk.into_arrow_chunk())
            .collect();
        Ok((schema, chunks))
    }
}

#[async_trait]
impl QueryProto for Query {
    async fn prom_ql(
        &self,
        request: Request<PromQlQuery>,
    ) -> Result<Response<PromQlResponse>, Status> {
        let promql = request.into_inner();
        let (schema, chunks) = self
            .prom_query(&promql.query)
            .await
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        let mut buffer = Vec::<u8>::new();
        let mut writer = FileWriter::try_new(
            &mut buffer,
            &schema,
            None,
            WriteOptions { compression: None },
        )
        .map_err(|err| Status::internal(format!("{:?}", err)))?;
        for chunk in chunks {
            writer.write(&chunk, None).unwrap();
        }
        Ok(Response::new(PromQlResponse { arrows: buffer }))
    }
}

#[cfg(test)]
mod test {
    use crate::Query;
    use arrow2::io::ipc::write::{FileWriter, WriteOptions};
    use common::time::Instant;
    use common::{Label, LabelValue, Scalar, ScalarValue};
    use context::Context;
    use std::sync::Arc;
    use storage::Storage;

    #[test]
    fn test_scan() {
        let storage = Arc::new(Storage::new(&[0], Arc::new(Context::new())));
        futures_lite::future::block_on(storage.write(
            String::from("test"),
            Instant::now(),
            vec![Label {
                name: String::from("label1"),
                value: LabelValue::String(String::from("value1")),
            }],
            vec![Scalar {
                name: String::from("value"),
                value: ScalarValue::Float(1.0),
            }],
        ))
        .unwrap();
        futures_lite::future::block_on(storage.write(
            String::from("test"),
            Instant::now(),
            vec![Label {
                name: String::from("label1"),
                value: LabelValue::String(String::from("value2")),
            }],
            vec![Scalar {
                name: String::from("value"),
                value: ScalarValue::Float(1.0),
            }],
        ))
        .unwrap();
        let query = Query::new(Arc::clone(&storage));
        let (schema, chunks) =
            futures_lite::future::block_on(query.prom_query("test{}[5m]")).unwrap();
        println!("{:?}, {:?}", schema, chunks);
        let mut buffer = Vec::<u8>::new();
        let mut writer = FileWriter::try_new(
            &mut buffer,
            &schema,
            None,
            WriteOptions { compression: None },
        )
        .unwrap();
        for chunk in &chunks {
            writer.write(chunk, None).unwrap();
        }
        println!("{:?}", buffer.len());
        println!("{:?}", buffer);
    }
}
