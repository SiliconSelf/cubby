//! Helper functions for interfacing between axum and ruma.
//! 
//! The main features of this module are the `RumaExtractor`, which is a
//! request body extractor that provides a given request, and `RumaResponder`,
//! which converts Ruma responses into ones Axum is happer about.

use std::ops::Deref;

use axum::{async_trait, body::{Body, Bytes}, extract::{FromRequest, Path, Request}, http::StatusCode, response::{IntoResponse, Response}, RequestPartsExt};
use bytes::BytesMut;
use ruma::api::{IncomingRequest, OutgoingResponse};

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
    Bytes: FromRequest<S>
{
    type Rejection = Response;
    
    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();
        let path_arguments: Path<Vec<String>> = parts.extract().await.unwrap();
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .unwrap();
        let new_request: Request<Bytes> = Request::from_parts(parts, body_bytes);
        let new_t = T::try_from_http_request(new_request, &path_arguments).unwrap();
        Ok(Self(new_t))
    }
}

pub struct RumaResponder<T>(pub T);

impl<T: OutgoingResponse> IntoResponse for RumaResponder<T> {
    fn into_response(self) -> Response {
        if let Ok(res) = self.0.try_into_http_response::<BytesMut>() {
            res.map(BytesMut::freeze).map(Body::from).into_response()   
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
