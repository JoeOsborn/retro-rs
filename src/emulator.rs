use crate::buttons::Buttons;
use crate::error::RetroRsError;
use crate::gfx::Gfx;
use crate::pixels::{argb555to888, rgb565to888, rgb888_to_rgb332};

use libloading::Library;
use libloading::Symbol;
#[allow(clippy::wildcard_imports)]
use rust_libretro_sys::*;
use std::ffi::{CStr, CString, c_char, c_uint, c_void};
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::panic;
use std::path::{Path, PathBuf};
use std::ptr;

unsafe extern "C" {
    fn retrors_log_print(lev: retro_log_level, fmt: *const i8, ...);
}

thread_local! {
    static CTX:std::cell::RefCell<Option<EmulatorContext>> = const{std::cell::RefCell::new(None)};
}

type NotSendSync = *const [u8; 0];
struct EmulatorCore {
    core_lib: Library,
    rom_path: CString,
    core: CoreFns,
    _marker: PhantomData<NotSendSync>,
}

#[allow(dead_code, clippy::struct_field_names)]
struct CoreFns {
    retro_api_version: unsafe extern "C" fn() -> c_uint,
    retro_cheat_reset: unsafe extern "C" fn(),
    retro_cheat_set: unsafe extern "C" fn(c_uint, bool, *const c_char),
    retro_deinit: unsafe extern "C" fn(),
    retro_get_memory_data: unsafe extern "C" fn(c_uint) -> *mut c_void,
    retro_get_memory_size: unsafe extern "C" fn(c_uint) -> usize,
    retro_get_region: unsafe extern "C" fn() -> c_uint,
    retro_get_system_av_info: unsafe extern "C" fn(*mut retro_system_av_info),
    retro_get_system_info: unsafe extern "C" fn(*mut retro_system_info),
    retro_init: unsafe extern "C" fn(),
    retro_load_game: unsafe extern "C" fn(*const retro_game_info) -> bool,
    retro_load_game_special: unsafe extern "C" fn(c_uint, *const retro_game_info, usize) -> bool,
    retro_reset: unsafe extern "C" fn(),
    retro_run: unsafe extern "C" fn(),
    retro_serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    retro_serialize_size: unsafe extern "C" fn() -> usize,
    retro_set_audio_sample: unsafe extern "C" fn(retro_audio_sample_t),
    retro_set_audio_sample_batch: unsafe extern "C" fn(retro_audio_sample_batch_t),
    retro_set_controller_port_device: unsafe extern "C" fn(c_uint, c_uint),
    retro_set_environment: unsafe extern "C" fn(retro_environment_t),
    retro_set_input_poll: unsafe extern "C" fn(retro_input_poll_t),
    retro_set_input_state: unsafe extern "C" fn(retro_input_state_t),
    retro_set_video_refresh: unsafe extern "C" fn(retro_video_refresh_t),
    retro_unload_game: unsafe extern "C" fn(),
    retro_unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
}

pub type ButtonCallback = Box<dyn Fn(u32, u32, u32, u32) -> i16>;

#[allow(dead_code)]
struct EmulatorContext {
    audio_sample: Vec<i16>,
    buttons: [Buttons; 2],
    button_callback: Option<ButtonCallback>,
    core_path: CString,
    frame_ptr: *const c_void,
    frame_pitch: usize,
    frame_width: u32,
    frame_height: u32,
    pixfmt: retro_pixel_format,
    image_depth: usize,
    memory_map: Vec<retro_memory_descriptor>,
    av_info: retro_system_av_info,
    sys_info: retro_system_info,
    gfx: Box<dyn Gfx>,
    _marker: PhantomData<NotSendSync>,
}

// A more pleasant wrapper over MemoryDescriptor
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryRegion {
    which: usize,
    pub flags: u64,
    pub len: usize,
    pub start: usize,
    pub offset: usize,
    pub name: String,
    pub select: usize,
    pub disconnect: usize,
}

pub struct Emulator {
    core: EmulatorCore,
}

