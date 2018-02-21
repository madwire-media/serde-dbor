use std;
use std::fmt::{self, Display};
use std::io::Error as IoError;

use serde::{ser, de};

/// Alias for a `Result` with the error type `serde_dbor::Error`
pub type Result<T> = std::result::Result<T, Error>;

/// Every single possible error that can be thrown from either Serialization or Deserialization
#[derive(Debug)]
pub enum Error {
    /// A generic error message (used by serde for custom errors)
    Message(String),

    /// An io error
    Io(IoError),

    /// Expected more bytes in input
    Eof,

    /// Expected one of various types, but got a byte of a different type instead
    ExpectedType(Vec<super::Type>, u8),

    /// Did not expect a specific value with this type
    UnexpectedValue(super::Type, u8),

    /// Not all of the input was fully parsed, some data was left behind
    TrailingBytes,

    /// Tried to deserialize a u64 into a usize, but usize is only 32 bits
    UsizeOverflow,

    /// Not a valid type that should ever be seen except in other error messages for debugging
    /// information
    NotAType,

    /// Tried to parse a number into a char but resulting char was invalid
    FailedToParseChar,

    /// Maps and sequences must have a known size before serialization
    MustKnowItemSize,
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
            Error::MustKnowItemSize => "Map or seq had unknown size during serialization",
        }
    }
}
