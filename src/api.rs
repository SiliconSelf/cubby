use std::ops::Deref;

use axum::{async_trait, body::{Body, Bytes}, extract::{FromRequest, Path, Request}, http::StatusCode, response::{IntoResponse, Response}, RequestPartsExt};
use bytes::BytesMut;
use ruma::api::{IncomingRequest, OutgoingResponse};

pub(crate) mod client;
pub(crate) mod federation;
pub(crate) mod appservice;

pub(crate) struct RumaExtractor<T> {
    pub(crate) body: T,
    // pub(crate) sender_user: Option<OwnedUserId>,
    // pub(crate) sender_device: Option<OwnedDeviceId>,
    // pub(crate) sender_servername: Option<OwnedServerName>,
    // // This is None when body is not a valid string
    // pub(crate) json_body: Option<CanonicalJsonValue>,
    // pub(crate) appservice_info: Option<RegistrationInfo>,
}

impl<T> Deref for RumaExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.body
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
        Ok(Self { body: new_t })
    }
}

pub(crate) struct RumaResponder<T>(T);

impl<T: OutgoingResponse> IntoResponse for RumaResponder<T> {
    fn into_response(self) -> Response {
        if let Ok(res) = self.0.try_into_http_response::<BytesMut>() {
            res.map(BytesMut::freeze).map(Body::from).into_response()   
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
