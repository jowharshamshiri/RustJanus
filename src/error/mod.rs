pub mod api_error;
pub mod socket_error;

pub use api_error::UnixSockApiError;
pub use socket_error::SocketError;

pub type Result<T> = std::result::Result<T, UnixSockApiError>;