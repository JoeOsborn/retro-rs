use crate::gfx::Gfx;
use rust_libretro_sys::{retro_hw_context_type, retro_hw_render_callback, retro_system_av_info};
use surfman::{Connection, ContextAttributeFlags, ContextAttributes, GLApi, GLVersion};
use surfman::{SurfaceAccess, SurfaceType};

pub struct GlGfx {}

impl Gfx for GlGfx {
    fn preferred_api(&self) -> retro_hw_context_type {
        retro_hw_context_type::RETRO_HW_CONTEXT_OPENGL
    }
    fn prepare_hardware_context(
        &mut self,
        av: retro_system_av_info,
        cb: &mut retro_hw_render_callback,
    ) -> bool {
        let ctype = cb.context_type;
        if ctype == retro_hw_context_type::RETRO_HW_CONTEXT_VULKAN
            || ctype == retro_hw_context_type::RETRO_HW_CONTEXT_DIRECT3D
        {
            println!("Tried to load unsupported gfx context type {ctype:?}");
            return false;
        }
        let Ok(connection) = Connection::new() else {
            println!("Could not create connection");
            return false;
        };
        let Ok(adapter) = connection.create_adapter() else {
            println!("Could not obtain adapter");
            return false;
        };
        let Ok(mut device) = connection.create_device(&adapter) else {
            println!("Could not create device");
            return false;
        };
        let attributes = ContextAttributes {
            version: GLVersion::new(3, 3),
            flags: ContextAttributeFlags::DEPTH | ContextAttributeFlags::STENCIL,
        };
        let Ok(context_descriptor) = device.create_context_descriptor(&attributes) else {
            println!("Failed to create context descriptor");
            return false;
        };
        let Ok(mut context) = device.create_context(&context_descriptor, None) else {
            println!("Failed to create context");
            return false;
        };
        let Ok(w) = i32::try_from(av.geometry.max_width).or(i32::try_from(av.geometry.base_width))
        else {
            println!("Invalid context width in {:?}", av.geometry);
            return false;
        };
        let Ok(h) =
            i32::try_from(av.geometry.max_height).or(i32::try_from(av.geometry.base_height))
        else {
            println!("Invalid context height in {:?}", av.geometry);
            return false;
        };

        let Ok(surface) = device.create_surface(
            &context,
            SurfaceAccess::GPUOnly,
            SurfaceType::Generic {
                size: euclid::Size2D::new(w, h),
            },
        ) else {
            println!("Failed to create surface");
            return false;
        };
        device
            .bind_surface_to_context(&mut context, surface)
            .unwrap();
        cb.version_major = 3;
        cb.version_minor = 3;
        cb.bottom_left_origin = true;
        cb.cache_context = true;
        cb.debug_context = false;
        //cb.context_reset = ;
        //cb.context_destroy = ;
        //cb.get_proc_address = ;
        true
    }
}
