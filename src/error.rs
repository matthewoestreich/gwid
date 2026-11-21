// use crate::v7::UUIDv7Error;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::num::ParseIntError;
use std::time::SystemTimeError;

#[derive(Debug)]
pub enum GwidError {
    ParseUUIDError(ParseIntError),
    SystemTimeError(SystemTimeError),
    // UuidV7(UUIDv7Error),
}

impl Display for GwidError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use GwidError::*;
        match self {
            ParseUUIDError(e) => write!(f, "Error parsing UUID : {e}"),
            SystemTimeError(e) => {
                write!(f, "System time error : {e}")
            } // UuidV7(e) => write!(f, "{e}"),
        }
    }
}

// impl From<UUIDv7Error> for GwidError {
//     fn from(error: UUIDv7Error) -> Self {
//         Self::UuidV7(error)
//     }
// }

impl From<SystemTimeError> for GwidError {
    fn from(error: SystemTimeError) -> Self {
        Self::SystemTimeError(error)
    }
}

impl From<ParseIntError> for GwidError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseUUIDError(error)
    }
}

impl Error for GwidError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ParseUUIDError(e) => Some(e),
            Self::SystemTimeError(e) => Some(e),
            // Self::UuidV7(e) => Some(e),
            // _ => None,
        }
    }
}
