use crate::buttons::Buttons;
use crate::error::*;
use crate::pixels::*;
use libc::c_char;
use libloading::Library;
use libloading::Symbol;
use libretro_sys::*;
use std::ffi::{c_void, CStr, CString};
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::panic;
use std::path::{Path, PathBuf};
use std::ptr;

type NotSendSync = *const [u8; 0];

static mut EMULATOR: *mut EmulatorCore = ptr::null_mut();
static mut CONTEXT: *mut EmulatorContext = ptr::null_mut();

struct EmulatorCore {
    core_lib: Box<Library>,
    core_path: CString,
    rom_path: CString,
    core: CoreAPI,
    _marker: PhantomData<NotSendSync>,
}

struct EmulatorContext {
    audio_sample: Vec<i16>,
    buttons: [Buttons; 2],
    frame_ptr: *const c_void,
    frame_pitch: usize,
    frame_width: u32,
    frame_height: u32,
    pixfmt: PixelFormat,
    image_depth: usize,
    memory_map: Vec<MemoryDescriptor>,
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

// Emulator token must not be send nor sync
pub struct Emulator {
    phantom: PhantomData<NotSendSync>,
}

impl Emulator {
    pub fn create(core_path: &Path, rom_path: &Path) -> Emulator {
        unsafe {
            assert!(EMULATOR.is_null());
            assert!(CONTEXT.is_null());
        }
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
        #[cfg(target_os = "linux")]
        let library: Library = {
            // Load library with `RTLD_NOW | RTLD_NODELETE` to fix a SIGSEGV
            ::libloading::os::unix::Library::open(Some(path), 0x2 | 0x1000)
                .unwrap()
                .into()
        };
        #[cfg(not(target_os = "linux"))]
        let library = Library::new(path).unwrap();
        let dll = Box::new(library);
        unsafe {
            let retro_set_environment = *(dll.get(b"retro_set_environment").unwrap());
            let retro_set_video_refresh = *(dll.get(b"retro_set_video_refresh").unwrap());
            let retro_set_audio_sample = *(dll.get(b"retro_set_audio_sample").unwrap());
            let retro_set_audio_sample_batch = *(dll.get(b"retro_set_audio_sample_batch").unwrap());
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
                core_path: CString::new(core_path.to_str().unwrap()).unwrap(),
                core: CoreAPI {
                    retro_set_environment,
                    retro_set_video_refresh,
                    retro_set_audio_sample,
                    retro_set_audio_sample_batch,
                    retro_set_input_poll,
                    retro_set_input_state,

                    retro_init,
                    retro_deinit,

                    retro_api_version,

                    retro_get_system_info,
                    retro_get_system_av_info,
                    retro_set_controller_port_device,

                    retro_reset,
                    retro_run,

                    retro_serialize_size,
                    retro_serialize,
                    retro_unserialize,

                    retro_cheat_reset,
                    retro_cheat_set,

                    retro_load_game,
                    retro_load_game_special,
                    retro_unload_game,

                    retro_get_region,
                    retro_get_memory_data,
                    retro_get_memory_size,
                },
                _marker: PhantomData,
            };
            let emup = Box::new(emu);
            // Store a pointer to the data
            EMULATOR = Box::leak(emup);
            // Forget the box so it doesn't drop
            let ctx = EmulatorContext {
                audio_sample: Vec::new(),
                buttons: [Buttons::new(), Buttons::new()],
                frame_ptr: ptr::null(),
                frame_pitch: 0,
                frame_width: 0,
                frame_height: 0,
                pixfmt: PixelFormat::ARGB1555,
                image_depth: 0,
                memory_map: Vec::new(),
                _marker: PhantomData,
            };
            // Ditto here for the context
            let ctxp = Box::new(ctx);
            CONTEXT = Box::leak(ctxp);
            let emu = &(*EMULATOR);
            // Set up callbacks
            (emu.core.retro_set_environment)(callback_environment);
            (emu.core.retro_set_video_refresh)(callback_video_refresh);
            (emu.core.retro_set_audio_sample)(callback_audio_sample);
            (emu.core.retro_set_audio_sample_batch)(callback_audio_sample_batch);
            (emu.core.retro_set_input_poll)(callback_input_poll);
            (emu.core.retro_set_input_state)(callback_input_state);
            // Load the game
            (emu.core.retro_init)();
            let mut sys_info = SystemInfo {
                library_name: ptr::null(),
                library_version: ptr::null(),
                valid_extensions: ptr::null(),
                need_fullpath: false,
                block_extract: false,
            };
            retro_get_system_info(&mut sys_info);
            let rom_cstr = &(*EMULATOR).rom_path;

            let mut rom_file = File::open(rom_path).unwrap();
            let mut buffer = Vec::new();
            rom_file.read_to_end(&mut buffer).unwrap();
            buffer.shrink_to_fit();
            let game_info = GameInfo {
                path: rom_cstr.as_ptr(),
                data: buffer.as_ptr() as *const c_void,
                size: buffer.len(),
                meta: ptr::null(),
            };
            (emu.core.retro_load_game)(&game_info);
            let mut av_info = SystemAvInfo {
                geometry: GameGeometry {
                    base_width: 0,
                    base_height: 0,
                    max_width: 0,
                    max_height: 0,
                    aspect_ratio: 0.0,
                },
                timing: SystemTiming {
                    fps: 0.0,
                    sample_rate: 0.0,
                },
            };
            (retro_get_system_av_info)(&mut av_info);
            Emulator {
                phantom: PhantomData,
            }
        }
    }
    pub fn get_library(&mut self) -> &Library {
        unsafe { &(*EMULATOR).core_lib }
    }
    pub fn get_symbol<'a, T>(&'a self, symbol: &[u8]) -> Option<Symbol<'a, T>> {
        let dll = unsafe { &(*EMULATOR).core_lib };
        let sym: Result<Symbol<T>, _> = unsafe { dll.get(symbol) };
        if sym.is_err() {
            return None;
        }
        Some(sym.unwrap())
    }
    pub fn run(&mut self, inputs: [Buttons; 2]) {
        unsafe {
            //clear audio buffers and whatever else
            (*CONTEXT).audio_sample.clear();
            //set inputs on CB
            (*CONTEXT).buttons = inputs;
            //run one step
            ((*EMULATOR).core.retro_run)()
        }
    }
    pub fn reset(&mut self) {
        unsafe {
            //clear audio buffers and whatever else
            (*CONTEXT).audio_sample.clear();
            //clear inputs on CB
            (*CONTEXT).buttons = [Buttons::new(), Buttons::new()];
            //clear fb
            (*CONTEXT).frame_ptr = ptr::null();
            ((*EMULATOR).core.retro_reset)()
        }
    }
    fn get_ram_size(&self, rtype: libc::c_uint) -> usize {
        unsafe { ((*EMULATOR).core.retro_get_memory_size)(rtype) as usize }
    }
    pub fn get_video_ram_size(&self) -> usize {
        self.get_ram_size(MEMORY_VIDEO_RAM)
    }
    pub fn get_system_ram_size(&self) -> usize {
        self.get_ram_size(MEMORY_SYSTEM_RAM)
    }
    pub fn get_save_ram_size(&self) -> usize {
        self.get_ram_size(MEMORY_SAVE_RAM)
    }
    pub fn video_ram_ref(&self) -> &[u8] {
        self.get_ram(MEMORY_VIDEO_RAM)
    }
    pub fn system_ram_ref(&self) -> &[u8] {
        self.get_ram(MEMORY_SYSTEM_RAM)
    }
    pub fn system_ram_mut(&mut self) -> &mut [u8] {
        self.get_ram_mut(MEMORY_SYSTEM_RAM)
    }
    pub fn save_ram(&self) -> &[u8] {
        self.get_ram(MEMORY_SAVE_RAM)
    }

    fn get_ram(&self, ramtype: libc::c_uint) -> &[u8] {
        let len = self.get_ram_size(ramtype);
        unsafe {
            let ptr: *const u8 = ((*EMULATOR).core.retro_get_memory_data)(ramtype).cast();
            std::slice::from_raw_parts(ptr, len)
        }
    }

    fn get_ram_mut(&mut self, ramtype: libc::c_uint) -> &mut [u8] {
        let len = self.get_ram_size(ramtype);
        unsafe {
            let ptr: *mut u8 = ((*EMULATOR).core.retro_get_memory_data)(ramtype).cast();
            std::slice::from_raw_parts_mut(ptr, len)
        }
    }

    pub fn memory_regions(&self) -> Vec<MemoryRegion> {
        let map = unsafe { &((*CONTEXT).memory_map) };
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
                    "".to_owned()
                } else {
                    unsafe { CStr::from_ptr(mdesc.addrspace) }
                        .to_string_lossy()
                        .into_owned()
                },
            })
            .collect()
    }
    pub fn memory_ref(&self, start: usize) -> Result<&[u8], RetroRsError> {
        for mr in self.memory_regions() {
            if mr.select != 0 && (start & mr.select) == 0 {
                continue;
            }
            if start >= mr.start && start < mr.start + mr.len {
                return self.memory_ref_mut(mr, start).map(|slice| &*slice);
            }
        }
        Err(RetroRsError::RAMCopyNotMappedIntoMemoryRegionError)
    }
    pub fn memory_ref_mut(
        &self,
        mr: MemoryRegion,
        start: usize,
    ) -> Result<&mut [u8], RetroRsError> {
        let maps = unsafe { &(*CONTEXT).memory_map };
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
    }

    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { (*CONTEXT).pixfmt }
    }
    pub fn framebuffer_size(&self) -> (usize, usize) {
        unsafe {
            (
                (*CONTEXT).frame_width as usize,
                (*CONTEXT).frame_height as usize,
            )
        }
    }
    pub fn framebuffer_pitch(&self) -> usize {
        unsafe { (*CONTEXT).frame_pitch }
    }
    fn peek_framebuffer<FBPeek, FBPeekRet>(&self, f: FBPeek) -> Result<FBPeekRet, RetroRsError>
    where
        FBPeek: FnOnce(&[u8]) -> FBPeekRet,
    {
        unsafe {
            if (*CONTEXT).frame_ptr.is_null() {
                Err(RetroRsError::NoFramebufferError)
            } else {
                let frame_slice = std::slice::from_raw_parts(
                    (*CONTEXT).frame_ptr as *const u8,
                    ((*CONTEXT).frame_height * ((*CONTEXT).frame_pitch as u32)) as usize,
                );
                Ok(f(frame_slice))
            }
        }
    }

    pub fn save(&self, bytes: &mut [u8]) {
        let size = self.save_size();
        assert!(bytes.len() >= size);
        unsafe { ((*EMULATOR).core.retro_serialize)(bytes.as_mut_ptr() as *mut c_void, size) }
    }
    pub fn load(&mut self, bytes: &[u8]) -> bool {
        let size = self.save_size();
        assert!(bytes.len() >= size);
        unsafe { ((*EMULATOR).core.retro_unserialize)(bytes.as_ptr() as *const c_void, size) }
    }
    pub fn save_size(&self) -> usize {
        unsafe { ((*EMULATOR).core.retro_serialize_size)() }
    }
    pub fn clear_cheats(&mut self) {
        unsafe { ((*EMULATOR).core.retro_cheat_reset)() }
    }
    pub fn set_cheat(&mut self, index: usize, enabled: bool, code: &str) {
        unsafe {
            // FIXME: Creates a memory leak since the libretro api won't let me from_raw() it back and drop it.  I don't know if libretro guarantees anything about ownership of this str to cores.
            ((*EMULATOR).core.retro_cheat_set)(
                index as u32,
                enabled,
                CString::new(code).unwrap().into_raw(),
            )
        }
    }
    pub fn get_pixel(&self, x: usize, y: usize) -> Result<(u8, u8, u8), RetroRsError> {
        let (w, _h) = self.framebuffer_size();
        self.peek_framebuffer(move |fb| match self.pixel_format() {
            PixelFormat::ARGB1555 => {
                let start = y * w + x;
                let gb = fb[start * 2];
                let arg = fb[start * 2 + 1];
                let (red, green, blue) = argb555to888(gb, arg);
                (red, green, blue)
            }
            PixelFormat::ARGB8888 => {
                let off = (y * w + x) * 4;
                (fb[off + 1], fb[off + 2], fb[off + 3])
            }
            PixelFormat::RGB565 => {
                let start = y * w + x;
                let gb = fb[start * 2];
                let rg = fb[start * 2 + 1];
                let (red, green, blue) = rgb565to888(gb, rg);
                (red, green, blue)
            }
        })
    }
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
                PixelFormat::ARGB1555 => {
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
                PixelFormat::ARGB8888 => {
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
                PixelFormat::RGB565 => {
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
            };
            assert_eq!(y, h);
            assert_eq!(x, 0);
        })
    }
    pub fn copy_framebuffer_rgb888(&self, slice: &mut [u8]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| {
            match fmt {
                PixelFormat::ARGB1555 => {
                    for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(3)) {
                        let gb = components[0];
                        let arg = components[1];
                        let (red, green, blue) = argb555to888(gb, arg);
                        dst[0] = red;
                        dst[1] = green;
                        dst[2] = blue;
                    }
                }
                PixelFormat::ARGB8888 => {
                    for (components, dst) in fb.chunks_exact(4).zip(slice.chunks_exact_mut(3)) {
                        let r = components[1];
                        let g = components[2];
                        let b = components[3];
                        dst[0] = r;
                        dst[1] = g;
                        dst[2] = b;
                    }
                }
                PixelFormat::RGB565 => {
                    for (components, dst) in fb.chunks_exact(2).zip(slice.chunks_exact_mut(3)) {
                        let gb = components[0];
                        let rg = components[1];
                        let (red, green, blue) = rgb565to888(gb, rg);
                        dst[0] = red;
                        dst[1] = green;
                        dst[2] = blue;
                    }
                }
            };
        })
    }
    pub fn copy_framebuffer_rgb332(&self, slice: &mut [u8]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| {
            match fmt {
                PixelFormat::ARGB1555 => {
                    for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                        let gb = components[0];
                        let arg = components[1];
                        let (red, green, blue) = argb555to888(gb, arg);
                        *dst = rgb888_to_rgb332(red, green, blue);
                    }
                }
                PixelFormat::ARGB8888 => {
                    for (components, dst) in fb.chunks_exact(4).zip(slice.iter_mut()) {
                        let r = components[1];
                        let g = components[2];
                        let b = components[3];
                        *dst = rgb888_to_rgb332(r, g, b);
                    }
                }
                PixelFormat::RGB565 => {
                    for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                        let gb = components[0];
                        let rg = components[1];
                        let (red, green, blue) = rgb565to888(gb, rg);
                        *dst = rgb888_to_rgb332(red, green, blue);
                    }
                }
            };
        })
    }
    pub fn copy_framebuffer_argb32(&self, slice: &mut [u32]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| {
            match fmt {
                PixelFormat::ARGB1555 => {
                    for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                        let gb = components[0];
                        let arg = components[1];
                        let (red, green, blue) = argb555to888(gb, arg);
                        *dst = 0xFF00_0000
                            | (u32::from(red) << 16)
                            | (u32::from(green) << 8)
                            | u32::from(blue);
                    }
                }
                PixelFormat::ARGB8888 => {
                    for (components, dst) in fb.chunks_exact(4).zip(slice.iter_mut()) {
                        *dst = (u32::from(components[0]) << 24)
                            | (u32::from(components[1]) << 16)
                            | (u32::from(components[2]) << 8)
                            | u32::from(components[3]);
                    }
                }
                PixelFormat::RGB565 => {
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
            };
        })
    }
    pub fn copy_framebuffer_rgba32(&self, slice: &mut [u32]) -> Result<(), RetroRsError> {
        let fmt = self.pixel_format();
        self.peek_framebuffer(move |fb| {
            match fmt {
                PixelFormat::ARGB1555 => {
                    for (components, dst) in fb.chunks_exact(2).zip(slice.iter_mut()) {
                        let gb = components[0];
                        let arg = components[1];
                        let (red, green, blue) = argb555to888(gb, arg);
                        *dst = (u32::from(red) << 24)
                            | (u32::from(green) << 16)
                            | (u32::from(blue) << 8)
                            | (u32::from(0xFF * (arg >> 7)));
                    }
                }
                PixelFormat::ARGB8888 => {
                    for (components, dst) in fb.chunks_exact(4).zip(slice.iter_mut()) {
                        *dst = (u32::from(components[1]) << 24)
                            | (u32::from(components[2]) << 16)
                            | (u32::from(components[3]) << 8)
                            | u32::from(components[0]);
                    }
                }
                PixelFormat::RGB565 => {
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
            };
        })
    }
}

