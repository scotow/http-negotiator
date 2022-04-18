use std::str::FromStr;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    MissingSeparator,
    InvalidWildcard,
    InvalidHeader,
    InvalidParam,
    InvalidQuality { source: <f32 as FromStr>::Err },
}

impl Error {
    pub fn is_invalid_quality(&self) -> bool {
        matches!(self, Error::InvalidQuality { source: _ })
    }
}
