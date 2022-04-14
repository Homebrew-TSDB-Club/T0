use crate::column::{LabelColumn, ScalarColumn};
use crate::error::{ScanError, WriteError};
use arrow2::array::{
    Array, MutableArray, MutableListArray, MutablePrimitiveArray, MutableUtf8Array, PrimitiveArray,
    TryPush,
};
use arrow2::chunk::Chunk;
use common::time::{Duration, Instant};
use common::util::IndexMap;
use common::{Label, LabelType, LabelValue, Scalar, ScalarType};
use context::Schema;
use croaring::Bitmap;
use ql::rosetta::{MatcherOp, MatcherRef, Range};
use std::sync::Arc;

#[derive(Debug)]
pub struct ScanChunk {
    pub start_at: Instant,
    pub time_interval: Duration,
    pub labels: IndexMap<Arc<str>, Arc<dyn Array>>,
    pub scalars: IndexMap<Arc<str>, Arc<dyn Array>>,
}

impl ScanChunk {
    fn new(start_at: Instant, time_interval: Duration) -> Self {
        Self {
            start_at,
            time_interval,
            labels: IndexMap::new(),
            scalars: IndexMap::new(),
        }
    }

    pub fn into_arrow_chunk(self) -> Chunk<Arc<dyn Array>> {
        let mut arrays =
            Vec::<Arc<dyn Array>>::with_capacity(self.labels.len() + self.scalars.len() + 1);
        let len = self.labels.first().map(|c| c.len()).unwrap_or(0);
        arrays.push(Arc::new(PrimitiveArray::<i64>::from(vec![
            Some(
                self.start_at.as_millis()
            );
            len
        ])));
        for label in self.labels {
            arrays.push(label);
        }
        for scalar in self.scalars {
            arrays.push(scalar);
        }
        Chunk::new(arrays)
    }
}

#[derive(Debug)]
pub(crate) struct MutableChunk {
    pub(crate) info: Info,
    stat: Stat,
    columns: Columns,
}

impl MutableChunk {
    pub(crate) fn new(schema: Arc<Schema>, start_at: Instant) -> Self {
        Self {
            info: Info {
                start_at,
                time_interval: schema.meta.time_interval,
                series_len: schema.meta.series_len,
            },
            stat: Default::default(),
            columns: Columns::new(schema),
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        labels: &[Option<&LabelValue>],
    ) -> Result<Option<Row>, WriteError> {
        let mut filtered = None;
        for (id, label) in labels.iter().enumerate() {
            self.columns
                .lookup(
                    &MatcherRef {
                        name: self.columns.labels[id].name().as_ref(),
                        op: MatcherOp::LiteralEqual,
                        value: label.as_deref(),
                    },
                    &mut filtered,
                )
                .map_err(|err| WriteError::InternalError {
                    err: format!("{:?}", err),
                })?;
            if let Some(filtered) = &filtered {
                if filtered.is_empty() {
                    return Ok(None);
                }
            }
        }
        Ok(filtered
            .and_then(|ids| ids.iter().next())
            .map(|id| Row::new(self, id)))
    }

    pub(crate) fn push(&mut self, labels: &[Option<&LabelValue>]) -> Row {
        for (offset, column) in self.columns.labels.iter_mut().enumerate() {
            let label = &labels[offset];
            match label {
                None => column.push_zero(),
                Some(value) => column.push(value),
            }
        }
        for column in self.columns.scalars.iter_mut() {
            column.push_zero();
        }
        let id = self.stat.add_num(1);
        Row::new(self, id - 1)
    }

    pub(crate) fn align_labels<'a>(
        &self,
        labels: &'a [Label],
    ) -> Vec<Option<&'a LabelType<&'a str>>> {
        let mut aligned_labels = vec![None; self.columns.labels.len()];
        for label in labels {
            match self.columns.labels.get_id(label.name) {
                None => {
                    unimplemented!()
                }
                Some(id) => {
                    aligned_labels[id] = Some(&label.value);
                }
            };
        }
        aligned_labels
    }

