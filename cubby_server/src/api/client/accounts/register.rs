use axum::extract::State;
use cubby_lib::{RumaExtractor, RumaResponder};
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

#[derive(IntoMatrixError)]
pub(crate) enum EndpointErrors {
    #[matrix_error(
        BAD_REQUEST,
        "M_USER_IN_USE",
        "The desired user ID is already taken."
    )]
    _InUse,
    #[matrix_error(
        BAD_REQUEST,
        "M_INVALID_USERNAME",
        "The desired user ID is not a valid user name."
    )]
    _InvalidUsername,
    #[matrix_error(
        BAD_REQUEST,
        "M_EXCLUSIVE",
        "The desired user ID is in the exclusive namespace claimed by an \
         application service."
    )]
    _Exclusive,
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
) -> RumaResponder<Response, EndpointErrors> {
    if !PROGRAM_CONFIG.allow_registration {
        return RumaResponder::Err(EndpointErrors::Disabled)
    }
    // Get DataFrame access
    let _frame = frames.get_lazy("users.parquet").await;
    // Create a device id if the request did not provide one
    let _device_id = match (&req.kind, &req.device_id) {
        // Generate a new ID regardless of if a guest provided one or if a user
        // did not provide one
        (RegistrationKind::Guest, _) | (RegistrationKind::User, None) => {
            let mut rng = rand::thread_rng();
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
            // frame_handle
            //     .column("guests")
            //     .;
        }
        RegistrationKind::User => {}
        _ => todo!(),
    }
    todo!();
}
