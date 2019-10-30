use std::error::Error;
use std::fmt::Display;
#[derive(Debug)]
pub enum RetroRsError {
    NoFramebufferError,
    ImageBufferError,
    TryFromIntError(std::num::TryFromIntError),
}
impl From<std::num::TryFromIntError> for RetroRsError {
    fn from(err: std::num::TryFromIntError) -> RetroRsError {
        RetroRsError::TryFromIntError(err)
    }
}
impl Display for RetroRsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            RetroRsError::NoFramebufferError => {
                write!(f, "This emulator does not have a framebuffer yet.")
            }
            RetroRsError::ImageBufferError => write!(f, "Failure in creating image buffer"),
            RetroRsError::TryFromIntError(ref err) => err.fmt(f),
        }
    }
}
impl Error for RetroRsError {}
