use crate::gfx::Gfx;
use rust_libretro_sys::{retro_hw_context_type, retro_hw_render_callback, retro_system_av_info};
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
) -> rust_libretro_sys::retro_proc_address_t {
    // dbg!(std::thread::current().id());
    unsafe {
        let proc_str = std::ffi::CStr::from_ptr(proc).to_str().unwrap();
        std::mem::transmute(get_proc_address_r(proc_str))
    }
}
pub unsafe extern "C" fn get_current_framebuffer() -> usize {
    // dbg!(std::thread::current().id());
    let ctx = GFX.lock().unwrap();
    ctx.as_ref().map_or(0, |ctx| ctx.get_fbo())
}

struct GlGfxData {
    w: i32,
    h: i32,
    context: surfman::Context,
    device: surfman::Device,
    surface: Option<surfman::Surface>,
    fbo: u32,
}
impl GlGfxData {
    fn get_fbo(&self) -> usize {
        self.fbo as usize
    }
    fn create(w: i32, h: i32, version_major: u8, version_minor: u8) -> Option<Self> {
        // dbg!(std::thread::current().id());
        let Ok(connection) = Connection::new() else {
            println!("Could not create connection");
            return None;
        };
        let Ok(adapter) = connection.create_adapter() else {
            println!("Could not obtain adapter");
            return None;
        };
        let Ok(mut device) = connection.create_device(&adapter) else {
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
                    size: euclid::Size2D::new(self.w, self.h),
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
    fn set_dimensions(&mut self, w: i32, h: i32) {
        if w == self.w && h == self.h && self.surface.is_some() {
            return;
        }
        self.destroy_surface();
        self.w = w;
        self.h = h;
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
        println!("Fb size {}", fb.len());
        unsafe {
            self.bind();
            let fbo = self.get_fbo();
            println!("Existing err {:x}, fbo {fbo}", gl::GetError());
            // gl::Flush();
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo as u32);
            println!("glBF err {:x}", gl::GetError());
            gl::PixelStorei(gl::PACK_ALIGNMENT, 4);
            println!("glPSi err {:x}", gl::GetError());
            gl::PixelStorei(gl::PACK_ROW_LENGTH, 0);
            println!("glPSi err {:x}", gl::GetError());
            gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
            println!("glBB err {:x}", gl::GetError());
            gl::ReadBuffer(gl::BACK);
            println!("glRB err {:x}", gl::GetError());
            gl::ReadPixels(
                0,
                0,
                self.w,
                self.h,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                fb.as_mut_ptr().cast(),
            );
            for pix in fb.chunks_exact_mut(4) {
                assert_eq!(pix.len(), 4);
                pix.swap(0, 3);
                pix.swap(1, 2);
            }

            println!("glReadPixels err {:x}", gl::GetError());
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
    context_reset: rust_libretro_sys::retro_hw_context_reset_t,
    context_destroy: rust_libretro_sys::retro_hw_context_reset_t,
}

impl Gfx for GlGfx {
    fn preferred_api(&self) -> retro_hw_context_type {
        retro_hw_context_type::RETRO_HW_CONTEXT_OPENGL
    }
    fn video_refresh(&mut self, w: u32, h: u32, _p: usize) {
        let Ok(w) = i32::try_from(w) else {
            println!("Bad width {w}");
            return;
        };
        let Ok(h) = i32::try_from(h) else {
            println!("Bad height {h}");
            return;
        };
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
        let w = i32::try_from(av.geometry.max_width).unwrap_or(-1);
        let h = i32::try_from(av.geometry.max_height).unwrap_or(-1);
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
        println!("Created ctx");
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
