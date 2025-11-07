use rust_libretro_sys::{retro_hw_context_type, retro_hw_render_callback, retro_system_av_info};

pub trait Gfx {
    fn preferred_api(&self) -> retro_hw_context_type;
    fn prepare_hardware_context(
        &mut self,
        _av: retro_system_av_info,
        cb: &mut retro_hw_render_callback,
    ) -> bool;
}

#[derive(Debug, Default)]
pub struct SoftwareGfx();
impl Gfx for SoftwareGfx {
    fn preferred_api(&self) -> retro_hw_context_type {
        retro_hw_context_type::RETRO_HW_CONTEXT_NONE
    }

    fn prepare_hardware_context(
        &mut self,
        _av: retro_system_av_info,
        _cb: &mut retro_hw_render_callback,
    ) -> bool {
        false
    }
}