    pub(crate) async fn scan(
        &self,
        projections: Option<&[String]>,
        filters: &[MatcherRef<'_>],
        range: Range,
        _limit: Option<usize>,
    ) -> Result<Option<ScanChunk>, ScanError> {
        let mut filtered = Some(Bitmap::from_iter(0..self.stat.record_num));
        for filter in filters {
            self.columns.lookup(filter, &mut filtered)?;
        }
        match filtered {
            None => Ok(None),
            Some(ids) => {
                let mut chunk = ScanChunk::new(self.info.start_at, self.info.time_interval);
                self.push_arrow_labels(&ids, &mut chunk);
                self.push_arrow_scalars(projections, range, &ids, &mut chunk);
                Ok(Some(chunk))
            }
        }
    }

    fn push_arrow_labels(&self, ids: &Bitmap, chunk: &mut ScanChunk) {
        for column in self.columns.labels.iter() {
            let mut array = match column.data_type() {
                LabelType::String(_) => MutableUtf8Array::<i32>::new(),
            };
            for id in ids.iter() {
                match column.get(id) {
                    None => array.push_null(),
                    Some(value) => match value {
                        LabelType::String(s) => array.push(Some(s)),
                    },
                }
            }
            chunk
                .labels
                .insert(Arc::clone(column.name()), array.into_arc());
        }
    }

    fn get_projection_id(&self, projections: Option<&[String]>) -> Vec<Option<usize>> {
        match projections {
            None => (0..self.columns.scalars.len()).map(Some).collect(),
            Some(projections) => projections
                .iter()
                .map(|column_name| self.columns.scalars.get_id(column_name.as_str()))
                .collect(),
        }
    }

    fn push_scalar_column(
        &self,
        column_id: usize,
        range: Range,
        ids: &Bitmap,
        chunk: &mut ScanChunk,
    ) {
        let range = self.get_range_offset(range);
        let column = &self.columns.scalars[column_id];
        match column.data_type() {
            ScalarType::Int(_) => {
                let mut array = MutableListArray::<i32, MutablePrimitiveArray<i64>>::new();
                for id in ids.iter() {
                    let series = column.get(id);
                    array
                        .try_push(series.as_ref().map(|series| match series {
                            ScalarType::Int(series) => series.range(range.clone()).iter().cloned(),
                            _ => unreachable!(),
                        }))
                        .unwrap();
                }
                chunk
                    .scalars
                    .insert(Arc::clone(column.name()), array.into_arc());
            }
            ScalarType::Float(_) => {
                let mut array = MutableListArray::<i32, MutablePrimitiveArray<f64>>::new();
                for id in ids.iter() {
                    let series = column.get(id);
                    array
                        .try_push(series.as_ref().map(|series| match series {
                            ScalarType::Float(series) => {
                                series.range(range.clone()).iter().cloned()
                            }
                            _ => unreachable!(),
                        }))
                        .unwrap();
                }
                chunk
                    .scalars
                    .insert(Arc::clone(column.name()), array.into_arc());
            }
        }
    }

    fn get_range_offset(&self, range: Range) -> std::ops::Range<usize> {
        let start = match range.start {
            None => 0,
            Some(start) => {
                let offset = (start - self.info.start_at) / self.info.time_interval;
                if offset <= 0 {
                    0
                } else {
                    offset as usize
                }
            }
        };
        let series_len = self.info.series_len as usize;
        let end = match range.end {
            None => series_len,
            Some(end) => {
                let offset = ((end - self.info.start_at) / self.info.time_interval) as usize;
                if offset >= series_len {
                    series_len
                } else {
                    offset
                }
            }
        };
        start..end
    }

    fn push_arrow_scalars(
        &self,
        projections: Option<&[String]>,
        range: Range,
        ids: &Bitmap,
        chunk: &mut ScanChunk,
    ) {
        let project_ids = self.get_projection_id(projections);
        for (offset, projection) in project_ids.into_iter().enumerate() {
            match projection {
                None => {
                    let mut array = MutableListArray::<i32, MutablePrimitiveArray<f64>>::new();
                    array
                        .try_push(Some((0..ids.cardinality()).map(|_| None)))
                        .unwrap();
                    chunk.scalars.insert(
                        Arc::from(projections.unwrap()[offset].as_ref()),
                        array.into_arc(),
                    );
                }
                Some(column_id) => self.push_scalar_column(column_id, range, ids, chunk),
            }
        }
    }

    pub(crate) async fn archive(self) {
        runtime::yield_now().await;
        // todo
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

#[derive(Debug)]
struct Columns {
    labels: IndexMap<Arc<str>, LabelColumn>,
    scalars: IndexMap<Arc<str>, ScalarColumn>,
}

impl Columns {
    fn new(schema: Arc<Schema>) -> Self {
        let mut labels = IndexMap::new();
        let mut scalars = IndexMap::new();

        for label_type in &schema.labels {
            let column = LabelColumn::new(label_type.clone());
            let name = Arc::clone(column.name());
            labels.insert(name, column);
        }

        for scalar_type in schema.scalars.iter() {
            let column = ScalarColumn::new(scalar_type.clone(), schema.meta.series_len);
            let name = Arc::clone(column.name());
            scalars.insert(name, column);
        }

        Self { labels, scalars }
    }

    pub(crate) fn lookup(
        &self,
        matcher: &MatcherRef,
        superset: &mut Option<Bitmap>,
    ) -> Result<(), ScanError> {
        let ids = self
            .labels
            .get(matcher.name)
            .ok_or_else(|| ScanError::NoSuchLabel {
                name: matcher.name.into(),
            })?
            .lookup(matcher.op, matcher.value);
        match ids {
            None => {
                if let Some(superset) = superset {
                    superset.clear();
                }
            }
            Some(ids) => match superset {
                Some(superset) => {
                    *superset = superset.and(ids);
                }
                None => {
                    *superset = Some(ids.clone());
                }
            },
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Row<'a> {
    chunk: &'a mut MutableChunk,
    id: u32,
}

impl<'a> Row<'a> {
    pub(crate) fn new(chunk: &'a mut MutableChunk, id: u32) -> Self {
        Self { id, chunk }
    }

    pub fn insert(&mut self, timestamp: Instant, scalars: &[Scalar]) {
        let offset = (timestamp - self.chunk.info.start_at) / self.chunk.info.time_interval;
        for scalar in scalars {
            let column = self.chunk.columns.scalars.get_mut(scalar.name.as_str());
            match column {
                None => {
                    unimplemented!()
                }
                Some(column) => {
                    let series = column.get_mut(self.id).unwrap();
                    match series {
                        ScalarType::Int(series) => {
                            series.insert(offset as u32, scalar.value.into());
                        }
                        ScalarType::Float(series) => {
                            series.insert(offset as u32, scalar.value.into());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::chunk::MutableChunk;
    use common::time::Instant;
    use common::util::IndexMap;
    use common::{LabelType, Scalar, ScalarType, ScalarValue};
    use context::{Schema, DEFAULT};
    use ql::rosetta::{MatcherOp, MatcherRef, Range};
    use std::sync::Arc;

    #[test]
    fn new_chunk() {
        let schema = Arc::new(Schema {
            labels: vec![],
            scalars: IndexMap::new(),
            label_arrows: vec![],
            scalar_arrows: vec![],
            meta: DEFAULT,
        });
        MutableChunk::new(schema, Instant::now());
    }

    #[test]
    fn chunk_insert() {
        let mut scalars = IndexMap::new();
        scalars.insert(
            String::from("test2"),
            ScalarType::Int(String::from("test2")),
        );
        let schema = Arc::new(Schema {
            labels: vec![LabelType::String(String::from("test1"))],
            scalars,
            label_arrows: vec![],
            scalar_arrows: vec![],
            meta: DEFAULT,
        });
        let now = Instant::now();
        let mut chunk = MutableChunk::new(schema, now);
        let empty = chunk.align_labels(&[]);
        assert!(chunk.get_mut(&empty).unwrap().is_none());
        let mut row = chunk.push(&empty);
        row.insert(
            now,
            &[Scalar {
                name: String::from("test2"),
                value: ScalarValue::Int(1),
            }],
        );
        assert!(chunk.get_mut(&empty).unwrap().is_some());
        println!("{:?}", chunk);

        futures::executor::block_on(async move {
            let projections = Some(vec![String::from("test2")]);
            let filters = vec![MatcherRef {
                name: "test1",
                op: MatcherOp::LiteralEqual,
                value: None,
            }];
            let range = Range {
                start: None,
                end: None,
            };
            let _limit = None;
            let res = chunk
                .scan(projections.as_deref(), &filters, range, _limit)
                .await;
            println!("{:?}", res);
        });
    }
}
