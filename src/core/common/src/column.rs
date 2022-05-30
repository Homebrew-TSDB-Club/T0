use crate::array::{
    Array, ConstFixedSizeListArray, IdListArray, IndexableArray, ListArray,
    NullableFixedSizeListArray, PrimitiveArray,
};
use crate::index::{IndexImpl, IndexType};
use crate::schema::{LabelDataType, ScalarDataType};
use crate::{LabelType, ScalarType};
use std::hash::Hash;

pub type UInt8Scalar = NullableFixedSizeListArray<u8>;
pub type UInt16Scalar = NullableFixedSizeListArray<u16>;
pub type UInt32Scalar = NullableFixedSizeListArray<u32>;
pub type UInt64Scalar = NullableFixedSizeListArray<u64>;
pub type Int8Scalar = NullableFixedSizeListArray<i8>;
pub type Int16Scalar = NullableFixedSizeListArray<i16>;
pub type Int32Scalar = NullableFixedSizeListArray<i32>;
pub type Int64Scalar = NullableFixedSizeListArray<i64>;
pub type Float32Scalar = NullableFixedSizeListArray<f32>;
pub type Float64Scalar = NullableFixedSizeListArray<f64>;
pub type Float128Scalar = NullableFixedSizeListArray<i128>;
pub type BoolScalar = NullableFixedSizeListArray<bool>;

pub type StringLabel = IdListArray<ListArray<u8>>;
pub type IPv4Label = IdListArray<ConstFixedSizeListArray<u8, 4>>;
pub type IPv6Label = IdListArray<ConstFixedSizeListArray<u8, 16>>;
pub type IntLabel = IdListArray<PrimitiveArray<i64>>;
pub type BoolLabel = IdListArray<PrimitiveArray<bool>>;

pub type ScalarArrayImpl = ScalarType<
    UInt8Scalar,
    UInt16Scalar,
    UInt32Scalar,
    UInt64Scalar,
    Int8Scalar,
    Int16Scalar,
    Int32Scalar,
    Int64Scalar,
    Float32Scalar,
    Float64Scalar,
    Float128Scalar,
    BoolScalar,
>;
pub type LabelArrayImpl = LabelType<StringLabel, IPv4Label, IPv6Label, IntLabel, BoolLabel>;

#[derive(Debug)]
pub enum Column {
    Scalar(ScalarColumnImpl),
    Label(LabelColumnImpl),
}

impl Column {
    pub fn get_index(&self, _index_type: IndexType<()>) -> Option<&LabelIndex> {
        match self {
            Column::Scalar(_) => None,
            Column::Label(column) => {
                if let Some(index) = column.index.get(0) {
                    return match index {
                        IndexType::Inverted(_) => Some(index),
                    };
                }
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct ScalarColumnImpl {
    array: ScalarArrayImpl,
}

impl ScalarColumnImpl {
    pub fn new(data_type: &ScalarDataType, series_len: usize) -> Self {
        let array = match data_type {
            ScalarDataType::UInt8(_) => ScalarType::UInt8(UInt8Scalar::new(series_len)),
            ScalarDataType::UInt16(_) => ScalarType::UInt16(UInt16Scalar::new(series_len)),
            ScalarDataType::UInt32(_) => ScalarType::UInt32(UInt32Scalar::new(series_len)),
            ScalarDataType::UInt64(_) => ScalarType::UInt64(UInt64Scalar::new(series_len)),
            ScalarDataType::Int8(_) => ScalarType::Int8(Int8Scalar::new(series_len)),
            ScalarDataType::Int16(_) => ScalarType::Int16(Int16Scalar::new(series_len)),
            ScalarDataType::Int32(_) => ScalarType::Int32(Int32Scalar::new(series_len)),
            ScalarDataType::Int64(_) => ScalarType::Int64(Int64Scalar::new(series_len)),
            ScalarDataType::Float32(_) => ScalarType::Float32(Float32Scalar::new(series_len)),
            ScalarDataType::Float64(_) => ScalarType::Float64(Float64Scalar::new(series_len)),
            ScalarDataType::Float128(_) => ScalarType::Float128(Float128Scalar::new(series_len)),
            ScalarDataType::Bool(_) => ScalarType::Bool(BoolScalar::new(series_len)),
        };
        Self { array }
    }
}

pub type LabelIndex = IndexImpl<usize>;

#[derive(Debug)]
pub struct LabelColumnImpl {
    array: LabelArrayImpl,
    index: Vec<LabelIndex>,
}

impl LabelColumnImpl {
    pub fn new(data_type: &LabelDataType, index: &[IndexType<()>]) -> Self {
        let (array, index) = match data_type {
            LabelDataType::String(_) => {
                let array = StringLabel::new();
                let index = index
                    .iter()
                    .map(|index| array.create_index(index.clone()))
                    .collect();
                (LabelArrayImpl::String(array), index)
            }
            LabelDataType::IPv4(_) => {
                let array = IPv4Label::new();
                let index = index
                    .iter()
                    .map(|index| array.create_index(index.clone()))
                    .collect();
                (LabelArrayImpl::IPv4(array), index)
            }
            LabelDataType::IPv6(_) => {
                let array = IPv6Label::new();
                let index = index
                    .iter()
                    .map(|index| array.create_index(index.clone()))
                    .collect();
                (LabelArrayImpl::IPv6(array), index)
            }
            LabelDataType::Int(_) => {
                let array = IntLabel::new();
                let index = index
                    .iter()
                    .map(|index| array.create_index(index.clone()))
                    .collect();
                (LabelArrayImpl::Int(array), index)
            }
            LabelDataType::Bool(_) => {
                let array = BoolLabel::new();
                let index = index
                    .iter()
                    .map(|index| array.create_index(index.clone()))
                    .collect();
                (LabelArrayImpl::Bool(array), index)
            }
        };
        Self { array, index }
    }
}

pub trait Indexable: IndexableArray {
    fn create_index(&self, data_type: IndexType<()>) -> IndexImpl<Self::ID>
    where
        Self::ID: Hash + Eq,
    {
        IndexImpl::new(data_type)
    }
}

impl<A: Array + Default> Indexable for IdListArray<A> where
    for<'a, 'b> A::ElementRef<'a>: PartialEq<A::ElementRef<'b>> + Hash
{
}
