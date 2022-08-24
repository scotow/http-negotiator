use std::str::FromStr;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("missing separator \"{0}\"")]
    MissingSeparator(char),
    #[error("too many parts")]
    TooManyParts,
    #[error("main part cannot be a wildcard")]
    InvalidWildcard,
    #[error("malformed header")]
    InvalidHeader,
    #[error("parameters not allowed")]
    ParamsNotAllowed,
    #[error("quality param not allowed")]
    QualityNotAllowed,
    #[error("invalid quality param")]
    InvalidQuality { source: <f32 as FromStr>::Err },
}
