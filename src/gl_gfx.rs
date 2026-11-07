use crate::gfx::Gfx;
use gisst_rust_libretro_sys::{retro_hw_context_type, retro_hw_render_callback, retro_system_av_info};
use surfman::{Connection, ContextAttributeFlags, ContextAttributes, GLVersion};
use surfman::{SurfaceAccess, SurfaceType};

static GFX: std::sync::Mutex<Option<GlGfxData>> = const { std::sync::Mutex::new(None) };

pub fn get_proc_address_r(proc: &str) -> *const std::ffi::c_void {
    // dbg!(std::thread::current().id());
    let ctx = GFX.lock().unwrap();
    let ctx = ctx.as_ref().unwrap();
    ctx.device.get_proc_address(&ctx.context, proc)
}
pub unsafe extern "C" fn get_proc_address(
    proc: *const std::ffi::c_char,
) -> gisst_rust_libretro_sys::retro_proc_address_t {
    // dbg!(std::thread::current().id());
    unsafe {
        let proc_str = std::ffi::CStr::from_ptr(proc).to_str().unwrap();
        std::mem::transmute(get_proc_address_r(proc_str))
    }
}
pub unsafe extern "C" fn get_current_framebuffer() -> usize {
    // dbg!(std::thread::current().id());
    let ctx = GFX.lock().unwrap();
    ctx.as_ref().map_or(0, GlGfxData::get_fbo) as usize
}

struct GlGfxData {
    w: u32,
    h: u32,
    context: surfman::Context,
    device: surfman::Device,
    surface: Option<surfman::Surface>,
    fbo: u32,
}
impl GlGfxData {
    fn get_fbo(&self) -> u32 {
        self.fbo
    }
    fn create(w: u32, h: u32, version_major: u8, version_minor: u8) -> Option<Self> {
        // dbg!(std::thread::current().id());
        let Ok(connection) = Connection::new() else {
            println!("Could not create connection");
            return None;
        };
        let Ok(adapter) = connection.create_adapter() else {
            println!("Could not obtain adapter");
            return None;
        };
        let Ok(device) = connection.create_device(&adapter) else {
            println!("Could not create device");
            return None;
        };
        let attributes = ContextAttributes {
            version: GLVersion::new(version_major, version_minor),
            flags: ContextAttributeFlags::DEPTH | ContextAttributeFlags::STENCIL,
        };
        let Ok(context_descriptor) = device.create_context_descriptor(&attributes) else {
            println!("Failed to create context descriptor");
            return None;
        };
        let Ok(context) = device.create_context(&context_descriptor, None) else {
            println!("Failed to create context");
            return None;
        };
        let Ok(()) = device.make_context_current(&context) else {
            println!("Failed to make context current");
            return None;
        };
        gl::load_with(|s| device.get_proc_address(&context, s));
        assert!(w < (i32::MAX as u32), "Framebuffer width too large");
        assert!(h < (i32::MAX as u32), "Framebuffer height too large");
        let mut ret = Self {
            w,
            h,
            fbo: 0,
            context,
            device,
            surface: None,
        };
        ret.create_surface();
        Some(ret)
    }
    fn create_surface(&mut self) {
        if self.w == 0 || self.h == 0 {
            return;
        }
        self.surface = self
            .device
            .create_surface(
                &self.context,
                SurfaceAccess::GPUOnly,
                SurfaceType::Generic {
                    size: euclid::Size2D::new(self.w.cast_signed(), self.h.cast_signed()),
                },
            )
            .ok();
        self.fbo = self
            .device
            .surface_info(self.surface.as_ref().unwrap())
            .framebuffer_object
            .unwrap()
            .0
            .get();
    }
    fn destroy_surface(&mut self) {
        if let Some(surf) = self.surface.as_mut() {
            self.device
                .destroy_surface(&mut self.context, surf)
                .unwrap();
            self.fbo = 0;
        }
    }
    fn set_dimensions(&mut self, w: u32, h: u32) {
        if w == self.w && h == self.h && self.surface.is_some() {
            return;
        }
        self.destroy_surface();
        self.w = w;
        self.h = h;
        assert!(self.w < (i32::MAX as u32), "Framebuffer width too large");
        assert!(self.h < (i32::MAX as u32), "Framebuffer height too large");
        self.create_surface();
    }
    fn bind(&mut self) {
        use gl;
        if let Some(surf) = self.surface.take() {
            let id = self.fbo;
            self.device
                .bind_surface_to_context(&mut self.context, surf)
                .unwrap();
            self.device.make_context_current(&self.context).unwrap();
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, id);
            }
        }
    }
    fn unbind(&mut self) {
        if self.surface.is_none()
            && let Some(surf) = self
                .device
                .unbind_surface_from_context(&mut self.context)
                .ok()
                .flatten()
        {
            let _ = self.surface.insert(surf);
        }
    }
    fn sync_framebuffer(&mut self, fb: &mut [u8]) {
        unsafe {
            self.bind();
            let fbo = self.get_fbo();
            if let e = gl::GetError()
                && e != 0
            {
                println!("Existing err {e:?}, fbo {fbo}");
            }
            // gl::Flush();
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            if let e = gl::GetError()
                && e != 0
            {
                println!("glBindFramebuffer err {e:?}");
            }
            gl::PixelStorei(gl::PACK_ALIGNMENT, 4);
            if let e = gl::GetError()
                && e != 0
            {
                println!("glPixelStorei err {e:?}");
            }
            gl::PixelStorei(gl::PACK_ROW_LENGTH, 0);
            if let e = gl::GetError()
                && e != 0
            {
                println!("glPixelStorei err {e:?}");
            }
            gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
            if let e = gl::GetError()
                && e != 0
            {
                println!("glBindBuffer err {e:?}");
            }
            gl::ReadBuffer(gl::BACK);
            if let e = gl::GetError()
                && e != 0
            {
                println!("glReadBuffer err {e:?}");
            }
            gl::ReadPixels(
                0,
                0,
                self.w.cast_signed(), // w > 0, w not bigger than 2^31
                self.h.cast_signed(),
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                fb.as_mut_ptr().cast(),
            );
            for pix in fb.as_chunks_mut::<4>().0 {
                assert_eq!(pix.len(), 4);
                pix.swap(0, 3);

                pix.swap(1, 2);
            }
            for line in 0..(self.h as usize / 2) {
                let pitch = (self.w * 4) as usize;
                let line_n = self.h as usize - 1 - line;
                if line_n == line {
                    break;
                }
                let line0_start = line * pitch;
                let line0_end = line0_start + pitch;
                let line_n_start = line_n * pitch;
                let line_n_end = line_n_start + pitch;
                let [l0, l_n] = fb
                    .get_disjoint_mut([line0_start..line0_end, line_n_start..line_n_end])
                    .unwrap();
                l0.swap_with_slice(l_n);
            }
            if let e = gl::GetError()
                && e != 0
            {
                println!("glReadPixels err {e:?}");
            }
            // self.unbind();
        }
    }
}

