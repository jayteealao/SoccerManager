//! A minimal baseline JPEG encoder (ITU-T T.81), written here so the crate
//! needs no dependency outside the allowed licences. YCbCr 4:4:4, the
//! standard Annex K quantisation and Huffman tables, a direct float DCT.
//! Slow and simple; it only writes contact sheets.

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21,
    28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61,
    54, 47, 55, 62, 63,
];

const Q_LUMA: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29, 51,
    87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120,
    101, 72, 92, 95, 98, 112, 100, 103, 99,
];

const Q_CHROMA: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99, 47, 66, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99,
];

// Annex K.3 tables: code counts per length (1..16) and symbol values.
const DC_L_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
const DC_L_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const DC_C_BITS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
const DC_C_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const AC_L_BITS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d];
const AC_L_VALS: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14,
    0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09,
    0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a,
    0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65,
    0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88,
    0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9,
    0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca,
    0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea,
    0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
];
const AC_C_BITS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77];
const AC_C_VALS: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32,
    0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16,
    0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39,
    0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64,
    0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86,
    0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8,
    0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9,
    0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
];

/// (code, length) per symbol, built from a BITS/VALS pair (Annex C).
fn huff(bits: &[u8; 16], vals: &[u8]) -> [(u16, u8); 256] {
    let mut t = [(0u16, 0u8); 256];
    let (mut code, mut k) = (0u16, 0usize);
    for (len, &n) in bits.iter().enumerate() {
        for _ in 0..n {
            t[vals[k] as usize] = (code, len as u8 + 1);
            code += 1;
            k += 1;
        }
        code <<= 1;
    }
    t
}

struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn put(&mut self, code: u16, len: u8) {
        self.acc = (self.acc << len) | code as u32;
        self.n += len as u32;
        while self.n >= 8 {
            let b = ((self.acc >> (self.n - 8)) & 0xff) as u8;
            self.out.push(b);
            if b == 0xff {
                self.out.push(0);
            }
            self.n -= 8;
        }
        self.acc &= (1 << self.n) - 1;
    }
    fn flush(&mut self) {
        if self.n > 0 {
            let pad = 8 - self.n as u8;
            self.put((1 << pad) - 1, pad);
        }
    }
}

fn category(v: i32) -> (u8, u16) {
    let a = v.unsigned_abs();
    let size = 32 - a.leading_zeros();
    let bits = if v < 0 { (v - 1) as u32 & ((1 << size) - 1) } else { v as u32 };
    (size as u8, bits as u16)
}

fn scaled(q: &[u8; 64], quality: u8) -> [u8; 64] {
    let quality = quality.clamp(1, 100) as u32;
    let s = if quality < 50 { 5000 / quality } else { 200 - 2 * quality };
    q.map(|v| ((v as u32 * s + 50) / 100).clamp(1, 255) as u8)
}

/// Encodes RGB8 pixels as a baseline JPEG file.
pub fn encode(rgb: &[u8], w: u32, h: u32, quality: u8) -> Vec<u8> {
    let ql = scaled(&Q_LUMA, quality);
    let qc = scaled(&Q_CHROMA, quality);
    let (dcl, dcc) = (huff(&DC_L_BITS, &DC_L_VALS), huff(&DC_C_BITS, &DC_C_VALS));
    let (acl, acc) = (huff(&AC_L_BITS, &AC_L_VALS), huff(&AC_C_BITS, &AC_C_VALS));

    let mut o = vec![0xFF, 0xD8];
    o.extend_from_slice(&[0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0]);
    for (id, q) in [(0u8, &ql), (1, &qc)] {
        o.extend_from_slice(&[0xFF, 0xDB, 0, 67, id]);
        o.extend(ZIGZAG.iter().map(|&z| q[z]));
    }
    o.extend_from_slice(&[0xFF, 0xC0, 0, 17, 8, (h >> 8) as u8, h as u8, (w >> 8) as u8, w as u8, 3]);
    o.extend_from_slice(&[1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]);
    for (class_id, bits, vals) in [
        (0x00u8, &DC_L_BITS, &DC_L_VALS[..]),
        (0x10, &AC_L_BITS, &AC_L_VALS[..]),
        (0x01, &DC_C_BITS, &DC_C_VALS[..]),
        (0x11, &AC_C_BITS, &AC_C_VALS[..]),
    ] {
        let len = 2 + 1 + 16 + vals.len();
        o.extend_from_slice(&[0xFF, 0xC4, (len >> 8) as u8, len as u8, class_id]);
        o.extend_from_slice(bits);
        o.extend_from_slice(vals);
    }
    o.extend_from_slice(&[0xFF, 0xDA, 0, 12, 3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]);

    let mut cos = [[0f32; 8]; 8];
    for (x, row) in cos.iter_mut().enumerate() {
        for (u, c) in row.iter_mut().enumerate() {
            *c = (((2 * x + 1) * u) as f32 * std::f32::consts::PI / 16.0).cos();
        }
    }
    let mut bw = Bits { out: Vec::new(), acc: 0, n: 0 };
    let mut pred = [0i32; 3];
    for by in (0..h).step_by(8) {
        for bx in (0..w).step_by(8) {
            let mut blocks = [[0f32; 64]; 3];
            for y in 0..8 {
                for x in 0..8 {
                    let px = (bx + x).min(w - 1);
                    let py = (by + y).min(h - 1);
                    let i = ((py * w + px) * 3) as usize;
                    let (r, g, b) = (rgb[i] as f32, rgb[i + 1] as f32, rgb[i + 2] as f32);
                    let k = (y * 8 + x) as usize;
                    blocks[0][k] = 0.299 * r + 0.587 * g + 0.114 * b - 128.0;
                    blocks[1][k] = -0.168_736 * r - 0.331_264 * g + 0.5 * b;
                    blocks[2][k] = 0.5 * r - 0.418_688 * g - 0.081_312 * b;
                }
            }
            for (c, blk) in blocks.iter().enumerate() {
                let q = if c == 0 { &ql } else { &qc };
                let (dc, ac) = if c == 0 { (&dcl, &acl) } else { (&dcc, &acc) };
                let mut coef = [0i32; 64];
                for v in 0..8 {
                    for u in 0..8 {
                        let mut s = 0.0;
                        for y in 0..8 {
                            for x in 0..8 {
                                s += blk[y * 8 + x] * cos[x][u] * cos[y][v];
                            }
                        }
                        let cu = if u == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
                        let cv = if v == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
                        let f = 0.25 * cu * cv * s;
                        coef[v * 8 + u] = (f / q[v * 8 + u] as f32).round() as i32;
                    }
                }
                let diff = coef[0] - pred[c];
                pred[c] = coef[0];
                let (sz, bits) = category(diff);
                bw.put(dc[sz as usize].0, dc[sz as usize].1);
                if sz > 0 {
                    bw.put(bits, sz);
                }
                let mut run = 0;
                for &z in &ZIGZAG[1..] {
                    let v = coef[z];
                    if v == 0 {
                        run += 1;
                        continue;
                    }
                    while run > 15 {
                        bw.put(ac[0xF0].0, ac[0xF0].1);
                        run -= 16;
                    }
                    let (sz, bits) = category(v);
                    let sym = (run << 4) as usize | sz as usize;
                    bw.put(ac[sym].0, ac[sym].1);
                    bw.put(bits, sz);
                    run = 0;
                }
                if run > 0 {
                    bw.put(ac[0].0, ac[0].1);
                }
            }
        }
    }
    bw.flush();
    o.extend(bw.out);
    o.extend_from_slice(&[0xFF, 0xD9]);
    o
}
