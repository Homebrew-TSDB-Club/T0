pub mod error;

use crate::error::Error;
use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::ipc::write::{FileWriter, WriteOptions};
use flat::query::{Language, QueryRequest};
use ql::promql::parse;
use ql::rosetta::Projection;
use std::sync::Arc;
use storage::StorageServer;

#[derive(Debug)]
pub struct QueryServer {
    storage: Arc<StorageServer>,
}

impl QueryServer {
    pub fn new(storage: Arc<StorageServer>) -> Self {
        Self { storage }
    }

    async fn promql(&self, q: &str) -> Result<(Schema, Vec<Chunk<Arc<dyn Array>>>), Error> {
        let expr = parse(q).map_err(|err| Error::ParseError { err })?;
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

    pub async fn query(&self, request: QueryRequest<'_>) -> Result<Vec<u8>, Error> {
        let (schema, chunks) = match request.language() {
            Language::PromQL => {
                let q = request.q();
                self.promql(q).await
            }
            _ => {
                unreachable!()
            }
        }?;

        let mut buffer = Vec::<u8>::new();
        let mut writer = FileWriter::try_new(
            &mut buffer,
            &schema,
            None,
            WriteOptions { compression: None },
        )
        .map_err(|err| Error::InternalError { err })?;
        for chunk in chunks {
            writer.write(&chunk, None).unwrap();
        }
        Ok(buffer)
    }
}

#[cfg(test)]
mod test {
    use crate::QueryServer;
    use arrow2::io::ipc::write::{FileWriter, WriteOptions};
    use common::time::Instant;
    use common::{Label, LabelValue, Scalar, ScalarValue};
    use context::Context;
    use std::sync::Arc;
    use storage::StorageServer;

    #[test]
    fn test_scan() {
        let storage = Arc::new(StorageServer::new(&[0], Arc::new(Context::new())));
        futures_lite::future::block_on(storage.inner_write(
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
        let (schema, chunks) = futures_lite::future::block_on(query.promql("test{}[5m]")).unwrap();
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
