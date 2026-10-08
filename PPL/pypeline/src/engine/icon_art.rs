// The game icon, drawn in code as 64x64 pixel art after the PypeLine key
// art: a rounded gray frame around a blue sky with clouds and blueprint
// lines, a code file over the PYPELINE title, an orange-domed machine with
// the Python logo in a gear, conveyor loops carrying ore on both sides, and
// a little train on the rails.
//
// Plain std only, and only `//` comments: build.rs includes this file with
// `include!`. It makes the window and .exe icons from this drawing, or from
// assets/icon/pypeline.png when that file exists (see assets/icon/README.md).

/// Width and height of the icon, in pixels.
pub const ICON_SIZE: usize = 64;

type Rgba = [u8; 4];

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    [r, g, b, 255]
}

const CLEAR: Rgba = [0, 0, 0, 0];
const FRAME_DARK: Rgba = rgb(56, 56, 72);
const FRAME: Rgba = rgb(152, 152, 168);
const FRAME_HI: Rgba = rgb(208, 208, 224);
const SKY_TOP: Rgba = rgb(88, 168, 240);
const SKY_MID: Rgba = rgb(120, 192, 248);
const SKY_LOW: Rgba = rgb(168, 224, 248);
const CLOUD: Rgba = rgb(248, 248, 248);
const CLOUD_SHADE: Rgba = rgb(200, 216, 240);
const INK: Rgba = rgb(32, 32, 48);
const PAPER: Rgba = rgb(240, 240, 232);
const PAPER_SHADE: Rgba = rgb(184, 192, 208);
const GOLD: Rgba = rgb(248, 208, 64);
const ORANGE: Rgba = rgb(240, 128, 40);
const ORANGE_HI: Rgba = rgb(248, 184, 96);
const ORANGE_DARK: Rgba = rgb(176, 72, 24);
const METAL: Rgba = rgb(120, 128, 144);
const METAL_DARK: Rgba = rgb(72, 80, 96);
const METAL_HI: Rgba = rgb(184, 192, 208);
const PY_BLUE: Rgba = rgb(56, 112, 176);
const PY_YELLOW: Rgba = rgb(248, 208, 64);
const BELT: Rgba = rgb(64, 64, 80);
const ORE: Rgba = rgb(136, 104, 88);
const ORE_HI: Rgba = rgb(200, 160, 120);
const TIE: Rgba = rgb(120, 80, 48);
const TRAIN: Rgba = rgb(200, 56, 48);
const GLASS: Rgba = rgb(168, 224, 248);
const CRATE: Rgba = rgb(192, 136, 72);
const CRATE_DARK: Rgba = rgb(128, 80, 40);

struct Canvas {
    px: Vec<Rgba>,
}

impl Canvas {
    fn new() -> Self {
        Canvas {
            px: vec![CLEAR; ICON_SIZE * ICON_SIZE],
        }
    }

    fn set(&mut self, x: i32, y: i32, c: Rgba) {
        if (0..ICON_SIZE as i32).contains(&x) && (0..ICON_SIZE as i32).contains(&y) {
            self.px[y as usize * ICON_SIZE + x as usize] = c;
        }
    }

    fn get(&self, x: i32, y: i32) -> Rgba {
        self.px[y as usize * ICON_SIZE + x as usize]
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// Every pixel whose center is within `r` of (cx, cy).
    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Rgba) {
        for y in 0..ICON_SIZE as i32 {
            for x in 0..ICON_SIZE as i32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.set(x, y, c);
                }
            }
        }
    }
}

/// Is (x, y) inside the icon's rounded square, shrunk by `inset`?
fn inside_frame(x: i32, y: i32, inset: i32) -> bool {
    let radius = 10 - inset;
    let (lo, hi) = (inset, ICON_SIZE as i32 - 1 - inset);
    if x < lo || x > hi || y < lo || y > hi {
        return false;
    }
    let cx = x.clamp(lo + radius, hi - radius);
    let cy = y.clamp(lo + radius, hi - radius);
    let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
    dx * dx + dy * dy <= (radius as f32 + 0.3).powi(2)
}