unsafe extern "C" fn callback_environment(cmd: u32, data: *mut c_void) -> bool {
    let result = panic::catch_unwind(|| {
        match cmd {
            ENVIRONMENT_SET_CONTROLLER_INFO => true,
            ENVIRONMENT_SET_PIXEL_FORMAT => {
                let pixfmti = *(data as *const u32);
                let pixfmt = PixelFormat::from_uint(pixfmti);
                if pixfmt.is_none() {
                    return false;
                }
                let pixfmt = pixfmt.unwrap();
                (*CONTEXT).image_depth = match pixfmt {
                    PixelFormat::ARGB1555 => 15,
                    PixelFormat::ARGB8888 => 32,
                    PixelFormat::RGB565 => 16,
                };
                (*CONTEXT).pixfmt = pixfmt;
                true
            }
            ENVIRONMENT_GET_SYSTEM_DIRECTORY => {
                *(data as *mut *const c_char) = (*EMULATOR).core_path.as_ptr();
                true
            }
            ENVIRONMENT_GET_CAN_DUPE => {
                *(data as *mut bool) = true;
                true
            }
            ENVIRONMENT_SET_MEMORY_MAPS => {
                let map = data as *const MemoryMap;
                let desc_slice =
                    std::slice::from_raw_parts((*map).descriptors, (*map).num_descriptors as usize);
                // Don't know who owns map or how long it will last
                (*CONTEXT).memory_map = Vec::new();
                // So we had better copy it
                (*CONTEXT).memory_map.extend_from_slice(desc_slice);
                // (Implicitly we also want to drop the old one, which we did by reassigning)
                true
            }
            _ => false,
        }
    });
    result.unwrap_or(false)
}

