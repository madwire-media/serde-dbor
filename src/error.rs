use std;
use std::fmt::{self, Display};
use std::io::Error as IoError;

use serde::{ser, de};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Io(IoError),
    Eof,
    ExpectedType(Vec<super::Type>, u8),
    UnexpectedValue(super::Type, u8),
    TrailingBytes,
    UsizeOverflow,
    NotAType,
    FailedToParseChar,
    TODO
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg:T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref io_error) => io_error.fmt(formatter),
            Error::ExpectedType(ref expected, ref got) => write!(formatter, "Expected any of \
                {:?}, but instead got byte {:x}", expected, got),
            Error::UnexpectedValue(ref ty, ref val) => write!(formatter, "Value {:x} is an \
                invalid value for type {:?}", val, ty),
            _ => formatter.write_str(std::error::Error::description(self)),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref msg) => msg,
            Error::Io(ref io_error) => io_error.description(),
            Error::Eof => "Unexpected end of input",
            Error::ExpectedType(_, _) => "Expected a different type",
            Error::UnexpectedValue(_, _) => "Did not expect a specific value",
            Error::TrailingBytes => "Finished deserialization with trailing bytes",
            Error::UsizeOverflow => "Number is too big to be deserialized into a usize",
            Error::NotAType => "You should never see this, a type was deserialized that doesn't \
                actually exist",
            Error::FailedToParseChar => "Failed to turn byte array into char",
            Error::TODO => "Unimplemented error, you shouldn't see this",
        }
    }
}
