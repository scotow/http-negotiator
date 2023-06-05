#[cfg(feature = "axum")]
mod axum;
mod content_type;
mod encoding;
mod error;
mod language;

use std::{borrow::Borrow, collections::BTreeMap, ops::Deref};

pub use content_type::*;
pub use encoding::*;
pub use error::Error;
pub use language::*;

#[cfg(feature = "axum")]
pub use crate::axum::*;

#[derive(PartialEq, Clone, Debug)]
pub enum MaybeWildcard<T> {
    Specific(T),
    Wildcard,
}

impl<T> MaybeWildcard<T> {
    pub fn from_str<'a>(input: T) -> MaybeWildcard<T>
    where
        T: PartialEq<&'a str>,
    {
        if input == "*" {
            Self::Wildcard
        } else {
            Self::Specific(input)
        }
    }

    pub fn matches<U>(&self, other: &U) -> bool
    where
        T: PartialEq<U>,
    {
        match self {
            MaybeWildcard::Specific(s) => s == other,
            MaybeWildcard::Wildcard => true,
        }
    }
}

impl<'a> From<&'a str> for MaybeWildcard<&'a str> {
    fn from(s: &'a str) -> Self {
        Self::from_str(s)
    }
}

pub trait AsNegotiationStr {
    fn as_str(&self) -> &str;
}

impl<T: AsRef<str>> AsNegotiationStr for T {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

pub trait NegotiationType {
    type Parsed;

    fn parse_elem<M: AsNegotiationStr>(input: &M) -> Result<Self::Parsed, Error>;

    fn parse_and_negotiate_header<'a, T>(
        supported: &'a [(Self::Parsed, T)],
        header: &str,
    ) -> Result<Option<&'a T>, Error>;

    #[cfg(feature = "axum")]
    fn associated_header() -> http::header::HeaderName;
}

#[derive(Clone, Debug)]
pub struct Negotiator<N: NegotiationType, T> {
    supported: Vec<(N::Parsed, T)>,
}

impl<N, T> Negotiator<N, T>
where
    N: NegotiationType,
{
    pub fn len(&self) -> usize {
        self.supported.len()
    }

    pub fn is_empty(&self) -> bool {
        self.supported.is_empty()
    }

    pub fn unwrap_first(&self) -> &T {
        &self.supported[0].1
    }
}

impl<N, T> Negotiator<N, T>
where
    N: NegotiationType,
    T: AsNegotiationStr,
{
    pub fn new<I>(iter: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        Ok(Self {
            supported: iter
                .into_iter()
                .map(|m| Ok((N::parse_elem(&m)?, m)))
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn negotiate(&self, header: &str) -> Result<Option<&T>, Error> {
        N::parse_and_negotiate_header(&self.supported, header)
    }
}

fn match_first<'a, 'b, S, T, H, F, I, J>(supported: I, from_header: J, mut f: F) -> Option<&'a T>
where
    S: 'a,
    H: 'b + ?Sized,
    I: IntoIterator<Item = &'a (S, T)> + Clone,
    J: IntoIterator<Item = &'b H>,
    F: FnMut(&'a S, &'b H) -> bool,
{
    from_header.into_iter().find_map(|h| {
        supported
            .clone()
            .into_iter()
            .find_map(|(s, v)| f(s, h).then(|| v))
    })
}

fn extract_quality<K, V>(params: &mut BTreeMap<K, V>) -> Result<f32, Error>
where
    K: Borrow<str> + Ord,
    V: Deref<Target = str>,
{
    params
        .remove("q")
        .map(|q| {
            q.parse::<f32>()
                .map_err(|err| Error::InvalidQuality { source: err })
        })
        .transpose()
        .map(|q| q.unwrap_or(1.))
}
