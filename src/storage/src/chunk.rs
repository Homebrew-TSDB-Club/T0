use common::column::{Column, LabelColumnImpl, ScalarColumnImpl};
use common::schema::{ColumnType, Schema};
use common::time::{Duration, Instant};
use common::util::OrderedMap;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct MutableChunk {
    pub(crate) info: Info,
    stat: Stat,
    pub(crate) columns: OrderedMap<Arc<str>, Column>,
}

impl MutableChunk {
    pub(crate) fn new(schema: &Schema, start_at: Instant) -> Self {
        let columns = schema
            .columns
            .iter()
            .map(|column| match &column.column_type {
                ColumnType::Label { data_type } => (
                    Arc::clone(&column.name),
                    Column::Label(LabelColumnImpl::new(data_type, &column.index)),
                ),
                ColumnType::Scalar {
                    data_type,
                    series_len,
                } => (
                    Arc::clone(&column.name),
                    Column::Scalar(ScalarColumnImpl::new(data_type, *series_len)),
                ),
            })
            .collect();
        Self {
            info: Info {
                start_at,
                time_interval: schema.meta.time_interval,
                series_len: schema.meta.series_len,
            },
            stat: Default::default(),
            columns,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Info {
    pub(crate) start_at: Instant,
    time_interval: Duration,
    series_len: u32,
}

impl Info {
    #[inline]
    pub(crate) fn end_at(&self) -> Instant {
        self.start_at + self.time_interval * (self.series_len - 1)
    }
}

#[derive(Debug, Default)]
struct Stat {
    record_num: u32,
}

impl Stat {
    #[inline]
    fn add_num(&mut self, n: u32) -> u32 {
        self.record_num += n;
        self.record_num
    }
}
