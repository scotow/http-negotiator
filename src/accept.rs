use std::collections::BTreeMap;

use crate::{extract_quality, matches_wildcard, mime_precision_score, parse_mime, AsMime, Error};

#[derive(Clone, Debug)]
pub struct AcceptNegotiator<T> {
    supported: Vec<(String, String, BTreeMap<String, String>, T)>,
}

impl<T> AcceptNegotiator<T> {
    pub fn new<I>(iter: I) -> Result<Self, Error>
    where
        T: AsMime,
        I: IntoIterator<Item = T>,
    {
        Ok(Self {
            supported: iter
                .into_iter()
                .map(|m| {
                    let (main, sub, params) = parse_mime::<String>(m.as_mime(), false)?;
                    if params.contains_key("q") {
                        return Err(Error::QualityNotAllowed);
                    }
                    Ok((main, sub, params, m))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn len(&self) -> usize {
        self.supported.len()
    }

    pub fn unwrap_first(&self) -> &T {
        &self.supported[0].3
    }
}

impl<T> AcceptNegotiator<T> {
    pub fn negotiate(&self, header: &str) -> Result<Option<&T>, Error> {
        let mimes = Self::parse_sort_header(header)?;

        for mime in mimes {
            for supported in &self.supported {
                if matches_wildcard(&supported.0, mime.0)
                    && matches_wildcard(&supported.1, mime.1)
                    && supported
                        .2
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .eq(mime.2.iter().map(|(&k, &v)| (k, v)))
                {
                    return Ok(Some(&supported.3));
                }
            }
        }

        Ok(None)
    }

    fn parse_sort_header(
        header: &str,
    ) -> Result<Vec<(&str, &str, BTreeMap<&str, &str>, f32)>, Error> {
        let mut mimes = header
            .split(',')
            .map(|m| {
                let (main, sub, mut params) = parse_mime::<&str>(m.trim(), true)?;
                let q = extract_quality(&mut params)?;
                Ok((main, sub, params, q))
            })
            .collect::<Result<Vec<_>, _>>()?;

        mimes.sort_by(|m1, m2| {
            m1.3.total_cmp(&m2.3)
                .then_with(|| {
                    mime_precision_score(&m1.0, &m1.1).cmp(&mime_precision_score(&m2.0, &m2.1))
                })
                .then_with(|| m1.2.len().cmp(&m2.2.len()))
                .reverse()
        });
        Ok(mimes)
    }
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

    use super::AcceptNegotiator;

    impl<S, T> Layer<S> for AcceptNegotiator<T>
    where
        T: Clone,
    {
        type Service = AcceptNegotiatorService<S, T>;

        fn layer(&self, inner: S) -> Self::Service {
            if self.len() == 0 {
                panic!("negotiator must not be empty");
            }

            AcceptNegotiatorService {
                inner,
                negotiator: Arc::new(self.clone()),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct AcceptNegotiatorService<S, T> {
        inner: S,
        negotiator: Arc<AcceptNegotiator<T>>,
    }

    impl<ResBody, S, T> Service<Request<ResBody>> for AcceptNegotiatorService<S, T>
    where
        S: Service<Request<ResBody>>,
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

    pub struct Negotiation<T>(pub T);

    #[async_trait]
    impl<B, T> FromRequest<B> for Negotiation<T>
    where
        T: Send + Sync + Clone + 'static,
        B: Send,
    {
        type Rejection = (StatusCode, String);

        async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
            let negotiator = req.extensions().get::<Arc<AcceptNegotiator<T>>>().ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Missing negotiator registration".to_owned(),
            ))?;
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
                res.unwrap_or_else(|| negotiator.unwrap_first()).clone(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::AcceptNegotiator;

    #[test]
    fn new() {
        assert_eq!(
            AcceptNegotiator::new(["text/plain"]).unwrap().supported,
            vec![(
                "text".to_owned(),
                "plain".to_owned(),
                BTreeMap::default(),
                "text/plain"
            )]
        );
    }

    #[test]
    fn parse_sort() {
        assert_eq!(
            AcceptNegotiator::<&str>::parse_sort_header(
                "text/*, text/plain, text/plain;format=flowed, */*"
            )
            .unwrap(),
            vec![
                ("text", "plain", BTreeMap::from([("format", "flowed")]), 1.),
                ("text", "plain", BTreeMap::default(), 1.),
                ("text", "*", BTreeMap::default(), 1.),
                ("*", "*", BTreeMap::default(), 1.),
            ]
        );

        assert_eq!(
            AcceptNegotiator::<&str>::parse_sort_header(
                "text/*, text/plain, text/plain;format=flowed, */*"
            )
            .unwrap(),
            vec![
                ("text", "plain", BTreeMap::from([("format", "flowed")]), 1.),
                ("text", "plain", BTreeMap::default(), 1.),
                ("text", "*", BTreeMap::default(), 1.),
                ("*", "*", BTreeMap::default(), 1.),
            ]
        );

        assert_eq!(
            AcceptNegotiator::<&str>::parse_sort_header(
                "text/plain;q=0.2,text/not-plain;q=0.4,text/hybrid"
            )
            .unwrap(),
            vec![
                ("text", "hybrid", BTreeMap::default(), 1.),
                ("text", "not-plain", BTreeMap::default(), 0.4),
                ("text", "plain", BTreeMap::default(), 0.2),
            ]
        );
    }

    #[test]
    fn negotiate() {
        assert!(AcceptNegotiator::new(["application/json"])
            .unwrap()
            .negotiate("text/html")
            .unwrap()
            .is_none());

        assert_eq!(
            AcceptNegotiator::new(["application/json"])
                .unwrap()
                .negotiate("application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("audio/mp3, application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            AcceptNegotiator::new(["application/json", "text/plain"])
                .unwrap()
                .negotiate("text/plain, application/json")
                .unwrap(),
            Some(&"text/plain")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/html;level=3", "text/html;level=2", "image/jpeg", "text/plain", "text/html", "text/html;level=1"])
                .unwrap()
                .negotiate("text/*;q=0.3, text/html;q=0.7, text/html;level=1, text/html;level=2;q=0.4, */*;q=0.5")
                .unwrap(),
            Some(&"text/html;level=1")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .unwrap(),
            Some(&"text/plain")
        );

        assert_eq!(
            AcceptNegotiator::new(["application/json", "text/plain"])
                .unwrap()
                .negotiate("text/plain;q=0.9, */*")
                .unwrap(),
            Some(&"application/json")
        );
    }
}
