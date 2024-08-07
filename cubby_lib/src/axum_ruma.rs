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
use ruma::api::{error::MatrixError, IncomingRequest, OutgoingResponse};

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
        let path_arguments: Path<Vec<String>> = parts
            .extract()
            .await
            .map_err(|_e| StatusCode::BAD_REQUEST.into_response())?;
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_e| StatusCode::BAD_REQUEST.into_response())?;
        let new_request: Request<Bytes> =
            Request::from_parts(parts, body_bytes);
        let new_t = T::try_from_http_request(new_request, &path_arguments)
            .map_err(|_e| StatusCode::BAD_REQUEST.into_response())?;
        Ok(Self(new_t))
    }
}

/// A trait that can be derived for enums to automatically generate well-formed
/// matrix errors.
pub trait IntoMatrixError {
    /// Convert the enum member
    fn into_matrix_error(self) -> MatrixError;
}

/// Responder for wrapping Ruma responses to use with Axum
pub enum CubbyResponder<T, E> {
    /// The happy path
    Ruma(T),
    /// Some error occurred
    MatrixError(E),
    /// Something that isn't Ok, but also doesn't implement `IntoMatrixError`
    ///
    /// This is mostly once off error types such as the 401 response for
    /// /v3/register ([spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register))
    OneOff(StatusCode, serde_json::Value),
}

impl<T, E> IntoResponse for CubbyResponder<T, E>
where
    T: OutgoingResponse,
    E: IntoMatrixError,
{
    fn into_response(self) -> Response {
        match self {
            CubbyResponder::Ruma(t) => {
                let Ok(body) = t.try_into_http_response::<BytesMut>() else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };
                body.map(BytesMut::freeze).map(Body::from).into_response()
            }
            CubbyResponder::MatrixError(e) => {
                let Ok(body) =
                    e.into_matrix_error().try_into_http_response::<BytesMut>()
                else {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };
                body.map(BytesMut::freeze).map(Body::from).into_response()
            }
            CubbyResponder::OneOff(c, v) => Response::builder()
                .status(c)
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        }
    }
}
