use common::index::IndexType;
use common::time::Instant;
use snafu::Snafu;
use std::sync::Arc;

#[derive(Snafu, Debug)]
pub enum WriteError {
    #[snafu(display("timestamp: {} of table: {:?} has been archived", t, table_name))]
    TimestampArchived { t: Instant, table_name: Arc<str> },
    #[snafu(display("internal error: {:?}", err))]
    InternalError { err: String },
}

#[derive(Snafu, Debug)]
pub enum ScanError {
    #[snafu(display("table {:?} does not exist", name))]
    NoSuchTable { name: String },
    #[snafu(display("label {:?} does not exist", name))]
    NoSuchLabel { name: String },
    #[snafu(display("scalar {:?} does not exist", name))]
    NoSuchScalar { name: String },
    #[snafu(display("column {:?} does not has index: {:?}", id, index))]
    NoSuchColumnIndex { id: usize, index: IndexType<()> },
}
