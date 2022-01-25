#![feature(const_fn_trait_bound)]

use num_traits::AsPrimitive;

pub mod time;
pub mod util;

#[derive(Debug)]
pub enum ScalarType<I, F> {
    Int(I),
    Float(F),
}

impl<I: Clone, F: Clone> Clone for ScalarType<I, F> {
    fn clone(&self) -> Self {
        match &self {
            ScalarType::Int(s) => ScalarType::Int(s.clone()),
            ScalarType::Float(s) => ScalarType::Float(s.clone()),
        }
    }
}

impl<I: Copy, F: Copy> Copy for ScalarType<I, F> {}

impl<I: AsPrimitive<i64>, F: AsPrimitive<i64>> From<ScalarType<I, F>> for i64 {
    fn from(s: ScalarType<I, F>) -> Self {
        match s {
            ScalarType::Int(s) => s.as_(),
            ScalarType::Float(s) => s.as_(),
        }
    }
}

impl<I: AsPrimitive<f64>, F: AsPrimitive<f64>> From<ScalarType<I, F>> for f64 {
    fn from(s: ScalarType<I, F>) -> Self {
        match s {
            ScalarType::Int(s) => s.as_(),
            ScalarType::Float(s) => s.as_(),
        }
    }
}

pub type ScalarValue = ScalarType<i64, f64>;

#[derive(Debug)]
pub struct Scalar {
    pub name: String,
    pub value: ScalarValue,
}

#[derive(Debug)]
pub enum LabelType<S> {
    String(S),
}

impl<S: Clone> Clone for LabelType<S> {
    fn clone(&self) -> Self {
        match &self {
            LabelType::String(s) => LabelType::String(s.clone()),
        }
    }
}

impl<S: Copy> Copy for LabelType<S> {}

pub type LabelValue = LabelType<String>;

#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
    pub value: LabelValue,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_new_chunk() {
        let s = Some(String::from("test"));
        let s: Option<&str> = s.as_ref().map(|s| s.as_ref());
    }
}