extern "C" fn callback_video_refresh(data: *const c_void, width: u32, height: u32, pitch: usize) {
    // Can't panic
    unsafe {
        // context's framebuffer just points to the given data.  Seems to work OK for gym-retro.
        if !data.is_null() {
            (*CONTEXT).frame_ptr = data;
            (*CONTEXT).frame_pitch = pitch;
            (*CONTEXT).frame_width = width;
            (*CONTEXT).frame_height = height;
        }
    }
}
extern "C" fn callback_audio_sample(left: i16, right: i16) {
    // Can't panic
    unsafe {
        let sample_buf = &mut (*CONTEXT).audio_sample;
        sample_buf.push(left);
        sample_buf.push(right);
    }
}
extern "C" fn callback_audio_sample_batch(data: *const i16, frames: usize) -> usize {
    // Can't panic
    unsafe {
        let sample_buf = &mut (*CONTEXT).audio_sample;
        let slice = std::slice::from_raw_parts(data, frames * 2);
        sample_buf.clear();
        sample_buf.extend_from_slice(slice);
        frames
    }
}

extern "C" fn callback_input_poll() {}

extern "C" fn callback_input_state(port: u32, device: u32, index: u32, id: u32) -> i16 {
    // Can't panic
    if port > 1 || device != 1 || index != 0 {
        // Unsupported port/device/index
        println!("Unsupported port/device/index");
        return 0;
    }
    let id = id;
    let port = port as usize;
    if id > 16 {
        println!("Unexpected button id {}", id);
        return 0;
    }
    unsafe {
        if (*CONTEXT).buttons[port].get(id) {
            1
        } else {
            0
        }
    }
}

