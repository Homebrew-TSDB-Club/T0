use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum RuntimeError {
    #[snafu(display(
        "worker num can not be greater than cpu num, require: {}, has: {}",
        require,
        has
    ))]
    NoMuchCore { require: usize, has: usize },
}
