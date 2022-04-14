pub mod error;
mod function;

use crate::error::Error;
use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::ipc::write::{FileWriter, WriteOptions};
use flat::query::{Language, QueryRequest};
use ql::promql::parse;
use ql::rosetta::{Expr, Projection};
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

    async fn storage_scan(
        &self,
        expr: &Expr,
    ) -> Result<(Schema, Vec<Chunk<Arc<dyn Array>>>), Error> {
        let mut projections = Vec::new();
        for projection in &expr.projection {
            projections.push(projection.name.as_ref())
        }
        let (schema, chunks) = self
            .storage
            .scan(
                &expr.resource.resource,
                Some(projections),
                &expr.filters,
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
        let expr = match request.language() {
            Language::PromQL => {
                let q = request.q();
                parse(q).map_err(|err| Error::ParseError { err })
            }
            _ => {
                unreachable!()
            }
        }?;

        let (schema, chunks) = self.storage_scan(&expr).await?;
        self.compute(&expr.projection, &expr.aggregation, &schema, &chunks);

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
    use ql::promql::parse;
    use std::sync::Arc;
    use storage::StorageServer;

    #[test]
    fn test_scan() {
        let storage = Arc::new(StorageServer::new(&[0], Arc::new(Context::new())));
        futures_lite::future::block_on(storage.inner_write(
            "test",
            vec![Label {
                name: "label1",
                value: LabelValue::String("value1"),
            }],
            vec![(
                Instant::now(),
                vec![Scalar {
                    name: String::from("value"),
                    value: ScalarValue::Float(1.0),
                }],
            )],
        ))
        .unwrap();
        futures_lite::future::block_on(storage.inner_write(
            "test",
            vec![Label {
                name: "label1",
                value: LabelValue::String("value2"),
            }],
            vec![(
                Instant::now(),
                vec![Scalar {
                    name: String::from("value"),
                    value: ScalarValue::Float(1.0),
                }],
            )],
        ))
        .unwrap();
        let query = QueryServer::new(Arc::clone(&storage));
        let expr = parse("test{}[5m]").unwrap();
        let (schema, chunks) = futures_lite::future::block_on(query.storage_scan(&expr)).unwrap();
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
