//! Accounts related endpoints

use axum::extract::State;
use cubby_lib::{RumaExtractor, RumaResponder};
use cubby_macros::IntoMatrixError;
use rand::{distributions::Alphanumeric, Rng};
use ruma::{api::client::account::register, OwnedDeviceId};

use crate::{config::PROGRAM_CONFIG, managers::dataframes::DataframeManager};

pub(crate) mod get_username_availability;

#[derive(IntoMatrixError)]
pub(crate) enum ReigstrationErrors {
    #[matrix_error(statuscode = "404", errcode = "M_WHATEVER", error = "An error")]
    Test
}

/// Register a new account with the homeserver
///
/// [Spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register)
pub(crate) async fn register(
    State(frames): State<DataframeManager>,
    RumaExtractor(req): RumaExtractor<register::v3::Request>,
) -> RumaResponder<register::v3::Response> {
    if !PROGRAM_CONFIG.allow_registration {
        // TODO: Return error here
    }
    // Get DataFrame access
    let frame = frames.get_lazy("users.parquet").await;
    // Create a device id if the request did not provide one
    let device_id = match (&req.kind, &req.device_id) {
        // Generate a new ID regardless of if a guest provided one or if a user
        // did not provide one
        (register::RegistrationKind::Guest, _)
        | (register::RegistrationKind::User, None) => {
            let mut rng = rand::thread_rng();
            let chars: String = (0..PROGRAM_CONFIG.device_id_length)
                .map(|_| rng.sample(Alphanumeric) as char)
                .collect();
            OwnedDeviceId::from(chars)
        }
        (register::RegistrationKind::User, Some(id)) => id.clone(),
        (..) => {
            unreachable!("What");
        }
    };
    // Process the registration request
    match req.kind {
        register::RegistrationKind::Guest => {
            // frame_handle
            //     .column("guests")
            //     .;
        }
        register::RegistrationKind::User => {}
        _ => todo!(),
    }
    todo!();
}
