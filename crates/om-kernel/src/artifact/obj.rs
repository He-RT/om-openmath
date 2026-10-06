//! Real world-coordinate indexed geometry, with vertex RGB extension and retained RGBA metadata.
use super::*;
use std::fmt::Write;
pub(super) fn export(
    data: &Scene3DData,
    title: &str,
    ctx: &Interrupt,
) -> Result<Artifact, ArtifactError> {
    crate::scene3d::validate(data, ctx).map_err(|e| invalid(&e.to_string()))?;
    if title.len() > 512 || title.chars().any(|c| c.is_control()) {
        return Err(invalid("OBJ标题无效"));
    }
    let metadata=serde_json::to_string(&serde_json::json!({"format_version":1,"kernel_version":env!("CARGO_PKG_VERSION"),"title":title,"scene":data})).map_err(|_|invalid("场景数据编码失败"))?;
    if metadata.len() > 8 * 1024 * 1024 {
        return Err(invalid("OBJ元数据超过8MiB"));
    }
    let mut text = format!(
        "# OpenMath actual kernel geometry\n# Vertex RGB extension; RGBA and samples are retained in scene_json\n# scene_json {metadata}\n"
    );
    let (mut vertex, mut normal) = (1_u32, 1_u32);
    for (index, mesh) in data.meshes.iter().enumerate() {
        writeln!(text, "o mesh_{index}").map_err(|_| invalid("OBJ格式化失败"))?;
        for (p, c) in mesh.positions.iter().zip(&mesh.colors) {
            ctx.tick()?;
            writeln!(
                text,
                "v {} {} {} {} {} {}",
                p[0], p[1], p[2], c[0], c[1], c[2]
            )
            .map_err(|_| invalid("OBJ格式化失败"))?;
        }
        for n in &mesh.normals {
            ctx.tick()?;
            writeln!(text, "vn {} {} {}", n[0], n[1], n[2])
                .map_err(|_| invalid("OBJ格式化失败"))?;
        }
        for f in &mesh.triangles {
            ctx.tick()?;
            writeln!(
                text,
                "f {}//{} {}//{} {}//{}",
                vertex + f[0],
                normal + f[0],
                vertex + f[1],
                normal + f[1],
                vertex + f[2],
                normal + f[2]
            )
            .map_err(|_| invalid("OBJ格式化失败"))?;
        }
        vertex += mesh.positions.len() as u32;
        normal += mesh.normals.len() as u32;
    }
    for (index, line) in data.lines.iter().enumerate() {
        writeln!(text, "o line_{index}").map_err(|_| invalid("OBJ格式化失败"))?;
        for (p, c) in line.positions.iter().zip(&line.colors) {
            ctx.tick()?;
            writeln!(
                text,
                "v {} {} {} {} {} {}",
                p[0], p[1], p[2], c[0], c[1], c[2]
            )
            .map_err(|_| invalid("OBJ格式化失败"))?;
        }
        if line.positions.len() > 1 {
            text.push('l');
            for i in 0..line.positions.len() {
                ctx.tick()?;
                write!(text, " {}", vertex + i as u32).map_err(|_| invalid("OBJ格式化失败"))?;
            }
            text.push('\n');
        }
        vertex += line.positions.len() as u32;
    }
    for point in &data.points {
        ctx.tick()?;
        let p = point.position;
        let c = point.color;
        writeln!(
            text,
            "v {} {} {} {} {} {}\np {}",
            p[0], p[1], p[2], c[0], c[1], c[2], vertex
        )
        .map_err(|_| invalid("OBJ格式化失败"))?;
        vertex += 1;
    }
    super::encode(text.as_bytes(), "model/obj", "obj", ctx)
}