unsafe impl Send for GlGfxData {}

impl Drop for GlGfxData {
    fn drop(&mut self) {
        self.unbind();
        self.destroy_surface();
        if let Err(e) = self.device.destroy_context(&mut self.context) {
            println!("Error destroying context {e:?}");
        }
    }
}
#[derive(Default)]
pub struct GlGfx {
    context_reset: gisst_rust_libretro_sys::retro_hw_context_reset_t,
    context_destroy: gisst_rust_libretro_sys::retro_hw_context_reset_t,
}

impl Gfx for GlGfx {
    fn preferred_api(&self) -> retro_hw_context_type {
        retro_hw_context_type::RETRO_HW_CONTEXT_OPENGL
    }
    fn video_refresh(&mut self, w: u32, h: u32, _p: usize) {
        let mut lock = GFX.lock().unwrap();
        let ctx = lock.as_mut().unwrap();
        let changed = if ctx.w != w || ctx.h != h {
            ctx.set_dimensions(w, h);
            true
        } else {
            false
        };
        drop(lock);
        if let Some(cb) = self.context_reset.as_ref()
            && changed
        {
            unsafe {
                cb();
            }
        }
    }
    fn prepare_hardware_context(
        &mut self,
        av: retro_system_av_info,
        cb: &mut retro_hw_render_callback,
    ) -> bool {
        let ctype = cb.context_type;
        cb.version_major = 4;
        cb.version_minor = 6;
        if ctype == retro_hw_context_type::RETRO_HW_CONTEXT_VULKAN
            || ctype == retro_hw_context_type::RETRO_HW_CONTEXT_DIRECT3D
        {
            println!("Tried to load unsupported gfx context type {ctype:?}");
            return false;
        }
        let w = av.geometry.max_width;
        let h = av.geometry.max_height;
        let mut lock = GFX.lock().unwrap();
        let ctx = GlGfxData::create(
            w,
            h,
            cb.version_major.try_into().unwrap(),
            cb.version_minor.try_into().unwrap(),
        );
        let success = ctx.is_some();
        let _ = std::mem::replace(&mut *lock, ctx);
        drop(lock);
        cb.bottom_left_origin = false;
        cb.cache_context = true;
        cb.get_proc_address = Some(get_proc_address);
        cb.get_current_framebuffer = Some(get_current_framebuffer);
        self.context_reset = cb.context_reset;
        self.context_destroy = cb.context_destroy;
        success
    }
    fn destroy_context(&mut self) {
        let mut lock = GFX.lock().unwrap();
        let ctx = lock.as_mut().unwrap();
        ctx.destroy_surface();
        if let Err(e) = ctx.device.destroy_context(&mut ctx.context) {
            println!("Error destroying context {e:?}");
        }
    }
    fn bind(&mut self) {
        let mut lock = GFX.lock().unwrap();
        let ctx = lock.as_mut().unwrap();
        ctx.bind();
    }
    fn unbind(&mut self) {
        let mut lock = GFX.lock().unwrap();
        let ctx = lock.as_mut().unwrap();
        ctx.unbind();
    }
    fn sync_framebuffer(&self, fb: &mut [u8]) {
        let mut lock = GFX.lock().unwrap();
        let ctx = lock.as_mut().unwrap();
        ctx.sync_framebuffer(fb);
    }
}
impl Drop for GlGfx {
    fn drop(&mut self) {
        let mut lock = GFX.lock().unwrap();
        let _ctx = lock.take();
    }
}
