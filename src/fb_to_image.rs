extern crate image;
use crate::emulator::Emulator;
use crate::error::*;
use std::convert::TryInto;
pub trait FramebufferToImageBuffer {
    /// # Errors
    /// [`RetroRsError::ImageBufferError`]: Failed to create image buffer
    /// Others: See [`Emulator::copy_framebuffer_rgb888`].
    fn create_imagebuffer(
        &self,
    ) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, RetroRsError>;
}
impl FramebufferToImageBuffer for Emulator {
    fn create_imagebuffer(
        &self,
    ) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, RetroRsError> {
        let (w, h) = self.framebuffer_size();
        let mut bytes = vec![0; w * h * 3];
        self.copy_framebuffer_rgb888(&mut bytes)?;
        let w: u32 = w.try_into()?;
        let h: u32 = h.try_into()?;
        image::ImageBuffer::from_vec(w, h, bytes).ok_or(RetroRsError::ImageBufferError)
    }
}
