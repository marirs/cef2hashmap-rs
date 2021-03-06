use std::fmt;

pub enum Error {
    NotCef,
    MalformedCef,
    Generic(String),
}

impl From<&str> for Error {
    fn from(err: &str) -> Error {
        Error::Generic(err.to_string())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Generic(msg) => write!(f, "Generic Error: {}", msg)?,
            Error::NotCef => write!(f, "Not a CEF String")?,
            Error::MalformedCef => write!(f, "Could be a malformed CEF string")?,
        }
        Ok(())
    }
}
