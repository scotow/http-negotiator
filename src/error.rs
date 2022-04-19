use std::str::FromStr;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("missing mime type separator \"/\"")]
    MissingSeparator,
    #[error("too many mime type parts")]
    TooManyPart,
    #[error("main part cannot be a wildcard if the sub part is not one")]
    InvalidWildcard,
    #[error("malformed header")]
    InvalidHeader,
    #[error("invalid quality param")]
    InvalidQuality { source: <f32 as FromStr>::Err },
}

impl Error {
    pub fn is_invalid_quality(&self) -> bool {
        matches!(self, Error::InvalidQuality { source: _ })
    }
}
