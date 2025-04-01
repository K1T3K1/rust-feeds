use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum AuthError {
    #[error("User is not allowed to send to channel: {}", .0)]
    UnauthPub(String),
    #[error("User is not allowed to listen to channel: {}", .0)]
    UnauthSub(String),
}

#[derive(Debug, Error)]
pub enum PublishError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    AuthError(#[from] AuthError),
}

#[derive(Debug, Error)]
pub enum SubscribeError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    AuthError(#[from] AuthError),
}
