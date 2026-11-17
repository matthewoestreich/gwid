use crate::v7::UUIDv7Error;

#[derive(Debug)]
pub enum GwidError {
    ParseUUIDError(std::num::ParseIntError),
    UuidV7(UUIDv7Error),
}

impl std::fmt::Display for GwidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use GwidError::*;
        match self {
            ParseUUIDError(parse_int_error) => write!(f, "Error parsing UUID {parse_int_error}"),
            UuidV7(err) => write!(f, "{err}"),
        }
    }
}

impl From<UUIDv7Error> for GwidError {
    fn from(error: UUIDv7Error) -> Self {
        Self::UuidV7(error)
    }
}

impl From<std::num::ParseIntError> for GwidError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::ParseUUIDError(error)
    }
}

impl std::error::Error for GwidError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseUUIDError(parse_int_err) => Some(parse_int_err),
            Self::UuidV7(err) => Some(err),
            // _ => None,
        }
    }
}
