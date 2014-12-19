use api;
use Level;

pub struct Logger {
    pub format: Box<Fn(&str, &Level) -> String + Sync + Send>,
    pub output: Vec<Output>,
    pub level: Level,
}

pub enum Output {
    Parent(Logger),
    File(Path),
    Stdout,
    Stderr,
    Custom(Box<api::Logger + Sync + Send>),
}