fn frame_and_sky(c: &mut Canvas) {
    for y in 0..ICON_SIZE as i32 {
        for x in 0..ICON_SIZE as i32 {
            let color = if !inside_frame(x, y, 0) {
                CLEAR
            } else if !inside_frame(x, y, 1) {
                FRAME_DARK
            } else if !inside_frame(x, y, 3) {
                // Lit from the top left.
                if x + y < ICON_SIZE as i32 - 4 {
                    FRAME_HI
                } else {
                    FRAME
                }
            } else if !inside_frame(x, y, 4) {
                FRAME_DARK
            } else {
                // Three bands of sky, dithered where they meet.
                let band = |y: i32| match y {
                    ..20 => SKY_TOP,
                    20..34 => SKY_MID,
                    _ => SKY_LOW,
                };
                if (y == 20 || y == 34) && (x % 2 == 0) {
                    band(y - 1)
                } else {
                    band(y)
                }
            };
            c.set(x, y, color);
        }
    }
}

fn cloud(c: &mut Canvas, cx: f32, cy: f32) {
    c.disc(cx, cy + 0.5, 3.0, CLOUD_SHADE);
    c.disc(cx - 3.5, cy + 1.5, 2.2, CLOUD_SHADE);
    c.disc(cx + 3.5, cy + 1.5, 2.2, CLOUD_SHADE);
    c.disc(cx, cy, 3.0, CLOUD);
    c.disc(cx - 3.5, cy + 1.0, 2.2, CLOUD);
    c.disc(cx + 3.5, cy + 1.0, 2.2, CLOUD);
}

/// The angle of (x, y) around (cx, cy), in degrees, counterclockwise from
/// pointing right, 0..360.
fn angle(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    let a = (-(y - cy)).atan2(x - cx).to_degrees();
    if a < 0.0 { a + 360.0 } else { a }
}

/// Faint blueprint lines in the sky: little flowcharts.
fn blueprint(c: &mut Canvas) {
    let line = rgb(112, 176, 232);
    for (x, y, w, h) in [(7, 24, 6, 4), (7, 32, 6, 4), (50, 22, 7, 4), (51, 31, 5, 5)] {
        for xx in x..x + w {
            c.set(xx, y, line);
            c.set(xx, y + h - 1, line);
        }
        for yy in y..y + h {
            c.set(x, yy, line);
            c.set(x + w - 1, yy, line);
        }
    }
    for y in 28..32 {
        c.set(10, y, line);
    }
    for y in 26..31 {
        c.set(53, y, line);
    }
}

/// A code file: a page with a folded corner and lines of code on it.
fn code_file(c: &mut Canvas, x: i32, y: i32) {
    c.rect(x, y, 9, 9, INK);
    c.rect(x + 1, y + 1, 7, 7, PAPER);
    // The folded corner.
    for (dx, dy) in [(8, 0), (7, 0), (8, 1)] {
        c.set(x + dx, y + dy, CLEAR);
    }
    c.set(x + 6, y, INK);
    c.set(x + 7, y + 1, INK);
    c.set(x + 8, y + 2, INK);
    c.set(x + 6, y + 1, PAPER_SHADE);
    c.set(x + 6, y + 2, PAPER_SHADE);
    c.set(x + 7, y + 2, PAPER_SHADE);
    for (row, len, color) in [(3, 4, PY_BLUE), (4, 5, METAL), (6, 3, TRAIN)] {
        c.rect(x + 2, y + row, len, 1, color);
    }
}

/// 5-row letters of varying width for the title.
fn letter(ch: char) -> &'static [&'static str] {
    match ch {
        'P' => &["####.", "#...#", "####.", "#....", "#...."],
        'Y' => &["#...#", ".#.#.", "..#..", "..#..", "..#.."],
        'E' => &["####", "#...", "###.", "#...", "####"],
        'L' => &["#...", "#...", "#...", "#...", "####"],
        'I' => &["###", ".#.", ".#.", ".#.", "###"],
        'N' => &["#...#", "##..#", "#.#.#", "#..##", "#...#"],
        _ => &["..", "..", "..", "..", ".."],
    }
}

