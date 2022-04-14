use arrow2::error::ArrowError;
use ql::error::Error as ParseError;
use snafu::Snafu;
use storage::error::ScanError;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("storage scan error: {:?}", err))]
    StorageError { err: ScanError },
    #[snafu(display("parse error: {:?}", err))]
    ParseError { err: ParseError },
    #[snafu(display("internal arrow error: {:?}", err))]
    InternalError { err: ArrowError },
    #[snafu(display("no such field: {:?}", name))]
    NoSuchField { name: String },
}
