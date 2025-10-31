use rust_libretro_sys::{
    RETRO_DEVICE_ID_JOYPAD_A, RETRO_DEVICE_ID_JOYPAD_B, RETRO_DEVICE_ID_JOYPAD_DOWN,
    RETRO_DEVICE_ID_JOYPAD_L, RETRO_DEVICE_ID_JOYPAD_L2, RETRO_DEVICE_ID_JOYPAD_L3,
    RETRO_DEVICE_ID_JOYPAD_LEFT, RETRO_DEVICE_ID_JOYPAD_R, RETRO_DEVICE_ID_JOYPAD_R2,
    RETRO_DEVICE_ID_JOYPAD_R3, RETRO_DEVICE_ID_JOYPAD_RIGHT, RETRO_DEVICE_ID_JOYPAD_SELECT,
    RETRO_DEVICE_ID_JOYPAD_START, RETRO_DEVICE_ID_JOYPAD_UP, RETRO_DEVICE_ID_JOYPAD_X,
    RETRO_DEVICE_ID_JOYPAD_Y,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Buttons(i16);
impl From<Buttons> for i16 {
    fn from(value: Buttons) -> Self {
        value.0
    }
}
impl Buttons {
    #[must_use]
    pub fn new() -> Self {
        Buttons::default()
    }
    /// # Panics
    /// If id is too large for the libretro controller API (0..16)
    #[must_use]
    pub fn get(self, id: u32) -> bool {
        assert!(id < 16);
        (self.0 & (1 << id)) != 0
    }
    /// # Panics: If id is too large for the libretro controller API (0..16)
    fn mask_inout(self, b: bool, id: u32) -> Self {
        assert!(id < 16);
        if b {
            Buttons(self.0 | (1 << id))
        } else {
            Buttons(self.0 & !(1 << id))
        }
    }
    #[must_use]
    pub fn up(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_UP)
    }
    #[must_use]
    pub fn down(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_DOWN)
    }
    #[must_use]
    pub fn left(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_LEFT)
    }
    #[must_use]
    pub fn right(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_RIGHT)
    }

    #[must_use]
    pub fn select(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_SELECT)
    }
    #[must_use]
    pub fn start(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_START)
    }
    #[must_use]
    pub fn a(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_A)
    }
    #[must_use]
    pub fn b(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_B)
    }
    #[must_use]
    pub fn y(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_Y)
    }
    #[must_use]
    pub fn x(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_X)
    }
    #[must_use]
    pub fn l1(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_L)
    }
    #[must_use]
    pub fn r1(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_R)
    }
    #[must_use]
    pub fn l2(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_L2)
    }
    #[must_use]
    pub fn r2(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_R2)
    }
    #[must_use]
    pub fn l3(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_L3)
    }
    #[must_use]
    pub fn r3(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_R3)
    }

    #[must_use]
    pub fn get_up(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_UP)
    }
    #[must_use]
    pub fn get_down(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_DOWN)
    }
    #[must_use]
    pub fn get_left(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_LEFT)
    }
    #[must_use]
    pub fn get_right(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_RIGHT)
    }

    #[must_use]
    pub fn get_select(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_SELECT)
    }
    #[must_use]
    pub fn get_start(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_START)
    }
    #[must_use]
    pub fn get_a(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_A)
    }
    #[must_use]
    pub fn get_b(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_B)
    }
    #[must_use]
    pub fn get_y(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_Y)
    }
    #[must_use]
    pub fn get_x(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_X)
    }
    #[must_use]
    pub fn get_l1(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_L)
    }
    #[must_use]
    pub fn get_r1(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_R)
    }
    #[must_use]
    pub fn get_l2(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_L2)
    }
    #[must_use]
    pub fn get_r2(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_R2)
    }
    #[must_use]
    pub fn get_l3(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_L3)
    }
    #[must_use]
    pub fn get_r3(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_R3)
    }
}
