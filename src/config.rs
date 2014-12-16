use Level;


pub struct Logger {
    pub format: Box<Fn(&str) -> String + Sync + Send>,
    pub output: Vec<Output>,
    pub level: Level,
}

pub enum Output {
    Parent(Logger),
    File(Path),
    Stdout,
    Stderr,
}
