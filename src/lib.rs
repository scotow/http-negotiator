mod accept;
mod encoding;
mod error;
// mod v2;

use std::{borrow::Borrow, collections::BTreeMap, ops::Deref};

pub use accept::*;
#[cfg(feature = "axum")]
pub use axum::*;
pub use error::Error;

pub trait AsMime {
    fn as_mime(&self) -> &str;
}

impl<T: AsRef<str>> AsMime for T {
    fn as_mime(&self) -> &str {
        self.as_ref()
    }
}

pub trait NegotiationType {
    type Parsed;

    fn parse_elem<M: AsMime>(input: &M) -> Result<Self::Parsed, Error>;

    fn parse_sort_header(header: &str) -> Result<Vec<(Self::Parsed, f32)>, Error>;

    fn is_match(supported: &Self::Parsed, header: &Self::Parsed) -> bool;
}

#[derive(Clone, Debug)]
pub struct Negotiator<N: NegotiationType, T> {
    supported: Vec<(N::Parsed, T)>,
}

impl<'a, N, T> Negotiator<N, T>
where
    N: NegotiationType,
{
    pub fn len(&self) -> usize {
        self.supported.len()
    }

    pub fn unwrap_first(&self) -> &T {
        &self.supported[0].1
    }
}

impl<'a, N, T> Negotiator<N, T>
where
    N: NegotiationType,
    T: AsMime,
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
        for mime in N::parse_sort_header(header)? {
            for (supported_parsed, value) in &self.supported {
                if N::is_match(supported_parsed, &mime.0) {
                    return Ok(Some(value));
                }
            }
        }
        Ok(None)
    }
}

fn parse_mime<'a, T>(mime: &'a str, from_header: bool) -> Result<(T, T, BTreeMap<T, T>), Error>
where
    T: From<&'a str> + Ord + Borrow<str>,
{
    let mut parts = mime.split(';');
    let left = parts.next().ok_or(Error::InvalidHeader)?.trim();

    let (main, sub) = left.split_once('/').ok_or(Error::MissingSeparator)?;
    if sub.contains('/') {
        return Err(Error::TooManyParts);
    }
    if from_header {
        if main == "*" && sub != "*" {
            return Err(Error::InvalidWildcard);
        }
    } else {
        if main == "*" || sub == "*" {
            return Err(Error::InvalidWildcard);
        }
    }

    let params = parts
        .map(|param| {
            let (k, v) = param.trim().split_once('=').ok_or(Error::InvalidHeader)?;
            Ok((k.into(), v.into()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if !from_header && params.contains_key("q") {
        return Err(Error::QualityNotAllowed);
    }

    Ok((main.into(), sub.into(), params))
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

fn mime_precision_score(main: &str, sub: &str) -> u8 {
    match (main, sub) {
        ("*", "*") => 0,
        (_, "*") => 1,
        _ => 2,
    }
}

fn matches_wildcard(specific: &str, maybe_wildcard: &str) -> bool {
    specific == maybe_wildcard || maybe_wildcard == "*"
}

#[cfg(feature = "axum")]
pub(crate) mod axum {
    use std::{
        sync::Arc,
        task::{Context, Poll},
    };

    use async_trait::async_trait;
    use axum_core::extract::{FromRequest, RequestParts};
    use http::{header, Request, StatusCode};
    use tower_layer::Layer;
    use tower_service::Service;

    use crate::{AsMime, NegotiationType, Negotiator};

    impl<S, N, T> Layer<S> for Negotiator<N, T>
    where
        Self: Clone,
        N: NegotiationType,
        T: Clone,
    {
        type Service = NegotiatorService<S, N, T>;

        fn layer(&self, inner: S) -> Self::Service {
            if self.len() == 0 {
                panic!("negotiator must not be empty");
            }

            Self::Service {
                inner,
                negotiator: Arc::new(self.clone()),
            }
        }
    }

    #[derive(Clone)]
    pub struct NegotiatorService<S, N: NegotiationType, T> {
        inner: S,
        negotiator: Arc<Negotiator<N, T>>,
    }

    impl<ResBody, S, N, T> Service<Request<ResBody>> for NegotiatorService<S, N, T>
    where
        S: Service<Request<ResBody>>,
        N: NegotiationType + 'static,
        <N as NegotiationType>::Parsed: Sync + Send,
        T: Clone + Send + Sync + 'static,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        #[inline]
        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, mut req: Request<ResBody>) -> Self::Future {
            req.extensions_mut().insert(Arc::clone(&self.negotiator));
            self.inner.call(req)
        }
    }

    pub struct Negotiation<N, T>(pub N, pub T);

    #[async_trait]
    impl<B, N, T> FromRequest<B> for Negotiation<N, T>
    where
        B: Send,
        N: NegotiationType + Default + 'static,
        <N as NegotiationType>::Parsed: Send + Sync,
        T: Send + Sync + Clone + AsMime + 'static,
    {
        type Rejection = (StatusCode, String);

        async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
            let negotiator =
                Arc::clone(req.extensions().get::<Arc<Negotiator<N, T>>>().ok_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing negotiator registration".to_owned(),
                ))?);
            let header = req.headers().get(header::ACCEPT);
            let res = match header {
                Some(header) => {
                    let header = header.to_str().map_err(|_| {
                        (StatusCode::BAD_REQUEST, "Invalid Accept header".to_owned())
                    })?;
                    negotiator
                        .negotiate(header)
                        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
                }
                None => None,
            };

            Ok(Negotiation(
                N::default(),
                res.unwrap_or_else(|| negotiator.unwrap_first()).clone(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::parse_mime;
    use crate::Error;

    #[test]
    fn parse() {
        // Basic.
        assert_eq!(
            parse_mime("text/plain", false).unwrap(),
            ("text", "plain", BTreeMap::default()),
        );

        // With one param.
        assert_eq!(
            parse_mime("text/html;level=1", false).unwrap(),
            ("text", "html", BTreeMap::from([("level", "1")]),)
        );

        // Param with space.
        assert_eq!(
            parse_mime("text/html; level=1", false).unwrap(),
            ("text", "html", BTreeMap::from([("level", "1")]),)
        );

        // Multiple params.
        assert_eq!(
            parse_mime("text/html;level=1;origin=EU", false).unwrap(),
            (
                "text",
                "html",
                BTreeMap::from([("level", "1"), ("origin", "EU")]),
            )
        );

        assert_eq!(
            parse_mime::<&str>("text/plain;q=1", false).unwrap_err(),
            Error::QualityNotAllowed,
        );

        assert_eq!(
            parse_mime::<&str>("*/plain", true).unwrap_err(),
            Error::InvalidWildcard
        );

        assert_eq!(
            parse_mime::<&str>("text/*", false).unwrap_err(),
            Error::InvalidWildcard
        );

        assert!(parse_mime::<&str>("text/*", true).is_ok());

        assert_eq!(
            parse_mime::<&str>("text/plain/extra", true).unwrap_err(),
            Error::TooManyParts
        );
    }
}
