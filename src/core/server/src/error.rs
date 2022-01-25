use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("invalid URI: {:?}, {:?}", uri, desc))]
    InvalidURI { uri: String, desc: String },

    #[snafu(display("service {:?} does not exist, has: {:?}", name, services))]
    ServiceNotExist {
        name: String,
        services: Vec<&'static str>,
    },

    #[snafu(display("internal API error: {:?}", err))]
    Internal {
        err: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
}
