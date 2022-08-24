mod content_type;
mod encoding;
mod error;
mod language;

use std::{borrow::Borrow, collections::BTreeMap, ops::Deref};

#[cfg(feature = "axum")]
pub use axum::*;
pub use content_type::*;
pub use error::Error;

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

    fn parse_sort_header(header: &str) -> Result<Vec<(Self::Parsed, f32)>, Error>;

    fn is_match(supported: &Self::Parsed, header: &Self::Parsed) -> bool;

    #[cfg(feature = "axum")]
    fn associated_header() -> http::header::HeaderName;
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
        for header_parsed in N::parse_sort_header(header)? {
            for (supported_parsed, value) in &self.supported {
                if N::is_match(supported_parsed, &header_parsed.0) {
                    return Ok(Some(value));
                }
            }
        }
        Ok(None)
    }
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

fn matches_wildcard(specific: &str, maybe_wildcard: &str) -> bool {
    specific == maybe_wildcard || maybe_wildcard == "*"
}

#[cfg(feature = "axum")]
pub(crate) mod axum {
    use std::{
        marker::PhantomData,
        sync::Arc,
        task::{Context, Poll},
    };

    use async_trait::async_trait;
    use axum_core::extract::{FromRequest, RequestParts};
    use http::{Request, StatusCode};
    use tower_layer::Layer;
    use tower_service::Service;

    use crate::{AsNegotiationStr, NegotiationType, Negotiator};

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

    pub struct Negotiation<N, T>(PhantomData<N>, T);

    impl<N, T> Negotiation<N, T> {
        pub fn into_inner(self) -> T {
            self.1
        }
    }

    #[async_trait]
    impl<B, N, T> FromRequest<B> for Negotiation<N, T>
    where
        B: Send,
        N: NegotiationType + 'static,
        <N as NegotiationType>::Parsed: Send + Sync,
        T: Send + Sync + Clone + AsNegotiationStr + 'static,
    {
        type Rejection = (StatusCode, String);

        async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
            let negotiator =
                Arc::clone(req.extensions().get::<Arc<Negotiator<N, T>>>().ok_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing negotiator registration".to_owned(),
                ))?);
            let header = req.headers().get(N::associated_header());
            let res = match header {
                Some(header) => {
                    let header = header.to_str().map_err(|_| {
                        (StatusCode::BAD_REQUEST, "Invalid accept header".to_owned())
                    })?;
                    negotiator
                        .negotiate(header)
                        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
                }
                None => None,
            };

            Ok(Negotiation(
                PhantomData,
                res.unwrap_or_else(|| negotiator.unwrap_first()).clone(),
            ))
        }
    }
}
