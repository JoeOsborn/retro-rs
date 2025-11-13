use rust_libretro_sys::{retro_hw_context_type, retro_hw_render_callback, retro_system_av_info};

pub trait Gfx {
    fn preferred_api(&self) -> retro_hw_context_type;
    fn video_refresh(&mut self, w: u32, h: u32, pitch: usize);
    fn prepare_hardware_context(
        &mut self,
        _av: retro_system_av_info,
        cb: &mut retro_hw_render_callback,
    ) -> bool;
    fn destroy_context(&mut self) {}
    fn bind(&mut self) {}
    fn unbind(&mut self) {}
    fn sync_framebuffer(&self, _fb: &mut [u8]) {}
}

#[derive(Debug, Default)]
pub struct SoftwareGfx();
impl Gfx for SoftwareGfx {
    fn preferred_api(&self) -> retro_hw_context_type {
        retro_hw_context_type::RETRO_HW_CONTEXT_NONE
    }
    fn destroy_context(&mut self) {}

    fn prepare_hardware_context(
        &mut self,
        _av: retro_system_av_info,
        _cb: &mut retro_hw_render_callback,
    ) -> bool {
        false
    }

    fn video_refresh(&mut self, _w: u32, _h: u32, _p: usize) {}
}
