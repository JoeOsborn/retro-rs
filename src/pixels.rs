#[inline(always)]
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

#[inline(always)]
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
#[inline(always)]
pub fn rgb332_to_rgb888(col: u8) -> (u8, u8, u8) {
    let col = col as u32;
    let r = (((col & 0b1110_0000) >> 5) * 255) / 8;
    let g = (((col & 0b0001_1100) >> 2) * 255) / 8;
    let b = ((col & 0b0000_0011) * 255) / 4;
    assert!(r <= 255);
    assert!(g <= 255);
    assert!(b <= 255);
    (r as u8, g as u8, b as u8)
}
#[inline(always)]
pub fn rgb888_to_rgb332(r: u8, g: u8, b: u8) -> u8 {
    let r = ((r as u32 * 8) / 256) as u8;
    let g = ((g as u32 * 8) / 256) as u8;
    let b = ((b as u32 * 4) / 256) as u8;
    assert!(r <= 7);
    assert!(g <= 7);
    assert!(b <= 3);
    (r << 5) + (g << 2) + b
}
