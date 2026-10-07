//! Fixed RGB names and hexadecimal literals shared by evaluation, scenes and portable exports.
/// Resolve a fixed display color; no host theme, network lookup or arbitrary expression execution.
pub fn rgb(source: &str) -> Option<[u8; 3]> {
    let hex = match source {
        "green" => "21854a",
        "blue" => "3266b0",
        "red" => "bc3838",
        "orange" => "ba7425",
        "purple" => "81549e",
        "cyan" => "248d91",
        "black" => "000000",
        "white" => "ffffff",
        "dark_green" => "14532d",
        "light_green" => "a3d977",
        _ if source.len() == 7
            && source.starts_with('#')
            && source[1..].bytes().all(|v| v.is_ascii_hexdigit()) =>
        {
            &source[1..]
        }
        _ => return None,
    };
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}
