#[inline]
#[must_use]
pub fn argb555to888(lo: u8, hi: u8) -> (u8, u8, u8) {
    let r = (hi & 0b0111_1100) >> 2;
    let g = ((hi & 0b0000_0011) << 3) + ((lo & 0b1110_0000) >> 5);
    let b = lo & 0b0001_1111;
    // Use high bits for empty low bits
    let r = (r << 3) | (r >> 2);
    let g = (g << 3) | (g >> 2);
    let b = (b << 3) | (b >> 2);
    (r, g, b)
}

#[inline]
#[must_use]
pub fn rgb565to888(lo: u8, hi: u8) -> (u8, u8, u8) {
    let r = (hi & 0b1111_1000) >> 3;
    let g = ((hi & 0b0000_0111) << 3) + ((lo & 0b1110_0000) >> 5);
    let b = lo & 0b0001_1111;
    // Use high bits for empty low bits
    let r = (r << 3) | (r >> 2);
    let g = (g << 2) | (g >> 3);
    let b = (b << 3) | (b >> 2);
    (r, g, b)
}
#[inline]
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn rgb332_to_rgb888(col: u8) -> (u8, u8, u8) {
    let col = u32::from(col);
    let r = (((col & 0b1110_0000) >> 5) * 255) / 8;
    let g = (((col & 0b0001_1100) >> 2) * 255) / 8;
    let b = ((col & 0b0000_0011) * 255) / 4;
    debug_assert!(r <= 255);
    debug_assert!(g <= 255);
    debug_assert!(b <= 255);
    (r as u8, g as u8, b as u8)
}
#[inline]
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn rgb888_to_rgb332(r: u8, g: u8, b: u8) -> u8 {
    let r = ((u32::from(r) * 8) / 256) as u8;
    let g = ((u32::from(g) * 8) / 256) as u8;
    let b = ((u32::from(b) * 4) / 256) as u8;
    debug_assert!(r <= 7);
    debug_assert!(g <= 7);
    debug_assert!(b <= 3);
    (r << 5) + (g << 2) + b
}
