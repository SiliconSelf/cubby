//! Code related to the username availability checking endpoint.
//!
//! [Spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register)

use axum::extract::State;
use cubby_lib::{CubbyResponder, FileManager, RumaExtractor};
use cubby_macros::IntoMatrixError;
use rand::{distributions::Alphanumeric, Rng};
use ruma::{
    api::client::account::register::{
        v3::{Request, Response},
        RegistrationKind,
    },
    OwnedDeviceId,
};
use tracing::{error, instrument};

use crate::{config::PROGRAM_CONFIG, managers::dataframes::ParquetManager};

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
    /// The request reached a code branch that was supposed to be unreachable.
    /// For this specific endpoint, at the time of writing the
    /// `RegistrationKind` enum was limited to `User` and `Guest`. This is
    /// exhaustively matched with the possibility of `Some(T)` or `None` for
    /// the provided device id. If this error is being thrown, it is most
    /// likely that the `RegistrationKind` enum has been expanded since this
    /// code was written and the match statement needs to be updated to reflect
    /// the new possibilities.
    #[matrix_error(
        INTERNAL_SERVER_ERROR,
        "M_UNREACHABLE",
        "Logic for handling this request reached code that is supposed to be \
         unreachable."
    )]
    Unreachable,
}

/// Register a new account with the homeserver
///
/// [Spec](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3register)
#[instrument(level = "trace")]
pub(crate) async fn endpoint(
    State(file_manager): State<FileManager>,
    RumaExtractor(req): RumaExtractor<Request>,
) -> CubbyResponder<Response, EndpointErrors> {
    if !PROGRAM_CONFIG.allow_registration {
        return CubbyResponder::MatrixError(EndpointErrors::Disabled);
    }

    // Get DataFrame access
    let _frame = file_manager.get_managed_lazyframe("users.parquet").await;
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
            error!(
                "Unreachable code was reached in the account registration \
                 endpoint! The code must be changed to handle this case."
            );
            return CubbyResponder::MatrixError(EndpointErrors::Unreachable);
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
