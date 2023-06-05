use std::{marker::PhantomData, ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum_core::{
    extract::{FromRef, FromRequestParts},
    response::{IntoResponse, Response},
};
use http::{request::Parts, StatusCode};

use crate::{AsNegotiationStr, Error, NegotiationType, Negotiator};

#[derive(Clone, Debug)]
pub struct Negotiation<N, T>(pub PhantomData<N>, pub T);

impl<N, T> Negotiation<N, T> {
    pub fn into_inner(self) -> T {
        self.1
    }
}

impl<N, T> Deref for Negotiation<N, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[async_trait]
impl<S, N, T> FromRequestParts<S> for Negotiation<N, T>
where
    Arc<Negotiator<N, T>>: FromRef<S>,
    S: Send + Sync,
    N: NegotiationType,
    T: AsNegotiationStr + Clone,
{
    type Rejection = NegotiationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let negotiator = Arc::<Negotiator<N, T>>::from_ref(state);
        let header = parts.headers.get(N::associated_header());
        let res = match header {
            Some(header) => {
                let header = header
                    .to_str()
                    .map_err(|_| NegotiationError::InvalidAcceptHeader)?;
                negotiator
                    .negotiate(header)
                    .map_err(NegotiationError::NegotiationFailure)?
            }
            None => None,
        };

        Ok(Negotiation(
            PhantomData,
            res.unwrap_or_else(|| negotiator.unwrap_first()).clone(),
        ))
    }
}

use thiserror::Error as ThisError;

#[derive(ThisError, Eq, PartialEq, Debug)]
pub enum NegotiationError {
    #[error("invalid accept header")]
    InvalidAcceptHeader,
    #[error("negotiation failure: {0}")]
    NegotiationFailure(Error),
}

impl IntoResponse for NegotiationError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, routing::get, Router};
    use axum_core::{extract::FromRef, response::IntoResponse};
    use http::{header::ACCEPT, Request, StatusCode};
    use tower::ServiceExt;

    use crate::{axum::Negotiation, AsNegotiationStr, ContentTypeNegotiation, Negotiator};

    #[derive(Clone)]
    enum Content {
        Json,
        Text,
    }

    impl AsNegotiationStr for Content {
        fn as_str(&self) -> &str {
            match self {
                Content::Json => "application/json",
                Content::Text => "text/plain",
            }
        }
    }

    #[derive(Clone)]
    struct AppState {
        negotiator: Arc<Negotiator<ContentTypeNegotiation, Content>>,
    }

    impl FromRef<AppState> for Arc<Negotiator<ContentTypeNegotiation, Content>> {
        fn from_ref(input: &AppState) -> Self {
            Arc::clone(&input.negotiator)
        }
    }

    fn router() -> Router {
        Router::new().route("/", get(handler)).with_state(AppState {
            negotiator: Arc::new(Negotiator::new([Content::Text, Content::Json]).unwrap()),
        })
    }

    async fn handler(
        Negotiation(_, content): Negotiation<ContentTypeNegotiation, Content>,
    ) -> impl IntoResponse {
        match content {
            Content::Json => format!("{{\"message\":\"hello\"}}"),
            Content::Text => "hello".to_owned(),
        }
    }

    #[tokio::test]
    async fn negotiate() {
        // JSON.
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(ACCEPT, "application/json")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"{\"message\":\"hello\"}");

        // Text.
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(ACCEPT, "text/plain")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"hello");

        // Default.
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"hello");

        // Error.
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(ACCEPT, "INVALID_MIME_TYPE")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
