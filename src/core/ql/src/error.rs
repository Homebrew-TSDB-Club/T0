use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("parser internal error: {:?}", err))]
    InternalError { err: String },
    #[snafu(display("query does not have a metric name"))]
    NoName,
}