impl Drop for Emulator {
    fn drop(&mut self) {
        unsafe {
            ((*EMULATOR).core.retro_unload_game)();
            ((*EMULATOR).core.retro_deinit)();
        }
        //TODO drop memory maps etc
        unsafe {
            // "remember" context and emulator we forgot before
            let _ctx = Box::from_raw(CONTEXT);
            let _emu = Box::from_raw(EMULATOR);
            CONTEXT = ptr::null_mut();
            EMULATOR = ptr::null_mut();
        }
        // let them drop naturally
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

    const PPU_BIT: usize = 1 << 31;

    fn get_byte(emu: &Emulator, addr: usize) -> u8 {
        emu.memory_ref(addr).expect("Couldn't read RAM!")[0]
    }

    #[cfg(feature = "use_image")]
    #[test]
    fn it_works() {
        // TODO change to a public domain rom or maybe 2048 core or something
        let mut emu = Emulator::create(
            Path::new("../mechlearn/mappy/cores/fceumm_libretro"),
            Path::new("roms/mario.nes"),
        );
        emu.run([Buttons::new(), Buttons::new()]);
        emu.reset();
        for i in 0..250 {
            emu.run([
                Buttons::new()
                    .start(i > 80 && i < 100)
                    .right(i >= 100)
                    .a((i >= 100 && i <= 150) || (i >= 180)),
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
                    .a((i >= 100 && i <= 150) || (i >= 180)),
                Buttons::new(),
            ]);
        }

        //emu will drop naturally
    }
}
