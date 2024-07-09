//! Exposes the ``IntoMatrixError`` trait

use ruma::api::error::MatrixError;

/// A trait that can be derived for enums to automatically generate well
/// formed matrix errors.
pub trait IntoMatrixError {
    /// Convert the enum member
    fn into_matrix_error(self) -> MatrixError;
}