fn title_width(text: &str) -> i32 {
    text.chars()
        .map(|ch| letter(ch)[0].len() as i32 + 1)
        .sum::<i32>()
        - 1
}

/// Bold black letters, two pixels tall for each row of the font.
fn title(c: &mut Canvas, x: i32, y: i32, text: &str) {
    let mut at = x;
    for ch in text.chars() {
        let rows = letter(ch);
        for (dy, row) in rows.iter().enumerate() {
            for (dx, px) in row.chars().enumerate() {
                if px == '#' {
                    c.rect(at + dx as i32, y + dy as i32 * 2, 1, 2, INK);
                }
            }
        }
        at += rows[0].len() as i32 + 1;
    }
}

/// A gear with the Python logo in its middle.
fn python_gear(c: &mut Canvas, gx: f32, gy: f32) {
    for y in 0..ICON_SIZE as i32 {
        for x in 0..ICON_SIZE as i32 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = ((fx - gx).powi(2) + (fy - gy).powi(2)).sqrt();
            let tooth = (angle(fx, fy, gx, gy) / 30.0 + 0.25).floor() as i32 % 2 == 0;
            if d <= 6.0 || (d <= 7.4 && tooth) {
                let color = if d > 6.6 {
                    METAL_DARK
                } else if fx + fy < gx + gy {
                    METAL_HI
                } else {
                    METAL
                };
                c.set(x, y, color);
            }
        }
    }
    // The Python logo: blue snake at the top left, yellow at the bottom right.
    c.disc(gx, gy, 5.0, INK);
    let logo = [
        "..BBBB..", "..BoBB..", "BBBBBB.Y", "BB....YY", "BB....YY", "B.YYYYYY", "..YYoY..",
        "..YYYY..",
    ];
    for (dy, row) in logo.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            let color = match ch {
                'B' => PY_BLUE,
                'Y' => PY_YELLOW,
                'o' => PAPER,
                _ => continue,
            };
            c.set(gx as i32 - 4 + dx as i32, gy as i32 - 4 + dy as i32, color);
        }
    }
}

fn machine(c: &mut Canvas) {
    let (cx, cy) = (32.0, 31.0);
    // Dome: an orange half circle with a highlight, on a gray rim.
    for y in 0..31 {
        for x in 0..ICON_SIZE as i32 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= 8.0 {
                let color = if d > 7.0 {
                    ORANGE_DARK
                } else if dx + dy < -5.5 && d > 3.5 {
                    ORANGE_HI
                } else {
                    ORANGE
                };
                c.set(x, y, color);
            }
        }
    }
    c.set(28, 26, CLOUD);
    c.rect(22, 30, 20, 3, INK);
    c.rect(23, 31, 18, 1, METAL);
    // Body: orange casing around a green circuit panel.
    c.rect(20, 33, 24, 15, INK);
    c.rect(21, 33, 22, 14, ORANGE_DARK);
    c.rect(21, 33, 22, 1, ORANGE);
    c.rect(21, 33, 1, 14, ORANGE);
    c.rect(23, 35, 18, 11, rgb(48, 96, 64));
    for (x, y) in [(24, 36), (39, 36), (24, 44), (39, 44)] {
        c.set(x, y, rgb(96, 200, 120));
    }
    python_gear(c, 32.0, 40.0);
}

/// One piece of ore on a belt.
fn ore(c: &mut Canvas, x: i32, y: i32) {
    c.rect(x, y, 2, 2, ORE);
    c.set(x, y, ORE_HI);
}

/// A racetrack-shaped conveyor loop, `w` by `h`, 3 pixels wide.
fn conveyor_loop(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    let inside = |px: i32, py: i32, inset: i32| -> bool {
        let r = (h / 2 - inset) as f32;
        let (lo, hi) = (x + h / 2, x + w - 1 - h / 2);
        let cy = y as f32 + (h - 1) as f32 / 2.0;
        let cx = px.clamp(lo, hi) as f32;
        let (dx, dy) = (px as f32 - cx, py as f32 - cy);
        dx * dx + dy * dy <= (r + 0.3) * (r + 0.3)
    };
    for py in y..y + h {
        for px in x..x + w {
            if !inside(px, py, 0) {
                continue;
            }
            let color = if !inside(px, py, 1) || inside(px, py, 4) && !inside(px, py, 5) {
                INK
            } else if inside(px, py, 4) {
                continue;
            } else if (px + py) % 4 == 0 && inside(px, py, 2) && !inside(px, py, 3) {
                GOLD
            } else {
                BELT
            };
            c.set(px, py, color);
        }
    }
}

