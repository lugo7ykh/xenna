use std::{
    error,
    fmt::{self, Display},
    io,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SyntaxError {
    UnexpectedToken(&'static str),
    UnclosedDelimiter(&'static str),
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken(expected) => {
                write!(f, "expected `{expected}`")
            }
            Self::UnclosedDelimiter(delim) => {
                write!(f, "delimiter `{delim}` not found before EOF")
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Error {
    Io(io::ErrorKind),
    Syntax(SyntaxError),
}
impl error::Error for Error {}

impl Display for Error {
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
