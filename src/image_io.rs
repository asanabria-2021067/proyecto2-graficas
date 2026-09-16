// El lector BMP se usa a partir de fase 3 (texturas desde assets/).
#![allow(dead_code)]

use std::fs::File;
use std::io::{self, Write};

use crate::framebuffer::{u32_to_rgb, Framebuffer};

// ---------- CRC32 (PNG chunk checksum) ----------

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0u32;
    while (n as usize) < 256 {
        let mut c = n;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[n as usize] = c;
        n += 1;
    }
    table
}

fn crc32(data: &[u8]) -> u32 {
    let table = crc32_table();
    let mut c: u32 = 0xFFFFFFFF;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFFFFFF
}

// ---------- Adler32 (zlib checksum) ----------

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

// ---------- Deflate, "stored" (uncompressed) blocks only ----------

fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 65535 * 5 + 5);
    if data.is_empty() {
        out.push(0x01);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0xFFFFu16.to_le_bytes());
        return out;
    }
    let mut i = 0;
    let n = data.len();
    while i < n {
        let remaining = n - i;
        let block_len = remaining.min(65535);
        let is_final = i + block_len >= n;
        out.push(if is_final { 0x01 } else { 0x00 });
        let len = block_len as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(&data[i..i + block_len]);
        i += block_len;
    }
    out
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x78);
    out.push(0x01);
    out.extend(deflate_stored(data));
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

// ---------- PNG writer ----------

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut chunk_body = Vec::with_capacity(4 + data.len());
    chunk_body.extend_from_slice(kind);
    chunk_body.extend_from_slice(data);
    out.extend_from_slice(&chunk_body);
    out.extend_from_slice(&crc32(&chunk_body).to_be_bytes());
}

/// Encodes an RGB8 image as an uncompressed-deflate PNG using only std.
pub fn write_png(path: &str, width: u32, height: u32, rgb: &[u8]) -> io::Result<()> {
    assert_eq!(rgb.len(), (width * height * 3) as usize);

    let mut raw = Vec::with_capacity((height * (1 + width * 3)) as usize);
    for y in 0..height {
        raw.push(0u8); // filter type: None
        let row_start = (y * width * 3) as usize;
        raw.extend_from_slice(&rgb[row_start..row_start + (width * 3) as usize]);
    }
    let compressed = zlib_compress(&raw);

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type: RGB
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut out, b"IHDR", &ihdr);
    write_chunk(&mut out, b"IDAT", &compressed);
    write_chunk(&mut out, b"IEND", &[]);

    let mut file = File::create(path)?;
    file.write_all(&out)
}

pub fn write_framebuffer_png(path: &str, fb: &Framebuffer) -> io::Result<()> {
    let mut rgb = Vec::with_capacity((fb.width * fb.height * 3) as usize);
    for &p in &fb.pixels {
        let (r, g, b) = u32_to_rgb(p);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    write_png(path, fb.width, fb.height, &rgb)
}

// ---------- BMP reader (24/32 bit, uncompressed) ----------

pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>, // 4 bytes per pixel, row 0 = top
}

pub fn read_bmp(path: &str) -> io::Result<Bitmap> {
    let data = std::fs::read(path)?;
    read_bmp_bytes(&data)
}

fn read_bmp_bytes(data: &[u8]) -> io::Result<Bitmap> {
    let err = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_string());
    if data.len() < 54 || data[0] != b'B' || data[1] != b'M' {
        return Err(err("not a BMP file"));
    }
    let data_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let dib_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);
    if dib_size < 40 {
        return Err(err("unsupported BMP DIB header"));
    }
    let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let height_raw = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bpp = u16::from_le_bytes([data[28], data[29]]);
    let compression = u32::from_le_bytes([data[30], data[31], data[32], data[33]]);
    if compression != 0 {
        return Err(err("compressed BMP not supported"));
    }
    if bpp != 24 && bpp != 32 {
        return Err(err("only 24/32 bit BMP supported"));
    }
    let width = width as u32;
    let flip = height_raw > 0;
    let height = height_raw.unsigned_abs();

    let bytes_per_pixel = (bpp / 8) as usize;
    let row_stride = (width as usize * bytes_per_pixel).div_ceil(4) * 4;

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        let src_row = if flip { height - 1 - y } else { y };
        let row_start = data_offset + src_row as usize * row_stride;
        for x in 0..width {
            let px = row_start + x as usize * bytes_per_pixel;
            if px + bytes_per_pixel > data.len() {
                return Err(err("BMP truncated"));
            }
            let b = data[px];
            let g = data[px + 1];
            let r = data[px + 2];
            let a = if bpp == 32 { data[px + 3] } else { 255 };
            let dst = ((y * width + x) * 4) as usize;
            rgba[dst] = r;
            rgba[dst + 1] = g;
            rgba[dst + 2] = b;
            rgba[dst + 3] = a;
        }
    }

    Ok(Bitmap { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler32_known_value() {
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"a"), 0x00620062);
    }

    #[test]
    fn crc32_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn png_roundtrip_smoke() {
        let w = 4;
        let h = 2;
        let rgb = vec![255u8; (w * h * 3) as usize];
        let path = std::env::temp_dir().join("diorama_test.png");
        let path_str = path.to_str().unwrap();
        write_png(path_str, w, h, &rgb).unwrap();
        let meta = std::fs::metadata(path_str).unwrap();
        assert!(meta.len() > 8);
        std::fs::remove_file(path_str).ok();
    }
}