fn conveyors(c: &mut Canvas) {
    conveyor_loop(c, 4, 29, 18, 15);
    conveyor_loop(c, 42, 29, 18, 15);
    // Short belts into the machine.
    c.rect(19, 34, 2, 3, BELT);
    c.rect(43, 34, 2, 3, BELT);
    for (x, y) in [(8, 30), (14, 41), (5, 36), (50, 30), (56, 39), (45, 41)] {
        ore(c, x, y);
    }
    // A little code file riding along each loop.
    for (x, y) in [(12, 30), (58, 35)] {
        c.rect(x, y, 2, 2, PAPER);
        c.set(x, y + 1, PY_BLUE);
    }
}

fn rails(c: &mut Canvas) {
    for x in 5..59 {
        if x % 3 != 2 {
            c.set(x, 56, TIE);
            c.set(x, 57, TIE);
        }
        c.set(x, 55, METAL_HI);
        c.set(x, 58, METAL_DARK);
    }
    // A ladder down from the left loop to the rails.
    for y in 44..55 {
        c.set(8, y, METAL_DARK);
        c.set(11, y, METAL_DARK);
        if y % 2 == 0 {
            c.rect(9, y, 2, 1, METAL);
        }
    }
}

fn wheel(c: &mut Canvas, x: f32) {
    c.disc(x, 54.0, 1.5, INK);
}

/// A boxy car with a dark outline.
fn car(c: &mut Canvas, x: i32, w: i32, top: i32, body: Rgba) {
    c.rect(x, top, w, 53 - top, INK);
    c.rect(x + 1, top + 1, w - 2, 51 - top, body);
    wheel(c, x as f32 + 2.0);
    wheel(c, (x + w) as f32 - 2.0);
}

/// A little train: an ore cart, an orange engine, two computers and a crate.
fn train(c: &mut Canvas) {
    car(c, 21, 8, 48, METAL);
    c.rect(23, 46, 4, 3, PAPER);
    c.set(24, 47, PY_BLUE);
    car(c, 30, 8, 47, ORANGE);
    c.rect(35, 45, 2, 3, INK);
    c.rect(31, 49, 3, 2, GLASS);
    car(c, 39, 8, 48, CRATE_DARK);
    c.rect(41, 45, 4, 4, INK);
    c.rect(42, 46, 2, 2, rgb(96, 200, 120));
    car(c, 48, 10, 48, CRATE_DARK);
    c.rect(50, 43, 7, 6, INK);
    c.rect(51, 44, 5, 4, CRATE);
    c.rect(51, 46, 5, 1, CRATE_DARK);
}

/// The icon at 64x64, as RGBA bytes row by row from the top.
pub fn icon_rgba() -> Vec<u8> {
    let mut c = Canvas::new();
    frame_and_sky(&mut c);
    blueprint(&mut c);
    cloud(&mut c, 12.0, 8.0);
    cloud(&mut c, 51.0, 7.0);
    code_file(&mut c, 28, 4);
    let width = title_width("PYPELINE");
    title(&mut c, (ICON_SIZE as i32 - width) / 2, 16, "PYPELINE");
    conveyors(&mut c);
    machine(&mut c);
    rails(&mut c);
    train(&mut c);
    // Nothing outside the rounded frame.
    for y in 0..ICON_SIZE as i32 {
        for x in 0..ICON_SIZE as i32 {
            if !inside_frame(x, y, 0) && c.get(x, y) != CLEAR {
                c.set(x, y, CLEAR);
            }
        }
    }
    c.px.concat()
}

