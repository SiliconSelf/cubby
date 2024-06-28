//! Helper functions for interfacing between axum and ruma.
//!
//! The main features of this module are the `RumaExtractor`, which is a
//! request body extractor that provides a given request, and `RumaResponder`,
//! which converts Ruma responses into ones Axum is happer about.

use std::ops::Deref;

use axum::{
    async_trait,
    body::{Body, Bytes},
    extract::{FromRequest, Path, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use bytes::BytesMut;
use ruma::api::{IncomingRequest, OutgoingResponse};

use crate::IntoMatrixError;

/// Extractor for pulling Ruma request structs from the Axum request body
pub struct RumaExtractor<T>(pub T);

impl<T> Deref for RumaExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for RumaExtractor<T>
where
    T: IncomingRequest,
    S: Send + Sync,
    Bytes: FromRequest<S>,
{
    type Rejection = Response;

    async fn from_request(
        req: Request<Body>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();
        let path_arguments: Path<Vec<String>> = parts.extract().await.unwrap();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let new_request: Request<Bytes> =
            Request::from_parts(parts, body_bytes);
        let new_t =
            T::try_from_http_request(new_request, &path_arguments).unwrap();
        Ok(Self(new_t))
    }
}

/// Responder for wrapping Ruma responses to use with Axum
pub enum RumaResponder<T, E> {
    /// The happy path
    Ok(T),
    /// Some error occured
    Err(E)
}

impl<T, E> IntoResponse for RumaResponder<T, E> where
    T: OutgoingResponse,
    E: IntoMatrixError
{
    fn into_response(self) -> Response {
        let Ok(res) = (match self {
            RumaResponder::Ok(t) => {
                t.try_into_http_response::<BytesMut>()
            },
            RumaResponder::Err(e) => {
                e.into_matrix_error().try_into_http_response::<BytesMut>()
            },
        }) else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        res.map(BytesMut::freeze).map(Body::from).into_response()
    }
}
