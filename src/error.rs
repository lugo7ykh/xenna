use std::{error, fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SyntaxError {
    MismatchedToken(&'static str),
    UnclosedDelimiter(&'static str),
    UnexpectedDelimiter(&'static str),
    UnexpectedEof,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedToken(token) => write!(f, "expected {token}"),
            Self::UnclosedDelimiter(delim) => write!(f, "expected {delim} before EOF"),
            Self::UnexpectedDelimiter(delim) => write!(f, "unexpected {delim}"),
            Self::UnexpectedEof => write!(f, "unexpected EOF"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Error {
    Io(io::ErrorKind),
    Syntax(SyntaxError),
}
impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Syntax(e) => e.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err.kind())
    }
}

impl From<SyntaxError> for Error {
    fn from(err: SyntaxError) -> Self {
        Error::Syntax(err)
    }
}
