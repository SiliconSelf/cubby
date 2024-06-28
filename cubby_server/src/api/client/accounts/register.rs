use axum::extract::State;
use cubby_lib::{IntoMatrixError, RumaExtractor, RumaResponder};
use rand::{distributions::Alphanumeric, Rng};
use ruma::{
    api::client::account::register::{
        v3::{Request, Response},
        RegistrationKind,
    },
    OwnedDeviceId,
};

use crate::{config::PROGRAM_CONFIG, managers::dataframes::DataframeManager};

pub(crate) enum EndpointErrors {}

impl IntoMatrixError for EndpointErrors {
    fn into_matrix_error(self) -> ruma::api::error::MatrixError {
        todo!();
    }
}

/// Register a new account with the homeserver
///
/// [Spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register)
pub(crate) async fn endpoint(
    State(frames): State<DataframeManager>,
    RumaExtractor(req): RumaExtractor<Request>,
) -> RumaResponder<Response, EndpointErrors> {
    if !PROGRAM_CONFIG.allow_registration {
        // TODO: Return error here
    }
    // Get DataFrame access
    let frame = frames.get_lazy("users.parquet").await;
    // Create a device id if the request did not provide one
    let device_id = match (&req.kind, &req.device_id) {
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