impl Emulator {
    /// # Panics
    /// If the platform is not Windows, Mac, or Linux; if the dylib fails to load successfully; if any Emulator has been created on this thread but not yet dropped.
    #[must_use]
    pub fn create(core_path: &Path, rom_path: &Path) -> Emulator {
        Self::create_with_gfx(core_path, rom_path, Box::new(crate::SoftwareGfx::default()))
    }
    /// # Panics
    /// If the platform is not Windows, Mac, or Linux; if the dylib fails to load successfully; if any Emulator has been created on this thread but not yet dropped.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn create_with_gfx(core_path: &Path, rom_path: &Path, gfx: Box<dyn Gfx>) -> Emulator {
        let emu = CTX.with_borrow_mut(move |ctx_opt| {
            assert!(
                ctx_opt.is_none(),
                "Can't use multiple emulators in one thread currently"
            );
            let suffix = if cfg!(target_os = "windows") {
                "dll"
            } else if cfg!(target_os = "macos") {
                "dylib"
            } else if cfg!(target_os = "linux") {
                "so"
            } else {
                panic!("Unsupported platform")
            };
            let path: PathBuf = core_path.with_extension(suffix);
            let core_path = core_path.parent().unwrap();
            #[cfg(target_os = "linux")]
            let dll: Library = unsafe {
                use libc::RTLD_NODELETE;
                use libloading::os::unix::{self, RTLD_LOCAL, RTLD_NOW};
                // Load library with `RTLD_NOW | RTLD_LOCAL | RTLD_NODELETE` to fix a SIGSEGV
                unix::Library::open(Some(path), RTLD_NOW | RTLD_LOCAL | RTLD_NODELETE)
                    .unwrap()
                    .into()
            };
            #[cfg(not(target_os = "linux"))]
            let dll = unsafe { Library::new(path).unwrap() };
            unsafe {
                let retro_set_environment = *(dll.get(b"retro_set_environment").unwrap());
                let retro_set_video_refresh = *(dll.get(b"retro_set_video_refresh").unwrap());
                let retro_set_audio_sample = *(dll.get(b"retro_set_audio_sample").unwrap());
                let retro_set_audio_sample_batch =
                    *(dll.get(b"retro_set_audio_sample_batch").unwrap());
                let retro_set_input_poll = *(dll.get(b"retro_set_input_poll").unwrap());
                let retro_set_input_state = *(dll.get(b"retro_set_input_state").unwrap());
                let retro_init = *(dll.get(b"retro_init").unwrap());
                let retro_deinit = *(dll.get(b"retro_deinit").unwrap());
                let retro_api_version = *(dll.get(b"retro_api_version").unwrap());
                let retro_get_system_info = *(dll.get(b"retro_get_system_info").unwrap());
                let retro_get_system_av_info = *(dll.get(b"retro_get_system_av_info").unwrap());
                let retro_set_controller_port_device =
                    *(dll.get(b"retro_set_controller_port_device").unwrap());
                let retro_reset = *(dll.get(b"retro_reset").unwrap());
                let retro_run = *(dll.get(b"retro_run").unwrap());
                let retro_serialize_size = *(dll.get(b"retro_serialize_size").unwrap());
                let retro_serialize = *(dll.get(b"retro_serialize").unwrap());
                let retro_unserialize = *(dll.get(b"retro_unserialize").unwrap());
                let retro_cheat_reset = *(dll.get(b"retro_cheat_reset").unwrap());
                let retro_cheat_set = *(dll.get(b"retro_cheat_set").unwrap());
                let retro_load_game = *(dll.get(b"retro_load_game").unwrap());
                let retro_load_game_special = *(dll.get(b"retro_load_game_special").unwrap());
                let retro_unload_game = *(dll.get(b"retro_unload_game").unwrap());
                let retro_get_region = *(dll.get(b"retro_get_region").unwrap());
                let retro_get_memory_data = *(dll.get(b"retro_get_memory_data").unwrap());
                let retro_get_memory_size = *(dll.get(b"retro_get_memory_size").unwrap());
                let emu = EmulatorCore {
                    core_lib: dll,
                    rom_path: CString::new(rom_path.to_str().unwrap()).unwrap(),
                    core: CoreFns {
                        retro_api_version,
                        retro_cheat_reset,
                        retro_cheat_set,
                        retro_deinit,
                        retro_get_memory_data,
                        retro_get_memory_size,

                        retro_get_region,
                        retro_get_system_av_info,

                        retro_get_system_info,

                        retro_init,
                        retro_load_game,
                        retro_load_game_special,

                        retro_reset,
                        retro_run,

                        retro_serialize,
                        retro_serialize_size,
                        retro_set_audio_sample,

                        retro_set_audio_sample_batch,
                        retro_set_controller_port_device,

                        retro_set_environment,
                        retro_set_input_poll,
                        retro_set_input_state,

                        retro_set_video_refresh,
                        retro_unload_game,
                        retro_unserialize,
                    },
                    _marker: PhantomData,
                };
                let sys_info = retro_system_info {
                    library_name: ptr::null(),
                    library_version: ptr::null(),
                    valid_extensions: ptr::null(),
                    need_fullpath: false,
                    block_extract: false,
                };
                let av_info = retro_system_av_info {
                    geometry: retro_game_geometry {
                        base_width: 0,
                        base_height: 0,
                        max_width: 0,
                        max_height: 0,
                        aspect_ratio: 0.0,
                    },
                    timing: retro_system_timing {
                        fps: 0.0,
                        sample_rate: 0.0,
                    },
                };

                let ctx = EmulatorContext {
                    av_info,
                    sys_info,
                    core_path: CString::new(core_path.to_str().unwrap()).unwrap(),
                    audio_sample: Vec::new(),
                    buttons: [Buttons::new(), Buttons::new()],
                    button_callback: None,
                    frame_ptr: ptr::null(),
                    frame_pitch: 0,
                    frame_width: 0,
                    frame_height: 0,
                    pixfmt: retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555,
                    image_depth: 0,
                    memory_map: Vec::new(),
                    gfx,
                    _marker: PhantomData,
                };
                *ctx_opt = Some(ctx);
                emu
            }
        });
        unsafe {
            // Set up callbacks
            (emu.core.retro_set_environment)(Some(callback_environment));
            (emu.core.retro_set_video_refresh)(Some(callback_video_refresh));
            (emu.core.retro_set_audio_sample)(Some(callback_audio_sample));
            (emu.core.retro_set_audio_sample_batch)(Some(callback_audio_sample_batch));
            (emu.core.retro_set_input_poll)(Some(callback_input_poll));
            (emu.core.retro_set_input_state)(Some(callback_input_state));
            // Load the core and game
            (emu.core.retro_init)();
            let rom_cstr = emu.rom_path.clone();
            let mut rom_file = File::open(rom_path).unwrap();
            let mut buffer = Vec::new();
            rom_file.read_to_end(&mut buffer).unwrap();
            buffer.shrink_to_fit();
            let game_info = retro_game_info {
                path: rom_cstr.as_ptr(),
                data: buffer.as_ptr().cast(),
                size: buffer.len(),
                meta: ptr::null(),
            };
            (emu.core.retro_load_game)(&raw const game_info);
            CTX.with_borrow_mut(|ctx| {
                let ctx = ctx.as_mut().unwrap();
                (emu.core.retro_get_system_info)(&raw mut ctx.sys_info);
                (emu.core.retro_get_system_av_info)(&raw mut ctx.av_info);
            });
        }
        Emulator { core: emu }
    }
    pub fn get_library(&mut self) -> &Library {
        &self.core.core_lib
    }
    #[must_use]
    pub fn get_symbol<'a, T>(&'a self, symbol: &[u8]) -> Option<Symbol<'a, T>> {
        let dll = &self.core.core_lib;
        let sym: Result<Symbol<T>, _> = unsafe { dll.get(symbol) };
        sym.ok()
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn run(&mut self, inputs: [Buttons; 2]) {
        CTX.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().unwrap();
            //clear audio buffers and whatever else
            ctx.audio_sample.clear();
            //set inputs on CB
            ctx.buttons = inputs;
            ctx.button_callback = None;
            ctx.gfx.bind();
        });
        unsafe {
            //run one step
            (self.core.core.retro_run)();
        }
        CTX.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().unwrap();
            ctx.gfx.unbind();
        });
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn run_with_button_callback(&mut self, input: Box<dyn Fn(u32, u32, u32, u32) -> i16>) {
        CTX.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().unwrap();
            //clear audio buffers and whatever else
            ctx.audio_sample.clear();
            //set inputs on CB
            ctx.button_callback = Some(Box::new(input));
            ctx.gfx.bind();
        });
        unsafe {
            //run one step
            (self.core.core.retro_run)();
        }
        CTX.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().unwrap();
            ctx.gfx.unbind();
        });
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn reset(&mut self) {
        CTX.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().unwrap();
            // clear audio buffers and whatever else
            ctx.audio_sample.clear();
            // set inputs on CB
            ctx.buttons = [Buttons::new(), Buttons::new()];
            ctx.button_callback = None;
            // clear fb
            ctx.frame_ptr = ptr::null();
        });
        unsafe { (self.core.core.retro_reset)() }
    }
    #[must_use]
    fn get_ram_size(&self, rtype: libc::c_uint) -> usize {
        unsafe { (self.core.core.retro_get_memory_size)(rtype) }
    }
    #[must_use]
    pub fn get_video_ram_size(&self) -> usize {
        self.get_ram_size(RETRO_MEMORY_VIDEO_RAM)
    }
    #[must_use]
    pub fn get_system_ram_size(&self) -> usize {
        self.get_ram_size(RETRO_MEMORY_SYSTEM_RAM)
    }
    #[must_use]
    pub fn get_save_ram_size(&self) -> usize {
        self.get_ram_size(RETRO_MEMORY_SAVE_RAM)
    }
    #[must_use]
    pub fn video_ram_ref(&self) -> &[u8] {
        self.get_ram(RETRO_MEMORY_VIDEO_RAM)
    }
    #[must_use]
    pub fn system_ram_ref(&self) -> &[u8] {
        self.get_ram(RETRO_MEMORY_SYSTEM_RAM)
    }
    #[must_use]
    pub fn system_ram_mut(&mut self) -> &mut [u8] {
        self.get_ram_mut(RETRO_MEMORY_SYSTEM_RAM)
    }
    #[must_use]
    pub fn save_ram(&self) -> &[u8] {
        self.get_ram(RETRO_MEMORY_SAVE_RAM)
    }

    #[must_use]
    fn get_ram(&self, ramtype: libc::c_uint) -> &[u8] {
        let len = self.get_ram_size(ramtype);
        unsafe {
            let ptr: *const u8 = (self.core.core.retro_get_memory_data)(ramtype).cast();
            std::slice::from_raw_parts(ptr, len)
        }
    }
    #[must_use]
    fn get_ram_mut(&mut self, ramtype: libc::c_uint) -> &mut [u8] {
        let len = self.get_ram_size(ramtype);
        unsafe {
            let ptr: *mut u8 = (self.core.core.retro_get_memory_data)(ramtype).cast();
            std::slice::from_raw_parts_mut(ptr, len)
        }
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    #[must_use]
    pub fn memory_regions(&self) -> Vec<MemoryRegion> {
        CTX.with_borrow(|ctx| {
            let map = &ctx.as_ref().unwrap().memory_map;
            map.iter()
                .enumerate()
                .map(|(i, mdesc)| MemoryRegion {
                    which: i,
                    flags: mdesc.flags,
                    len: mdesc.len,
                    start: mdesc.start,
                    offset: mdesc.offset,
                    select: mdesc.select,
                    disconnect: mdesc.disconnect,
                    name: if mdesc.addrspace.is_null() {
                        String::new()
                    } else {
                        unsafe { CStr::from_ptr(mdesc.addrspace) }
                            .to_string_lossy()
                            .into_owned()
                    },
                })
                .collect()
        })
    }
    /// # Errors
    /// [`RetroRsError::RAMCopyNotMappedIntoMemoryRegionError`]: Returns an error if the desired address is not mapped into memory regions
    /// # Panics
    /// If called on a thread without a running emulator core
    pub fn memory_ref(&self, start: usize) -> Result<&[u8], RetroRsError> {
        for mr in self.memory_regions() {
            if mr.select != 0 && (start & mr.select) == 0 {
                continue;
            }
            if start >= mr.start && start < mr.start + mr.len {
                return CTX.with_borrow(|ctx| {
                    let maps = &ctx.as_ref().unwrap().memory_map;
                    if mr.which >= maps.len() {
                        // TODO more aggressive checking of mr vs map
                        return Err(RetroRsError::RAMMapOutOfRangeError);
                    }
                    let start = (start - mr.start) & !mr.disconnect;
                    let map = &maps[mr.which];
                    //0-based at this point, modulo offset
                    let ptr: *mut u8 = map.ptr.cast();
                    let slice = unsafe {
                        let ptr = ptr.add(start).add(map.offset);
                        std::slice::from_raw_parts(ptr, map.len - start)
                    };
                    Ok(slice)
                });
            } else if start < mr.start {
                return Err(RetroRsError::RAMCopySrcOutOfBoundsError);
            }
        }
        Err(RetroRsError::RAMCopyNotMappedIntoMemoryRegionError)
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    /// # Errors
    /// [`RetroRsError::RAMMapOutOfRangeError`]: The desired address is out of mapped range
    /// [`RetroRsError::RAMCopySrcOutOfBoundsError`]: The desired range is not in the requested region
    pub fn memory_ref_mut(
        &mut self,
        mr: &MemoryRegion,
        start: usize,
    ) -> Result<&mut [u8], RetroRsError> {
        CTX.with_borrow_mut(|ctx| {
            let maps = &mut ctx.as_mut().unwrap().memory_map;
            if mr.which >= maps.len() {
                // TODO more aggressive checking of mr vs map
                return Err(RetroRsError::RAMMapOutOfRangeError);
            }
            if start < mr.start {
                return Err(RetroRsError::RAMCopySrcOutOfBoundsError);
            }
            let start = (start - mr.start) & !mr.disconnect;
            let map = &maps[mr.which];
            //0-based at this point, modulo offset
            let ptr: *mut u8 = map.ptr.cast();
            let slice = unsafe {
                let ptr = ptr.add(start).add(map.offset);
                std::slice::from_raw_parts_mut(ptr, map.len - start)
            };
            Ok(slice)
        })
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    #[must_use]
    pub fn pixel_format(&self) -> retro_pixel_format {
        CTX.with_borrow(|ctx| ctx.as_ref().unwrap().pixfmt)
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    #[must_use]
    pub fn framebuffer_size(&self) -> (usize, usize) {
        CTX.with_borrow(|ctx| {
            let ctx = ctx.as_ref().unwrap();
            (ctx.frame_width as usize, ctx.frame_height as usize)
        })
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    #[must_use]
    pub fn framebuffer_pitch(&self) -> usize {
        CTX.with_borrow(|ctx| ctx.as_ref().unwrap().frame_pitch)
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn peek_framebuffer<FBPeek, FBPeekRet>(&self, f: FBPeek) -> Result<FBPeekRet, RetroRsError>
    where
        FBPeek: FnOnce(&[u8]) -> FBPeekRet,
    {
        CTX.with_borrow(|ctx| {
            let ctx = ctx.as_ref().unwrap();
            if ctx.frame_ptr.is_null() {
                Err(RetroRsError::NoFramebufferError)
            } else {
                unsafe {
                    ctx.gfx.sync_framebuffer(std::slice::from_raw_parts_mut(
                        ctx.frame_ptr.cast_mut().cast(),
                        ctx.frame_pitch * ctx.frame_height as usize,
                    ));
                    #[allow(clippy::cast_possible_truncation)]
                    let frame_slice = std::slice::from_raw_parts(
                        ctx.frame_ptr.cast(),
                        (ctx.frame_height * (ctx.frame_pitch as u32)) as usize,
                    );
                    Ok(f(frame_slice))
                }
            }
        })
    }
    #[allow(clippy::missing_panics_doc, clippy::unused_self)]
    pub fn peek_audio_sample<AudioPeek, AudioPeekRet>(&self, f: AudioPeek) -> AudioPeekRet
    where
        AudioPeek: FnOnce(&[i16]) -> AudioPeekRet,
    {
        CTX.with_borrow(|ctx| f(&ctx.as_ref().unwrap().audio_sample))
    }
    /// # Panics
    /// If called on a thread without a running emulator core
    #[must_use]
    pub fn get_audio_sample_rate(&self) -> f64 {
        CTX.with_borrow_mut(|ctx| ctx.as_ref().unwrap().av_info.timing.sample_rate)
    }
    /// # Panics
    /// If called on a thread without a running emulator core
    #[must_use]
    pub fn get_video_fps(&self) -> f64 {
        CTX.with_borrow_mut(|ctx| ctx.as_ref().unwrap().av_info.timing.fps)
    }
    #[must_use]
    pub fn get_aspect_ratio(&self) -> f32 {
        CTX.with_borrow_mut(|ctx| ctx.as_ref().unwrap().av_info.geometry.aspect_ratio)
    }

    #[must_use]
    pub fn save(&self, bytes: &mut [u8]) -> bool {
        let size = self.save_size();
        if bytes.len() < size {
            return false;
        }
        unsafe { (self.core.core.retro_serialize)(bytes.as_mut_ptr().cast(), size) }
    }
    #[must_use]
    pub fn load(&mut self, bytes: &[u8]) -> bool {
        let size = self.save_size();
        if bytes.len() < size {
            return false;
        }
        unsafe { (self.core.core.retro_unserialize)(bytes.as_ptr().cast(), size) }
    }
    #[must_use]
    pub fn save_size(&self) -> usize {
        unsafe { (self.core.core.retro_serialize_size)() }
    }
    pub fn clear_cheats(&mut self) {
        unsafe { (self.core.core.retro_cheat_reset)() }
    }
    /// # Panics
    /// May panic if code can't be converted to a [`CString`]
    pub fn set_cheat(&mut self, index: usize, enabled: bool, code: &str) {
        unsafe {
            // FIXME: Creates a memory leak since the libretro api won't let me from_raw() it back and drop it.  I don't know if libretro guarantees anything about ownership of this str to cores.
            #[allow(clippy::cast_possible_truncation)]
            (self.core.core.retro_cheat_set)(
                index as u32,
                enabled,
                CString::new(code).unwrap().into_raw(),
            );
        }
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn get_pixel(&self, x: usize, y: usize) -> Result<(u8, u8, u8), RetroRsError> {
        let (w, _h) = self.framebuffer_size();
        self.peek_framebuffer(move |fb| match self.pixel_format() {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                let start = y * w + x;
                let gb = fb[start * 2];
                let arg = fb[start * 2 + 1];
                let (red, green, blue) = argb555to888(gb, arg);
                (red, green, blue)
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                let off = (y * w + x) * 4;
                (fb[off + 1], fb[off + 2], fb[off + 3])
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                let start = y * w + x;
                let gb = fb[start * 2];
                let rg = fb[start * 2 + 1];
                let (red, green, blue) = rgb565to888(gb, rg);
                (red, green, blue)
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    #[allow(clippy::many_single_char_names)]
    pub fn for_each_pixel(
        &self,
        mut f: impl FnMut(usize, usize, u8, u8, u8),
    ) -> Result<(), RetroRsError> {
        let (w, h) = self.framebuffer_size();
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| {
            let mut x = 0;
            let mut y = 0;
            match fmt {
                retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                    for components in fb.chunks_exact(2) {
                        let gb = components[0];
                        let arg = components[1];
                        let (red, green, blue) = argb555to888(gb, arg);
                        f(x, y, red, green, blue);
                        x += 1;
                        if x >= w {
                            y += 1;
                            x = 0;
                        }
                    }
                }
                retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                    for components in fb.chunks_exact(4) {
                        let red = components[1];
                        let green = components[2];
                        let blue = components[3];
                        f(x, y, red, green, blue);
                        x += 1;
                        if x >= w {
                            y += 1;
                            x = 0;
                        }
                    }
                }
                retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                    for components in fb.chunks_exact(2) {
                        let gb = components[0];
                        let rg = components[1];
                        let (red, green, blue) = rgb565to888(gb, rg);
                        f(x, y, red, green, blue);
                        x += 1;
                        if x >= w {
                            y += 1;
                            x = 0;
                        }
                    }
                }
                _ => panic!("Unsupported pixel format"),
            }
            assert_eq!(y, h);
            assert_eq!(x, 0);
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn copy_framebuffer_rgb888(&self, slice: &mut [u8]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| match fmt {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(3)) {
                    let gb = components[0];
                    let arg = components[1];
                    let (red, green, blue) = argb555to888(gb, arg);
                    dst[0] = red;
                    dst[1] = green;
                    dst[2] = blue;
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                for (components, dst) in fb.chunks_exact(4).zip(slice.chunks_exact_mut(3)) {
                    let r = components[1];
                    let g = components[2];
                    let b = components[3];
                    dst[0] = r;
                    dst[1] = g;
                    dst[2] = b;
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(3)) {
                    let gb = components[0];
                    let rg = components[1];
                    let (red, green, blue) = rgb565to888(gb, rg);
                    dst[0] = red;
                    dst[1] = green;
                    dst[2] = blue;
                }
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn copy_framebuffer_rgba8888(&self, slice: &mut [u8]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| match fmt {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(4)) {
                    let gb = components[0];
                    let arg = components[1];
                    let (red, green, blue) = argb555to888(gb, arg);
                    dst[0] = red;
                    dst[1] = green;
                    dst[2] = blue;
                    dst[3] = (arg >> 7) * 0xFF;
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                for (components, dst) in fb.chunks_exact(4).zip(slice.chunks_exact_mut(4)) {
                    let a = components[0];
                    let r = components[1];
                    let g = components[2];
                    let b = components[3];
                    dst[0] = r;
                    dst[1] = g;
                    dst[2] = b;
                    dst[3] = a;
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(4)) {
                    let gb = components[0];
                    let rg = components[1];
                    let (red, green, blue) = rgb565to888(gb, rg);
                    dst[0] = red;
                    dst[1] = green;
                    dst[2] = blue;
                    dst[3] = 0xFF;
                }
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn copy_framebuffer_rgb332(&self, slice: &mut [u8]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| match fmt {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                    let gb = components[0];
                    let arg = components[1];
                    let (red, green, blue) = argb555to888(gb, arg);
                    *dst = rgb888_to_rgb332(red, green, blue);
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                for (components, dst) in fb.chunks_exact(4).zip(slice.iter_mut()) {
                    let r = components[1];
                    let g = components[2];
                    let b = components[3];
                    *dst = rgb888_to_rgb332(r, g, b);
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                    let gb = components[0];
                    let rg = components[1];
                    let (red, green, blue) = rgb565to888(gb, rg);
                    *dst = rgb888_to_rgb332(red, green, blue);
                }
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn copy_framebuffer_argb32(&self, slice: &mut [u32]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| match fmt {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                    let gb = components[0];
                    let arg = components[1];
                    let (red, green, blue) = argb555to888(gb, arg);
                    *dst = (0xFF00_0000 * (u32::from(arg) >> 7))
                        | (u32::from(red) << 16)
                        | (u32::from(green) << 8)
                        | u32::from(blue);
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                for (components, dst) in fb.chunks_exact(4).zip(slice.iter_mut()) {
                    *dst = (u32::from(components[0]) << 24)
                        | (u32::from(components[1]) << 16)
                        | (u32::from(components[2]) << 8)
                        | u32::from(components[3]);
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                    let gb = components[0];
                    let rg = components[1];
                    let (red, green, blue) = rgb565to888(gb, rg);
                    *dst = 0xFF00_0000
                        | (u32::from(red) << 16)
                        | (u32::from(green) << 8)
                        | u32::from(blue);
                }
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn copy_framebuffer_rgba32(&self, slice: &mut [u32]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| match fmt {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                    let gb = components[0];
                    let arg = components[1];
                    let (red, green, blue) = argb555to888(gb, arg);
                    *dst = (u32::from(red) << 24)
                        | (u32::from(green) << 16)
                        | (u32::from(blue) << 8)
                        | (u32::from(arg >> 7) * 0x0000_00FF);
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                for (components, dst) in fb.chunks_exact(4).zip(slice.iter_mut()) {
                    *dst = (u32::from(components[1]) << 24)
                        | (u32::from(components[2]) << 16)
                        | (u32::from(components[3]) << 8)
                        | u32::from(components[0]);
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                    let gb = components[0];
                    let rg = components[1];
                    let (red, green, blue) = rgb565to888(gb, rg);
                    *dst = (u32::from(red) << 24)
                        | (u32::from(green) << 16)
                        | (u32::from(blue) << 8)
                        | 0x0000_00FF;
                }
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
    /// # Panics
    /// Panics if the pixel format used by the core is not supported for reads.
    /// # Errors
    /// [`RetroRsError::NoFramebufferError`]: Emulator has not created a framebuffer.
    pub fn copy_framebuffer_rgba_f32x4(&self, slice: &mut [f32]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| match fmt {
            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(4)) {
                    let gb = components[0];
                    let arg = components[1];
                    let (red, green, blue) = argb555to888(gb, arg);
                    let alpha = f32::from(arg >> 7);
                    dst[0] = f32::from(red) / 255.;
                    dst[1] = f32::from(green) / 255.;
                    dst[2] = f32::from(blue) / 255.;
                    dst[3] = alpha;
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => {
                for (components, dst) in fb.chunks_exact(4).zip(slice.chunks_exact_mut(4)) {
                    dst[0] = f32::from(components[0]) / 255.;
                    dst[1] = f32::from(components[1]) / 255.;
                    dst[2] = f32::from(components[2]) / 255.;
                    dst[3] = f32::from(components[3]) / 255.;
                }
            }
            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => {
                for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(4)) {
                    let gb = components[0];
                    let rg = components[1];
                    let (red, green, blue) = rgb565to888(gb, rg);
                    let alpha = 1.;
                    dst[0] = f32::from(red) / 255.;
                    dst[1] = f32::from(green) / 255.;
                    dst[2] = f32::from(blue) / 255.;
                    dst[3] = alpha;
                }
            }
            _ => panic!("Unsupported pixel format"),
        })
    }
}

unsafe extern "C" fn callback_environment(cmd: u32, data: *mut c_void) -> bool {
    let result = panic::catch_unwind(|| {
        CTX.with_borrow_mut(|ctx| {
                let ctx = ctx.as_mut().unwrap();
                match cmd {
                    RETRO_ENVIRONMENT_SET_CONTROLLER_INFO => true,
                    RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
                        let pixfmt = unsafe { *(data as *const retro_pixel_format) };
                        dbg!(pixfmt);
                        ctx.image_depth = match pixfmt {
                            retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555 => 15,
                            retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888 => 32,
                            retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565 => 16,
                            _ => panic!("Unsupported pixel format"),
                        };
                        ctx.pixfmt = pixfmt;
                        true
                    }
                    RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY
                    | RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY => unsafe {
                        *(data.cast()) = ctx.core_path.as_ptr();
                        true
                    },
                    RETRO_ENVIRONMENT_GET_CAN_DUPE => unsafe {
                        *(data.cast()) = true;
                        true
                    },
                    RETRO_ENVIRONMENT_SET_MEMORY_MAPS => unsafe {
                        let map: *const retro_memory_map = data.cast();
                        let desc_slice = std::slice::from_raw_parts(
                            (*map).descriptors,
                            (*map).num_descriptors as usize,
                        );
                        // Don't know who owns map or how long it will last
                        ctx.memory_map = Vec::new();
                        // So we had better copy it
                        ctx.memory_map.extend_from_slice(desc_slice);
                        // (Implicitly we also want to drop the old one, which we did by reassigning)
                        true
                    },
                    RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER => unsafe {
                        *(data.cast()) = ctx.gfx.preferred_api() as c_uint;
                        true
                    },
                    RETRO_ENVIRONMENT_SET_HW_RENDER => unsafe {
                        /* todo create or provide opengl context */
                        let hw_render_cb: *mut retro_hw_render_callback = data.cast();
                        ctx.gfx
                            .prepare_hardware_context(ctx.av_info, hw_render_cb.as_mut().unwrap())
                    },
                    RETRO_ENVIRONMENT_GET_LOG_INTERFACE => unsafe {
                        let log_cb: *mut retro_log_callback = data.cast();
                        *log_cb = retro_log_callback {
                            log: Some(retrors_log_print),
                        };
                        true
                    },
                    RETRO_ENVIRONMENT_GET_VARIABLE => unsafe {
                        let var: *mut retro_variable = data.cast();
                        let var = var.as_mut().unwrap();
                        let key = CStr::from_ptr(var.key.cast()).to_str().unwrap();
                        #[allow(clippy::match_same_arms)]
                        match key {
                            "ppsspp_internal_resolution" => {
                                var.value = c"480x272".as_ptr().cast();
                                true
                            },
                            "ppsspp_backend" => {
                                var.value = c"opengl".as_ptr().cast();
                                true
                            },
                            "ppsspp_psp_model" => {
                                var.value = c"psp_2000_3000".as_ptr().cast();
                                true
                            },
                            "ppsspp_cache_iso" => { var.value =c"disabled".as_ptr().cast(); true },
                            "ppsspp_change_mac_address01" => { var.value =c"e".as_ptr().cast(); true },
                            "ppsspp_change_mac_address02" => { var.value =c"c".as_ptr().cast(); true },
                            "ppsspp_change_mac_address03" => { var.value =c"c".as_ptr().cast(); true },
                            "ppsspp_change_mac_address04" => { var.value =c"a".as_ptr().cast(); true },
                            "ppsspp_change_mac_address05" => { var.value =c"4".as_ptr().cast(); true },
                            "ppsspp_change_mac_address06" => { var.value =c"7".as_ptr().cast(); true },
                            "ppsspp_change_mac_address07" => { var.value =c"b".as_ptr().cast(); true },
                            "ppsspp_change_mac_address08" => { var.value =c"c".as_ptr().cast(); true },
                            "ppsspp_change_mac_address09" => { var.value =c"5".as_ptr().cast(); true },
                            "ppsspp_change_mac_address10" => { var.value =c"b".as_ptr().cast(); true },
                            "ppsspp_change_mac_address11" => { var.value =c"1".as_ptr().cast(); true },
                            "ppsspp_change_mac_address12" => { var.value =c"d".as_ptr().cast(); true },
                            _ => false,
                        }
                    },
                    RETRO_ENVIRONMENT_SHUTDOWN => {
                        ctx.gfx.destroy_context();
                        true
                    },
                    _ => false,
                }
            })
    });
    result.unwrap_or(false)
}

extern "C" fn callback_video_refresh(data: *const c_void, width: u32, height: u32, pitch: usize) {
    // Can't panic
    // context's framebuffer just points to the given data.  Seems to work OK for gym-retro.
    if !data.is_null() {
        CTX.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().unwrap();
            let pitch = if pitch == 0 {
                width as usize * 4
            } else {
                pitch
            };
            if data as isize == -1 {
                if width != ctx.frame_width
                    || height != ctx.frame_height
                    || pitch != ctx.frame_pitch
                {
                    unsafe {
                        if !ctx.frame_ptr.is_null() {
                            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                ctx.frame_ptr.cast::<u8>().cast_mut(),
                                ctx.frame_pitch * ctx.frame_height as usize,
                            )));
                        }
                        ctx.pixfmt = retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888;
                        println!(
                            "real size is {height} * {pitch} = {}",
                            height as usize * pitch
                        );
                        ctx.frame_ptr =
                            Box::leak(vec![255; height as usize * pitch].into_boxed_slice())
                                .as_ptr()
                                .cast();
                    }
                }
            } else {
                ctx.frame_ptr = data;
            }
            ctx.frame_pitch = pitch;
            ctx.frame_width = width;
            ctx.frame_height = height;
            ctx.gfx.video_refresh(width, height, pitch);
        });
    }
}
extern "C" fn callback_audio_sample(left: i16, right: i16) {
    // Can't panic
    CTX.with_borrow_mut(|ctx| {
        let ctx = ctx.as_mut().unwrap();
        let sample_buf = &mut ctx.audio_sample;
        sample_buf.push(left);
        sample_buf.push(right);
    });
}
extern "C" fn callback_audio_sample_batch(data: *const i16, frames: usize) -> usize {
    // Can't panic
    CTX.with_borrow_mut(|ctx| {
        let ctx = ctx.as_mut().unwrap();
        let sample_buf = &mut ctx.audio_sample;
        let slice = unsafe { std::slice::from_raw_parts(data, frames * 2) };
        sample_buf.extend_from_slice(slice);
        frames
    })
}

extern "C" fn callback_input_poll() {}

extern "C" fn callback_input_state(port: u32, device: u32, index: u32, id: u32) -> i16 {
    // Can't panic
    if port > 1 || device != 1 || index != 0 {
        // Unsupported port/device/index
        // println!("Unsupported port/device/index {port}/{device}/{index}");
        return 0;
    }
    let bitmask_enabled = (device == RETRO_DEVICE_JOYPAD) && (id == RETRO_DEVICE_ID_JOYPAD_MASK);
    CTX.with_borrow(|ctx| {
        let ctx = ctx.as_ref().unwrap();
        if let Some(cb) = &ctx.button_callback {
            cb(port, device, index, id)
        } else if bitmask_enabled {
            let port = port as usize;
            i16::from(ctx.buttons[port])
        } else {
            let port = port as usize;
            i16::from(ctx.buttons[port].get(id))
        }
    })
}

impl Drop for Emulator {
    fn drop(&mut self) {
        unsafe {
            (self.core.core.retro_unload_game)();
            (self.core.core.retro_deinit)();
        }
        CTX.with_borrow_mut(Option::take);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[cfg(feature = "use_image")]
    extern crate image;
    #[cfg(feature = "use_image")]
    use crate::fb_to_image::*;

    fn mario_is_dead(emu: &Emulator) -> bool {
        emu.system_ram_ref()[0x0770] == 0x03
    }

    // const PPU_BIT: usize = 1 << 31;

    // fn get_byte(emu: &Emulator, addr: usize) -> u8 {
    // emu.memory_ref(addr).expect("Couldn't read RAM!")[0]
    // }

    #[test]
    fn create_drop_create() {
        // TODO change to a public domain rom or maybe 2048 core or something
        let mut emu = Emulator::create(
            Path::new("../../.config/retroarch/cores/fceumm_libretro"),
            Path::new("roms/mario.nes"),
        );
        drop(emu);
        emu = Emulator::create(
            Path::new("../../.config/retroarch/cores/fceumm_libretro1"),
            Path::new("roms/mario.nes"),
        );
        drop(emu);
    }
    #[cfg(feature = "use_image")]
    #[test]
    fn it_works() {
        // TODO change to a public domain rom or maybe 2048 core or something
        let mut emu = Emulator::create(
            Path::new("../../.config/retroarch/cores/fceumm_libretro2"),
            Path::new("roms/mario.nes"),
        );
        emu.run([Buttons::new(), Buttons::new()]);
        emu.reset();
        for i in 0..150 {
            emu.run([
                Buttons::new()
                    .start(i > 80 && i < 100)
                    .right(i >= 100)
                    .a(i >= 100),
                Buttons::new(),
            ]);
        }
        let fb = emu.create_imagebuffer();
        fb.unwrap().save("out.png").unwrap();
        let mut died = false;
        for _ in 0..10000 {
            emu.run([Buttons::new().right(true), Buttons::new()]);
            if mario_is_dead(&emu) {
                died = true;
                let fb = emu.create_imagebuffer();
                fb.unwrap().save("out2.png").unwrap();
                break;
            }
        }
        assert!(died);
        emu.reset();
        for i in 0..250 {
            emu.run([
                Buttons::new()
                    .start(i > 80 && i < 100)
                    .right(i >= 100)
                    .a((100..=150).contains(&i) || (i >= 180)),
                Buttons::new(),
            ]);
        }

        //emu will drop naturally
    }
    #[test]
    fn it_works_with_callback() {
        // TODO change to a public domain rom or maybe 2048 core or something
        let mut emu = Emulator::create(
            Path::new("../../.config/retroarch/cores/fceumm_libretro3"),
            Path::new("roms/mario.nes"),
        );
        emu.run([Buttons::new(), Buttons::new()]);
        emu.reset();
        for i in 0..150 {
            emu.run_with_button_callback(Box::new(move |port, _dev, _idx, id| {
                if port == 0 {
                    let buttons = Buttons::new()
                        .start(i > 80 && i < 100)
                        .right(i >= 100)
                        .a((100..=150).contains(&i) || (i >= 180));
                    if id == RETRO_DEVICE_ID_JOYPAD_MASK {
                        i16::from(buttons)
                    } else {
                        i16::from(buttons.get(id))
                    }
                } else {
                    0
                }
            }));
        }
        let mut died = false;
        for _ in 0..10000 {
            emu.run_with_button_callback(Box::new(|_port, _dev, _idx, id| {
                let buttons = Buttons::new().right(true);
                if id == RETRO_DEVICE_ID_JOYPAD_MASK {
                    i16::from(buttons)
                } else {
                    i16::from(buttons.get(id))
                }
            }));
            if mario_is_dead(&emu) {
                died = true;
                break;
            }
        }
        assert!(died);
        //emu will drop naturally
    }
    #[test]
    fn hw_works() {
        // use renderdoc::RenderDoc;
        // let mut renderdoc: RenderDoc<renderdoc::V110> = RenderDoc::new().unwrap();
        let mut emu = Emulator::create_with_gfx(
            Path::new("cores/ppsspp_libretro"),
            Path::new("roms/patapon.iso"),
            Box::new(crate::GlGfx::default()),
        );
        emu.run([Buttons::new(), Buttons::new()]);
        for _ in 0..600 {
            emu.run([Buttons::new(), Buttons::new()]);
        }
        // renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
        emu.run([Buttons::new(), Buttons::new()]);
        // renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());
        let mut pixels = vec![255_u8; 480 * 272 * 4];
        emu.copy_framebuffer_rgba8888(&mut pixels).unwrap();

        image::save_buffer(
            "petscop.png",
            &pixels,
            480,
            272,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
        // loop {
        // std::thread::sleep_ms(500);
        // }
    }
}
