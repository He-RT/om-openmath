//! Pure deterministic SVG/PNG generation and data serialization; no CAS reevaluation or host IO.
mod figure;
mod png;
mod raster;
use crate::protocol::*;
pub use figure::export_plot;
use om_core::{Expr, Interrupt};
/// Real artifact failure, including the shared interrupt budget.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// Invalid geometry, format or pure data.
    #[error("{0}")]
    Invalid(String),
    /// Actual budget/cancellation failure.
    #[error(transparent)]
    Abort(#[from] om_core::Abort),
    /// Existing pure data serializer failure.
    #[error(transparent)]
    Data(#[from] om_eval::EvalError),
}
pub(crate) fn invalid(s: &str) -> ArtifactError {
    ArtifactError::Invalid(s.into())
}
/// Encode checked exact bytes for portable host persistence.
pub fn encode(
    bytes: &[u8],
    mime: &str,
    extension: &str,
    ctx: &Interrupt,
) -> Result<Artifact, ArtifactError> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid("导出字节超过16MiB"));
    }
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for (i, part) in bytes.chunks(3).enumerate() {
        if i % 256 == 0 {
            ctx.tick()?;
        }
        let (a, b, c) = (
            part[0],
            *part.get(1).unwrap_or(&0),
            *part.get(2).unwrap_or(&0),
        );
        out.push(table[(a >> 2) as usize] as char);
        out.push(table[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if part.len() > 1 {
            table[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if part.len() > 2 {
            table[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    Ok(Artifact {
        mime: mime.into(),
        extension: extension.into(),
        base64: out,
        byte_len: bytes.len() as u32,
    })
}
/// Serialize a retained pure value directly, without evaluating strings or symbolic function calls.
pub fn export_data(
    value: &Expr,
    format: DataExportFormat,
    ctx: &Interrupt,
) -> Result<Artifact, ArtifactError> {
    let (text, mime, ext) = match format {
        DataExportFormat::Csv => (
            om_eval::data_csv(value, ctx)?,
            "text/csv;charset=utf-8",
            "csv",
        ),
        DataExportFormat::Json => (
            om_eval::data_json(value, ctx)?,
            "application/json;charset=utf-8",
            "json",
        ),
    };
    encode(text.as_bytes(), mime, ext, ctx)
}

/// Decode the exact produced bytes for native CLI persistence, with canonical padding and length validation.
pub fn decode(artifact: &Artifact) -> Result<Vec<u8>, ArtifactError> {
    let s = artifact.base64.as_bytes();
    if !s.len().is_multiple_of(4) || artifact.byte_len > 16 * 1024 * 1024 {
        return Err(invalid("导出字节格式/长度无效"));
    }
    fn value(b: u8) -> Result<u8, ArtifactError> {
        Ok(match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(invalid("base64字节无效")),
        })
    }
    let mut bytes = Vec::with_capacity(artifact.byte_len as usize);
    for (i, q) in s.chunks_exact(4).enumerate() {
        let a = value(q[0])?;
        let b = value(q[1])?;
        bytes.push((a << 2) | (b >> 4));
        if q[2] == b'=' {
            if q[3] != b'=' || i + 1 != s.len() / 4 || b & 15 != 0 {
                return Err(invalid("base64填充无效"));
            }
            continue;
        }
        let c = value(q[2])?;
        bytes.push((b << 4) | (c >> 2));
        if q[3] == b'=' {
            if i + 1 != s.len() / 4 || c & 3 != 0 {
                return Err(invalid("base64填充无效"));
            }
            continue;
        }
        bytes.push((c << 6) | value(q[3])?);
    }
    if bytes.len() != artifact.byte_len as usize {
        return Err(invalid("导出字节长度不匹配"));
    }
    Ok(bytes)
}
