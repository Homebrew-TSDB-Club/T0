use crate::{Error, QueryServer};
use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use ql::rosetta::{Aggregation, Projection};
use std::sync::Arc;

impl QueryServer {
    pub(crate) fn compute(
        &self,
        projections: &[Projection],
        aggregations: Option<Aggregation>,
        schema: &Schema,
        chunks: &[Chunk<Arc<dyn Array>>],
    ) -> Result<(), Error> {
        if let Some(aggregations) = aggregations {
            for projection in aggregations.labels {
                let offset = schema
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| field.name == projection)
                    .ok_or_else(|| Error::NoSuchField {
                        name: projection.clone(),
                    })?
                    .0;
                for chunk in chunks {
                    let array = &chunk[offset];
                }
            }
        }

        todo!()
    }

    fn sum(&self) {}
}
