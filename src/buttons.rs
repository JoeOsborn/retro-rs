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

impl Buttons {
    pub fn new() -> Self {
        Buttons::default()
    }
    pub fn get(self, id: u32) -> bool {
        assert!(id < 16);
        (self.0 & (1 << id)) != 0
    }
    fn mask_inout(self, b: bool, id: u32) -> Self {
        assert!(id < 16);
        if b {
            Buttons(self.0 | (1 << id))
        } else {
            Buttons(self.0 & !(1 << id))
        }
    }
    pub fn up(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_UP)
    }
    pub fn down(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_DOWN)
    }
    pub fn left(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_LEFT)
    }
    pub fn right(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_RIGHT)
    }

    pub fn select(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_SELECT)
    }
    pub fn start(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_START)
    }
    pub fn a(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_A)
    }
    pub fn b(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_B)
    }
    pub fn y(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_Y)
    }
    pub fn x(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_X)
    }
    pub fn l1(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_L)
    }
    pub fn r1(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_R)
    }
    pub fn l2(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_L2)
    }
    pub fn r2(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_R2)
    }
    pub fn l3(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_L3)
    }
    pub fn r3(self, b: bool) -> Self {
        self.mask_inout(b, RETRO_DEVICE_ID_JOYPAD_R3)
    }

    pub fn get_up(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_UP)
    }
    pub fn get_down(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_DOWN)
    }
    pub fn get_left(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_LEFT)
    }
    pub fn get_right(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_RIGHT)
    }

    pub fn get_select(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_SELECT)
    }
    pub fn get_start(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_START)
    }
    pub fn get_a(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_A)
    }
    pub fn get_b(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_B)
    }
    pub fn get_y(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_Y)
    }
    pub fn get_x(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_X)
    }
    pub fn get_l1(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_L)
    }
    pub fn get_r1(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_R)
    }
    pub fn get_l2(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_L2)
    }
    pub fn get_r2(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_R2)
    }
    pub fn get_l3(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_L3)
    }
    pub fn get_r3(self) -> bool {
        self.get(RETRO_DEVICE_ID_JOYPAD_R3)
    }
}
