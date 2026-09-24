//! Minimal image helpers: PNG/JPEG writing, downscaling, contact sheets, and
//! a tiny bitmap font for labels.

use std::path::Path;

#[derive(Clone)]
pub struct Rgba {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Rgba {
    pub fn new(w: u32, h: u32, fill: [u8; 4]) -> Self {
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            px.extend_from_slice(&fill);
        }
        Self { w, h, px }
    }

    pub fn from_raw(w: u32, h: u32, px: Vec<u8>) -> Self {
        Self { w, h, px }
    }

    /// Box-filter downscale by an integer-free ratio.
    pub fn resize(&self, nw: u32, nh: u32) -> Rgba {
        let mut out = Rgba::new(nw, nh, [0; 4]);
        let sx = self.w as f32 / nw as f32;
        let sy = self.h as f32 / nh as f32;
        for y in 0..nh {
            for x in 0..nw {
                let x0 = (x as f32 * sx) as u32;
                let x1 = (((x + 1) as f32 * sx) as u32).max(x0 + 1);
                let y0 = (y as f32 * sy) as u32;
                let y1 = (((y + 1) as f32 * sy) as u32).max(y0 + 1);
                let mut acc = [0u32; 4];
                let mut n = 0;
                for yy in y0..y1.min(self.h) {
                    for xx in x0..x1.min(self.w) {
                        let i = ((yy * self.w + xx) * 4) as usize;
                        for c in 0..4 {
                            acc[c] += self.px[i + c] as u32;
                        }
                        n += 1;
                    }
                }
                let o = ((y * nw + x) * 4) as usize;
                for c in 0..4 {
                    out.px[o + c] = (acc[c] / n.max(1)) as u8;
                }
            }
        }
        out
    }

    pub fn blit(&mut self, src: &Rgba, ox: u32, oy: u32) {
        for y in 0..src.h {
            for x in 0..src.w {
                let (dx, dy) = (ox + x, oy + y);
                if dx >= self.w || dy >= self.h {
                    continue;
                }
                let s = ((y * src.w + x) * 4) as usize;
                let d = ((dy * self.w + dx) * 4) as usize;
                self.px[d..d + 4].copy_from_slice(&src.px[s..s + 4]);
            }
        }
    }

    pub fn load_png(path: &Path) -> Rgba {
        let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        let px: Vec<u8> = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => buf[..info.buffer_size()].chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            png::ColorType::Grayscale => buf[..info.buffer_size()].iter().flat_map(|&g| [g, g, g, 255]).collect(),
            other => panic!("unsupported PNG colour type {other:?}"),
        };
        Rgba { w: info.width, h: info.height, px }
    }

    pub fn save_png(&self, path: &Path) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        let f = std::fs::File::create(path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), self.w, self.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::High);
        enc.write_header().unwrap().write_image_data(&self.px).unwrap();
    }

    pub fn save_jpeg(&self, path: &Path, quality: u8) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        let rgb: Vec<u8> = self.px.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        std::fs::write(path, crate::jpeg::encode(&rgb, self.w, self.h, quality)).unwrap();
    }

    /// Draws text with the 5x7 font, scaled by `s`.
    pub fn text(&mut self, x: u32, y: u32, s: u32, msg: &str, colour: [u8; 4]) {
        let mut cx = x;
        for ch in msg.chars() {
            let glyph = glyph(ch.to_ascii_uppercase());
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (0x10 >> col) != 0 {
                        for yy in 0..s {
                            for xx in 0..s {
                                let (px, py) = (cx + col * s + xx, y + row as u32 * s + yy);
                                if px < self.w && py < self.h {
                                    let i = ((py * self.w + px) * 4) as usize;
                                    self.px[i..i + 4].copy_from_slice(&colour);
                                }
                            }
                        }
                    }
                }
            }
            cx += 6 * s;
        }
    }
}

/// Lays out tiles in a grid with an optional header row and label column.
pub fn contact_sheet(
    tiles: &[Vec<Rgba>],
    tile: u32,
    col_labels: &[String],
    row_labels: &[String],
) -> Rgba {
    let rows = tiles.len() as u32;
    let cols = tiles.iter().map(|r| r.len()).max().unwrap_or(0) as u32;
    let head = if col_labels.is_empty() { 0 } else { 28 };
    let side = if row_labels.is_empty() { 0 } else { 96 };
    let gap = 4;
    let mut sheet = Rgba::new(side + cols * (tile + gap) + gap, head + rows * (tile + gap) + gap, [18, 20, 24, 255]);
    for (c, l) in col_labels.iter().enumerate() {
        sheet.text(side + gap + c as u32 * (tile + gap) + 6, 8, 2, l, [220, 220, 220, 255]);
    }
    for (r, row) in tiles.iter().enumerate() {
        if let Some(l) = row_labels.get(r) {
            for (k, line) in l.split('\n').enumerate() {
                sheet.text(6, head + gap + r as u32 * (tile + gap) + 8 + k as u32 * 12, 1, line, [220, 220, 220, 255]);
            }
        }
        for (c, t) in row.iter().enumerate() {
            let t = if t.w != tile { t.resize(tile, tile) } else { t.clone() };
            sheet.blit(&t, side + gap + c as u32 * (tile + gap), head + gap + r as u32 * (tile + gap));
        }
    }
    sheet
}

fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x11, 0x0A, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '.' => [0, 0, 0, 0, 0, 0x0C, 0x0C],
        ':' => [0, 0x0C, 0x0C, 0, 0x0C, 0x0C, 0],
        '-' => [0, 0, 0, 0x1F, 0, 0, 0],
        '+' => [0, 0x04, 0x04, 0x1F, 0x04, 0x04, 0],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        '%' => [0x18, 0x19, 0x02, 0x04, 0x08, 0x13, 0x03],
        _ => [0; 7],
    }
}
