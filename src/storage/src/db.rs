use crate::chunk::ScanChunk;
use crate::error::{ScanError, WriteError};
use crate::table::Table;
use common::time::Instant;
use common::{Label, Scalar};
use context::Context;
use hashbrown::HashMap;
use ql::rosetta::{Matcher, Range};
use std::sync::Arc;

pub(crate) struct Shard {
    tables: HashMap<Arc<str>, Table>,
    context: Arc<Context>,
}

impl Shard {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        Self {
            tables: HashMap::new(),
            context,
        }
    }

    pub(crate) fn write(
        &mut self,
        table_name: &str,
        labels: &[Label],
        scalars: &[(Instant, Vec<Scalar>)],
    ) -> Result<(), WriteError> {
        let schema = self
            .context
            .get_schema_or_else(table_name, || Context::create_schema(labels, scalars));
        let table_name = Arc::from(table_name);
        let table = self
            .tables
            .entry(Arc::clone(&table_name))
            .or_insert_with(|| Table::new(table_name, schema));
        table.write(labels, scalars)
    }

    pub(crate) async fn scan(
        &self,
        table_name: &str,
        projections: Option<&[String]>,
        filters: &[Matcher<'static>],
        range: Range,
        limit: Option<usize>,
    ) -> Result<Vec<ScanChunk>, ScanError> {
        self.tables
            .get(table_name)
            .ok_or_else(|| ScanError::NoSuchTable {
                name: table_name.to_owned(),
            })?
            .scan(projections, filters, range, limit)
            .await
    }
}
