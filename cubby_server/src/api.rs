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

pub(crate) mod appservice;
pub(crate) mod client;
pub(crate) mod federation;
