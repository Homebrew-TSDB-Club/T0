use arrow2::datatypes::{DataType, Field};
use common::time::{Duration, Instant};
use common::util::IndexMap;
use common::{Label, LabelType, LabelValue, Scalar, ScalarType, ScalarValue};
use dashmap::DashMap;
use std::sync::Arc;

pub const DEFAULT: TableMeta = TableMeta {
    series_len: 120,
    time_interval: Duration::SECOND,
    mutable_chunk_num: 5,
};

#[derive(Debug)]
pub struct TableMeta {
    pub series_len: u32,
    pub time_interval: Duration,
    pub mutable_chunk_num: u32,
}

impl TableMeta {
    #[inline]
    pub fn chunk_duration(&self) -> Duration {
        self.time_interval * self.series_len
    }
}

#[derive(Debug)]
pub struct Schema {
    pub labels: Vec<LabelType<String>>,
    pub scalars: IndexMap<String, ScalarType<String, String>>,
    pub label_arrows: Vec<Field>,
    pub scalar_arrows: Vec<Field>,
    pub meta: TableMeta,
}

#[derive(Debug, Default)]
pub struct Context {
    schemas: DashMap<String, Arc<Schema>>,
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_schema_or_else<F>(&self, name: &str, f: F) -> Arc<Schema>
    where
        F: FnOnce() -> Schema,
    {
        Arc::clone(
            &*self
                .schemas
                .entry(String::from(name))
                .or_insert_with(|| Arc::new(f())),
        )
    }

    pub fn get_schema(&self, name: &str) -> Option<Arc<Schema>> {
        self.schemas.get(name).map(|s| Arc::clone(s.value()))
    }

    pub fn create_schema(labels: &[Label], scalars: &[(Instant, Vec<Scalar>)]) -> Schema {
        let mut label_columns = Vec::new();
        let mut label_arrows = Vec::new();
        for label in labels {
            match label.value {
                LabelValue::String(_) => {
                    label_columns.push(LabelType::String(label.name.to_owned()));
                    label_arrows.push(Field::new(label.name.to_owned(), DataType::Utf8, true));
                }
            }
        }

        let mut scalar_columns = IndexMap::new();
        let mut scalar_arrows = Vec::new();
        for scalar in &scalars[0].1 {
            let (column, arrow) = match scalar.value {
                ScalarValue::Int(_) => (ScalarType::Int(scalar.name.to_owned()), DataType::Int64),
                ScalarValue::Float(_) => {
                    (ScalarType::Float(scalar.name.to_owned()), DataType::Float64)
                }
            };
            scalar_columns.insert(scalar.name.to_owned(), column);
            scalar_arrows.push(Field::new(
                scalar.name.to_owned(),
                DataType::List(Box::new(Field::new("item", arrow, true))),
                false,
            ));
        }
        Schema {
            labels: label_columns,
            scalars: scalar_columns,
            label_arrows,
            scalar_arrows,
            meta: DEFAULT,
        }
    }
}
