use std::fmt;

#[derive(Debug)]
pub enum Error {
    Git(git2::Error),
    Io(std::io::Error),
    Config(String),
    Terminal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Git(e) => write!(f, "Git error: {}", e),
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Config(msg) => write!(f, "Config error: {}", msg),
            Error::Terminal(msg) => write!(f, "Terminal error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Error::Git(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