/// The icon at `size` pixels square: whole-number scaled up, or averaged
/// down for sizes under 64 (which must divide 64).
pub fn icon_rgba_sized(size: usize) -> Vec<u8> {
    let src = icon_rgba();
    let mut out = vec![0u8; size * size * 4];
    if size >= ICON_SIZE {
        for y in 0..size {
            for x in 0..size {
                let (sx, sy) = (x * ICON_SIZE / size, y * ICON_SIZE / size);
                let s = (sy * ICON_SIZE + sx) * 4;
                out[(y * size + x) * 4..][..4].copy_from_slice(&src[s..s + 4]);
            }
        }
        return out;
    }
    shrink(&src, ICON_SIZE, ICON_SIZE / size)
}

/// Shrink a square RGBA image `size` pixels wide by averaging each `block`
/// by `block` square. Colors are weighted by alpha, so see-through corners
/// do not darken the edges.
pub fn shrink(rgba: &[u8], size: usize, block: usize) -> Vec<u8> {
    let out_size = size / block;
    let mut out = vec![0u8; out_size * out_size * 4];
    for y in 0..out_size {
        for x in 0..out_size {
            let mut sum = [0u32; 4];
            for by in 0..block {
                for bx in 0..block {
                    let s = ((y * block + by) * size + x * block + bx) * 4;
                    let a = u32::from(rgba[s + 3]);
                    for (i, total) in sum.iter_mut().take(3).enumerate() {
                        *total += u32::from(rgba[s + i]) * a;
                    }
                    sum[3] += a;
                }
            }
            let o = (y * out_size + x) * 4;
            for i in 0..3 {
                out[o + i] = sum[i].checked_div(sum[3]).unwrap_or(0) as u8;
            }
            out[o + 3] = (sum[3] / (block * block) as u32) as u8;
        }
    }
    out
}

/// A Windows .ico holding the drawn icon at 256, 64, 32 and 16 pixels.
pub fn icon_ico() -> Vec<u8> {
    let images: Vec<(usize, Vec<u8>)> = [256, 64, 32, 16]
        .into_iter()
        .map(|s| (s, icon_rgba_sized(s)))
        .collect();
    ico(&images)
}

/// A Windows .ico from square RGBA images, given as (size, pixels).
pub fn ico(images: &[(usize, Vec<u8>)]) -> Vec<u8> {
    let bitmaps: Vec<Vec<u8>> = images
        .iter()
        .map(|(size, rgba)| ico_bitmap(*size, rgba))
        .collect();
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for ((size, _), bitmap) in images.iter().zip(&bitmaps) {
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(bitmap.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += bitmap.len();
    }
    for bitmap in bitmaps {
        out.extend_from_slice(&bitmap);
    }
    out
}

/// One .ico image: a 32-bit bitmap, bottom row first, then its 1-bit mask.
fn ico_bitmap(size: usize, rgba: &[u8]) -> Vec<u8> {
    let mask_row = size.div_ceil(32) * 4;
    let mut out = Vec::new();
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(size as i32).to_le_bytes());
    out.extend_from_slice(&(size as i32 * 2).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((size * size * 4 + mask_row * size) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);
    for y in (0..size).rev() {
        for x in 0..size {
            let p = &rgba[(y * size + x) * 4..][..4];
            out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    for y in (0..size).rev() {
        let mut row = vec![0u8; mask_row];
        for x in 0..size {
            if rgba[(y * size + x) * 4 + 3] == 0 {
                row[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&row);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_are_see_through_and_the_middle_is_solid() {
        let rgba = icon_rgba();
        assert_eq!(rgba.len(), ICON_SIZE * ICON_SIZE * 4);
        let alpha = |x: usize, y: usize| rgba[(y * ICON_SIZE + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(ICON_SIZE - 1, ICON_SIZE - 1), 0);
        assert_eq!(alpha(ICON_SIZE / 2, ICON_SIZE / 2), 255);
    }

    #[test]
    fn ico_lists_every_size() {
        let ico = icon_ico();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 4);
        // The last image ends exactly at the end of the file.
        let entry = &ico[6 + 16 * 3..6 + 16 * 4];
        let len = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        assert_eq!(offset + len, ico.len());
    }
}
