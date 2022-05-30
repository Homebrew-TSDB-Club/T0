use common::array::Array;
use std::sync::Arc;

mod context;

pub trait Operation {
    fn apply<A: Array>(&self, array: Arc<Box<A>>);
}

pub struct Pipeline {
    operations: Vec<Box<dyn Operation>>,
}

pub struct FilterExecution;

impl Operation for FilterExecution {}
