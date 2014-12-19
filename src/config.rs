use api;

pub struct Logger {
    pub format: Box<Fn(&str, &api::Level) -> String + Sync + Send>,
    pub output: Vec<Output>,
    pub level: api::Level,
}

pub enum Output {
    Parent(Logger),
    File(Path),
    Stdout,
    Stderr,
    Custom(Box<api::Logger + Sync + Send>),
}
