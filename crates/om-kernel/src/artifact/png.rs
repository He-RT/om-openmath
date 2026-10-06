//! Minimal PNG RGBA encoder with lossless stored zlib blocks, CRC32 and exact UTF-8 metadata.
use super::{ArtifactError, figure::Drawing, invalid};
use om_core::Interrupt;
fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0_u32;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 == 1 { 0xedb88320 } else { 0 };
        }
    }
    !c
}
fn chunk(
    out: &mut Vec<u8>,
    kind: &[u8; 4],
    body: &[u8],
    ctx: &Interrupt,
) -> Result<(), ArtifactError> {
    for _ in body.chunks(1024) {
        ctx.tick()?;
    }
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let start = out.len() - body.len() - 4;
    out.extend_from_slice(&crc(&out[start..]).to_be_bytes());
    Ok(())
}
pub(super) fn encode(
    d: &Drawing,
    pixels: Vec<u8>,
    ctx: &Interrupt,
) -> Result<Vec<u8>, ArtifactError> {
    let stride = d.width as usize * 4;
    if pixels.len() != stride * d.height as usize {
        return Err(invalid("PNG像素形状无效"));
    }
    let mut raw = Vec::with_capacity(pixels.len() + d.height as usize);
    for row in pixels.chunks(stride) {
        ctx.tick()?;
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks = raw.len().div_ceil(65535);
    for (i, block) in raw.chunks(65535).enumerate() {
        ctx.tick()?;
        z.push(u8::from(i + 1 == blocks));
        let len = block.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for block in raw.chunks(5552) {
        ctx.tick()?;
        for &v in block {
            a += u32::from(v);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = vec![];
    header.extend_from_slice(&d.width.to_be_bytes());
    header.extend_from_slice(&d.height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header, ctx)?;
    chunk(&mut out, b"sRGB", &[0], ctx)?;
    let mut metadata = b"OpenMath\0\0\0\0\0".to_vec();
    metadata.extend_from_slice(d.metadata.as_bytes());
    chunk(&mut out, b"iTXt", &metadata, ctx)?;
    chunk(&mut out, b"IDAT", &z, ctx)?;
    chunk(&mut out, b"IEND", &[], ctx)?;
    Ok(out)
}
