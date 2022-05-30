use crate::index::IndexType;
use crate::time::Duration;
use crate::{LabelType, ScalarType};
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug)]
pub struct Meta {
    pub series_len: u32,
    pub time_interval: Duration,
    pub mutable_chunk_num: u32,
}

impl Meta {
    #[inline]
    pub fn chunk_duration(&self) -> Duration {
        self.time_interval * self.series_len
    }
}

#[derive(Debug, PartialEq)]
pub struct Column {
    pub column_type: ColumnType,
    pub name: Arc<str>,
    pub index: Vec<IndexType<()>>,
}

pub type LabelDataType = LabelType<(), (), (), (), ()>;
pub type ScalarDataType = ScalarType<(), (), (), (), (), (), (), (), (), (), (), ()>;

#[derive(Debug, PartialEq)]
pub enum ColumnType {
    Label {
        data_type: LabelDataType,
    },
    Scalar {
        data_type: ScalarDataType,
        series_len: usize,
    },
}

#[derive(Debug)]
pub struct Schema {
    pub columns: Vec<Column>,
    pub meta: Meta,
}
