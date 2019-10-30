mod emulator;
pub use emulator::Emulator;
mod error;
pub use error::*;

#[cfg(feature = "use_image")]
mod fb_to_image;
#[cfg(feature = "use_image")]
pub use fb_to_image::*;
