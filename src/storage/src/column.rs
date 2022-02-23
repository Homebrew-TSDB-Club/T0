use crate::util::dictionary::StringDictionary;
use common::{LabelType, LabelValue, ScalarType};
use croaring::Bitmap;
use hashbrown::HashMap;
use ql::rosetta::MatcherOp;
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Series<G> {
    data: Vec<Option<G>>,
}

impl<G: 'static + Default + Clone + Copy> Series<G> {
    pub(crate) fn new(len: u32) -> Self {
        Self {
            data: vec![Default::default(); len as usize],
        }
    }

    #[inline]
    pub(crate) fn insert(&mut self, index: u32, scalar: G) {
        self.data[index as usize] = Some(scalar);
    }

    #[inline]
    pub(crate) fn range(&self, range: Range<usize>) -> &[Option<G>] {
        &self.data[range]
    }
}

#[derive(Debug)]
pub(crate) struct ScalarColumn {
    name: Arc<str>,
    data: ScalarType<Vec<Series<i64>>, Vec<Series<f64>>>,
    series_len: u32,
}

impl ScalarColumn {
    pub(crate) fn new(column_type: ScalarType<String, String>, series_len: u32) -> Self {
        match column_type {
            ScalarType::Int(name) => Self {
                name: Arc::from(name),
                data: ScalarType::Int(Vec::new()),
                series_len,
            },
            ScalarType::Float(name) => Self {
                name: Arc::from(name),
                data: ScalarType::Float(Vec::new()),
                series_len,
            },
        }
    }

    #[inline]
    pub(crate) fn push_zero(&mut self) {
        match &mut self.data {
            ScalarType::Int(column) => column.push(Series::new(self.series_len)),
            ScalarType::Float(column) => column.push(Series::new(self.series_len)),
        };
    }

    #[inline]
    pub(crate) fn get(&self, offset: u32) -> Option<ScalarType<&Series<i64>, &Series<f64>>> {
        return match &self.data {
            ScalarType::Int(data) => data.get(offset as usize).map(ScalarType::Int),
            ScalarType::Float(data) => data.get(offset as usize).map(ScalarType::Float),
        };
    }

    #[inline]
    pub(crate) fn get_mut(
        &mut self,
        offset: u32,
    ) -> Option<ScalarType<&mut Series<i64>, &mut Series<f64>>> {
        match &mut self.data {
            ScalarType::Int(data) => data.get_mut(offset as usize).map(ScalarType::Int),
            ScalarType::Float(data) => data.get_mut(offset as usize).map(ScalarType::Float),
        }
    }

    #[inline]
    pub(crate) fn name(&self) -> &Arc<str> {
        &self.name
    }

    #[inline]
    pub(crate) fn data_type(&self) -> ScalarType<(), ()> {
        match self.data {
            ScalarType::Int(_) => ScalarType::Int(()),
            ScalarType::Float(_) => ScalarType::Float(()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct StringArray {
    data: Vec<usize>,
    values: StringDictionary,
}

impl StringArray {
    pub(crate) fn new() -> Self {
        Self {
            data: Vec::new(),
            values: StringDictionary::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct LabelColumn {
    name: Arc<str>,
    data: LabelType<StringArray>,
    index: HashMap<usize, Bitmap>,
}

impl LabelColumn {
    pub(crate) fn new(column_type: LabelType<String>) -> Self {
        let mut index = HashMap::new();
        index.insert(0, Bitmap::create());
        match column_type {
            LabelType::String(name) => Self {
                name: Arc::from(name),
                data: LabelType::String(StringArray::new()),
                index,
            },
        }
    }

    pub(crate) fn lookup(
        &self,
        op: MatcherOp,
        value: Option<&LabelType<String>>,
    ) -> Option<&Bitmap> {
        match &self.data {
            LabelType::String(data) => match op {
                MatcherOp::LiteralEqual => {
                    let id = match value {
                        Some(s) => {
                            let LabelType::String(s) = s;
                            data.values.lookup(s)?
                        }
                        None => 0,
                    };
                    self.index.get(&id)
                }
                _ => unimplemented!(),
            },
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, label: &LabelValue) {
        match &mut self.data {
            LabelType::String(data) => match label {
                LabelValue::String(s) => {
                    let id = data.values.lookup_or_insert(s);
                    data.data.push(id);
                    let bitmap = self.index.entry(id).or_insert_with(Bitmap::create);
                    bitmap.add(data.data.len() as u32 - 1);
                }
            },
        }
    }

    #[inline]
    pub(crate) fn push_zero(&mut self) {
        match &mut self.data {
            LabelType::String(data) => {
                data.data.push(0);
                self.index.entry(0).and_modify(|map| {
                    map.add(data.data.len() as u32 - 1);
                });
            }
        }
    }

    #[inline]
    pub(crate) fn get(&self, id: u32) -> Option<LabelType<&str>> {
        match &self.data {
            LabelType::String(data) => {
                let id = *data.data.get(id as usize)?;
                data.values.get(id).map(LabelType::String)
            }
        }
    }

    #[inline]
    pub(crate) fn data_type(&self) -> LabelType<()> {
        match self.data {
            LabelType::String(_) => LabelType::String(()),
        }
    }

    #[inline]
    pub(crate) fn name(&self) -> &Arc<str> {
        &self.name
    }
}

#[cfg(test)]
mod test {
    use crate::column::{LabelColumn, ScalarColumn};
    use common::{LabelType, LabelValue, ScalarType};
    use ql::rosetta::MatcherOp;

    #[test]
    fn test_scalar_column() {
        let mut scalar = ScalarColumn::new(ScalarType::Float(String::from("test")), 1);
        scalar.push_zero();
        let series = scalar.get_mut(0);
        if let Some(ScalarType::Float(series)) = series {
            assert_eq!(series.data.len(), 1);
            series.insert(0, 1.);
            assert_eq!(series.data[0], Some(1.))
        } else {
            unreachable!()
        }
    }

    #[test]
    fn test_label_column() {
        let mut label = LabelColumn::new(LabelType::String(String::from("test")));
        label.push_zero();
        assert!(label.get(0).is_none());
        label.push(&LabelValue::String(String::from("test")));
        assert!(label.get(1).is_some());
    }

    #[test]
    fn test_label_lookup() {
        let mut label = LabelColumn::new(LabelType::String(String::from("test")));
        label.push_zero();
        assert!(label.get(0).is_none());
        label.push(&LabelValue::String(String::from("test")));
        assert!(label.get(1).is_some());
        let id = label.lookup(MatcherOp::LiteralEqual, None);
        assert!(id.is_some());
        assert_eq!(id.map(|id| id.iter().next().unwrap()), Some(0));
        let id = label.lookup(
            MatcherOp::LiteralEqual,
            Some(&LabelType::String(String::from("test"))),
        );
        assert!(id.is_some());
        assert_eq!(id.map(|id| id.iter().next().unwrap()), Some(1));
    }
}
