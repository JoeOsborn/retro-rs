use std::error::Error;
use std::fmt::Display;
#[derive(Debug)]
pub enum RetroRsError {
    NoFramebufferError,
    ImageBufferError,
    TryFromIntError(std::num::TryFromIntError),
    RAMCopyDestTooSmallError,
    RAMCopySrcOutOfBoundsError,
    RAMMapOutOfRangeError,
    RAMCopyCrossedRegionError,
    RAMCopyNotMappedIntoMemoryRegionError,
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
            RetroRsError::RAMCopyDestTooSmallError => {
                write!(f, "Destination for RAM copy too small")
            }
            RetroRsError::RAMCopySrcOutOfBoundsError => {
                write!(f, "Source address range for RAM copy out of bounds")
            }
            RetroRsError::RAMMapOutOfRangeError => {
                write!(f, "Given memory map is not valid for this core")
            }
            RetroRsError::RAMCopyCrossedRegionError => {
                write!(f, "RAM copy crossed over memory region boundaries")
            }
            RetroRsError::RAMCopyNotMappedIntoMemoryRegionError => {
                write!(f, "RAM copy doesn't start within a memory region")
            }
        }
    }
}
impl Error for RetroRsError {}
