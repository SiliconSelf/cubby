//! Code related to the username availability checking endpoint.
//!
//! [Spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register)

use axum::extract::State;
use cubby_lib::{CubbyResponder, RumaExtractor};
use cubby_macros::IntoMatrixError;
use rand::{distributions::Alphanumeric, Rng};
use ruma::{
    api::client::account::register::{
        v3::{Request, Response},
        RegistrationKind,
    },
    OwnedDeviceId,
};

use crate::{config::PROGRAM_CONFIG, managers::dataframes::DataframeManager};

/// All the possible errors that can be returned by the endpoint
#[derive(IntoMatrixError)]
pub(crate) enum EndpointErrors {
    /// The requested username is already in use
    #[matrix_error(
        BAD_REQUEST,
        "M_USER_IN_USE",
        "The desired user ID is already taken."
    )]
    _InUse,
    /// The requested username is invalid
    #[matrix_error(
        BAD_REQUEST,
        "M_INVALID_USERNAME",
        "The desired user ID is not a valid user name."
    )]
    _InvalidUsername,
    /// The requested username is in the exclusive namespace of an appservice
    #[matrix_error(
        BAD_REQUEST,
        "M_EXCLUSIVE",
        "The desired user ID is in the exclusive namespace claimed by an \
         application service."
    )]
    _Exclusive,
    /// Registration is currently disabled on the server
    #[matrix_error(
        FORBIDDEN,
        "M_FORBIDDEN",
        "Registration is disabled on this homeserver."
    )]
    Disabled,
}

/// Register a new account with the homeserver
///
/// [Spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register)
pub(crate) async fn endpoint(
    State(frames): State<DataframeManager>,
    RumaExtractor(req): RumaExtractor<Request>,
) -> CubbyResponder<Response, EndpointErrors> {
    if !PROGRAM_CONFIG.allow_registration {
        return CubbyResponder::MatrixError(EndpointErrors::Disabled);
    }

    // Get DataFrame access
    let _frame = frames.get_lazy("users.parquet");
    // Create a device id if the request did not provide one
    let _device_id = match (&req.kind, &req.device_id) {
        // Generate a new ID regardless of if a guest provided one or if a user
        // did not provide one
        (RegistrationKind::Guest, _) | (RegistrationKind::User, None) => {
            let mut rng = rand::thread_rng();
            // SAFETY: THis as conversion is ok because rand will only return
            // things that can be cast as char
            #[allow(clippy::as_conversions)]
            let chars: String = (0..PROGRAM_CONFIG.device_id_length)
                .map(|_| rng.sample(Alphanumeric) as char)
                .collect();
            OwnedDeviceId::from(chars)
        }
        (RegistrationKind::User, Some(id)) => id.clone(),
        (..) => {
            unreachable!("What");
        }
    };
    // Process the registration request
    match req.kind {
        RegistrationKind::Guest => {
            todo!();
        }
        RegistrationKind::User => {
            todo!();
        }
        _ => todo!(),
    }
}
