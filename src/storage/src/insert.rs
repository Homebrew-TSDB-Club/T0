use crate::chunk::MutableChunk;
use crate::error::ScanError;
use common::index::IndexType;

#[derive(Debug)]
pub struct Filter {
    column_id: usize,
    value: String,
    use_index: Option<IndexType<()>>,
}

impl MutableChunk {
    fn lookup_or_insert(&mut self, filters: &[Filter]) -> Result<usize, ScanError> {
        let mut filtered = None;
        for filter in filters {
            let column = &self.columns[filter.column_id];
            if let Some(index) = &filter.use_index {
                let index = column.get_index(index.clone()).ok_or_else(|| {
                    ScanError::NoSuchColumnIndex {
                        id: filter.column_id,
                        index: index.clone(),
                    }
                })?;
            }
        }
        todo!()
    }
}
