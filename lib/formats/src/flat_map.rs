//! Builds a flat, fully in-memory map (GAT + GND + RSW) so the client can enter
//! a map that ships no files in the GRF. The walkability rule must match the
//! server's mcache for the same map.

use crate::FormatError;

/// Describes the flat map to generate. Dimensions are in GAT cells.
pub struct FlatMapSpec {
    pub name: String,
    pub gat_width: i32,
    pub gat_height: i32,
    /// Ground texture, as a path relative to `data/texture/`.
    pub texture: String,
}

/// Raw bytes of the three map files, ready to be served as `<name>.gat/.gnd/.rsw`.
pub struct FlatMapFiles {
    pub gat: Vec<u8>,
    pub gnd: Vec<u8>,
    pub rsw: Vec<u8>,
}

/// Largest accepted side, in GAT cells. Real maps stay far below this; the cap
/// keeps a bad spec from allocating gigabytes.
const MAX_SIDE: i32 = 4096;
/// Width of the texture name field in a GND file.
const TEXTURE_FIELD_LEN: usize = 80;
/// Width of the file name fields in an RSW file.
const RSW_NAME_FIELD_LEN: usize = 40;

const GAT_BLOCKED: i32 = 1;
const GAT_WALKABLE: i32 = 0;
/// Raw water type below zero makes the renderer skip water entirely.
const NO_WATER: i32 = -1;

fn invalid(msg: String) -> FormatError {
    FormatError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, msg))
}

fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes `s` as an EUC-KR string NUL-padded to `len` bytes. `s` must be shorter
/// than `len`, so the field always ends in at least one NUL.
fn put_padded_string(
    out: &mut Vec<u8>,
    s: &str,
    len: usize,
    what: &str,
) -> Result<(), FormatError> {
    let (encoded, _, had_errors) = encoding_rs::EUC_KR.encode(s);
    if had_errors || encoded.contains(&0) {
        return Err(invalid(format!(
            "{what} {s:?} is not a valid EUC-KR string"
        )));
    }
    if encoded.len() >= len {
        return Err(invalid(format!(
            "{what} {s:?} is {} bytes, must be under {len}",
            encoded.len()
        )));
    }
    out.extend_from_slice(&encoded);
    out.resize(out.len() + (len - encoded.len()), 0);
    Ok(())
}

fn validate(spec: &FlatMapSpec) -> Result<(), FormatError> {
    for (axis, v) in [
        ("gat_width", spec.gat_width),
        ("gat_height", spec.gat_height),
    ] {
        if v <= 0 || v % 2 != 0 || v > MAX_SIDE {
            return Err(invalid(format!(
                "{axis} must be a positive even number up to {MAX_SIDE}, got {v}"
            )));
        }
    }
    if spec.name.is_empty() {
        return Err(invalid("map name must not be empty".to_string()));
    }
    if spec.texture.is_empty() {
        return Err(invalid("texture must not be empty".to_string()));
    }
    Ok(())
}

fn build_gat(spec: &FlatMapSpec) -> Vec<u8> {
    let (w, h) = (spec.gat_width, spec.gat_height);
    let mut out = Vec::with_capacity(14 + (w * h) as usize * 20);
    out.extend_from_slice(b"GRAT");
    out.extend_from_slice(&[1, 2]);
    put_i32(&mut out, w);
    put_i32(&mut out, h);
    for y in 0..h {
        for x in 0..w {
            for _ in 0..4 {
                put_f32(&mut out, 0.0);
            }
            let walkable = (1..=w - 2).contains(&x) && (1..=h - 2).contains(&y);
            put_i32(&mut out, if walkable { GAT_WALKABLE } else { GAT_BLOCKED });
        }
    }
    out
}

fn build_gnd(spec: &FlatMapSpec) -> Result<Vec<u8>, FormatError> {
    let (w, h) = (spec.gat_width / 2, spec.gat_height / 2);
    let mut out = Vec::with_capacity(256 + (w * h) as usize * 28);
    out.extend_from_slice(b"GRGN");
    out.extend_from_slice(&[1, 7]);
    put_i32(&mut out, w);
    put_i32(&mut out, h);
    put_f32(&mut out, 10.0);

    put_i32(&mut out, 1);
    put_i32(&mut out, TEXTURE_FIELD_LEN as i32);
    put_padded_string(&mut out, &spec.texture, TEXTURE_FIELD_LEN, "texture")?;

    // One 8x8 lightmap: full shadow-map brightness, no colour tint.
    put_i32(&mut out, 1);
    put_i32(&mut out, 8);
    put_i32(&mut out, 8);
    put_i32(&mut out, 1);
    out.extend_from_slice(&[255u8; 64]);
    out.extend_from_slice(&[0u8; 192]);

    put_i32(&mut out, 1);
    for v in [0.0f32, 1.0, 0.0, 1.0] {
        put_f32(&mut out, v);
    }
    for v in [0.0f32, 0.0, 1.0, 1.0] {
        put_f32(&mut out, v);
    }
    out.extend_from_slice(&0i16.to_le_bytes());
    out.extend_from_slice(&0i16.to_le_bytes());
    out.extend_from_slice(&[255, 255, 255, 255]);

    for _ in 0..(w * h) {
        for _ in 0..4 {
            put_f32(&mut out, 0.0);
        }
        put_i32(&mut out, 0);
        put_i32(&mut out, -1);
        put_i32(&mut out, -1);
    }
    Ok(out)
}

fn build_rsw(spec: &FlatMapSpec) -> Result<Vec<u8>, FormatError> {
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(b"GRSW");
    out.extend_from_slice(&[1, 9]);
    put_padded_string(&mut out, "", RSW_NAME_FIELD_LEN, "ini file")?;
    put_padded_string(
        &mut out,
        &format!("{}.gnd", spec.name),
        RSW_NAME_FIELD_LEN,
        "gnd file",
    )?;
    put_padded_string(
        &mut out,
        &format!("{}.gat", spec.name),
        RSW_NAME_FIELD_LEN,
        "gat file",
    )?;
    put_padded_string(&mut out, "", RSW_NAME_FIELD_LEN, "source file")?;

    // water: level, type, wave height, wave speed, wave pitch, anim speed
    put_f32(&mut out, 0.0);
    put_i32(&mut out, NO_WATER);
    put_f32(&mut out, 1.0);
    put_f32(&mut out, 2.0);
    put_f32(&mut out, 50.0);
    put_i32(&mut out, 3);

    // light: longitude, latitude, diffuse, ambient, shadow map alpha
    put_i32(&mut out, 45);
    put_i32(&mut out, 45);
    for v in [1.0f32, 1.0, 1.0, 0.3, 0.3, 0.3] {
        put_f32(&mut out, v);
    }
    put_f32(&mut out, 1.0);

    // ground bounds: top, bottom, left, right
    for v in [-500, 500, -500, 500] {
        put_i32(&mut out, v);
    }

    // object count
    put_i32(&mut out, 0);
    Ok(out)
}

pub fn build_flat_map(spec: &FlatMapSpec) -> Result<FlatMapFiles, FormatError> {
    validate(spec)?;
    // Build the cheap RSW first so an oversized name fails before the big buffers.
    let rsw = build_rsw(spec)?;
    let gnd = build_gnd(spec)?;
    let gat = build_gat(spec);
    Ok(FlatMapFiles { gat, gnd, rsw })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gat::GatFile;
    use crate::gnd::GndFile;
    use crate::rsw::RswFile;

    const TEXTURE: &str = "\u{c720}\u{c800}\u{c778}\u{d130}\u{d398}\u{c774}\u{c2a4}\\grass.bmp";

    fn spec(w: i32, h: i32) -> FlatMapSpec {
        FlatMapSpec {
            name: "tanthu".to_string(),
            gat_width: w,
            gat_height: h,
            texture: TEXTURE.to_string(),
        }
    }

    fn tanthu() -> FlatMapFiles {
        build_flat_map(&spec(64, 64)).expect("tanthu spec is valid")
    }

    fn expected_walkable(x: i32, y: i32, w: i32, h: i32) -> bool {
        (1..=w - 2).contains(&x) && (1..=h - 2).contains(&y)
    }

    #[test]
    fn gat_is_64x64_with_walkable_interior_and_blocked_border() {
        let gat = GatFile::parse(&tanthu().gat).unwrap();
        assert_eq!((gat.width, gat.height), (64, 64));
        assert_eq!(gat.version, (1, 2));
        for (x, y) in [(32, 32), (1, 1), (62, 62), (1, 62), (62, 1)] {
            assert!(gat.is_walkable(x, y), "({x},{y}) must be walkable");
        }
        for (x, y) in [(0, 0), (63, 10), (10, 63), (0, 32), (63, 63), (32, 0)] {
            assert!(!gat.is_walkable(x, y), "({x},{y}) must be blocked");
        }
        let walkable = gat.cells.iter().filter(|c| c.is_walkable()).count();
        assert_eq!(walkable, 62 * 62);
    }

    #[test]
    fn gat_cells_are_flat_at_height_zero() {
        let gat = GatFile::parse(&tanthu().gat).unwrap();
        assert!(gat.cells.iter().all(|c| c.height_sw == 0.0
            && c.height_se == 0.0
            && c.height_nw == 0.0
            && c.height_ne == 0.0));
        assert_eq!(gat.get_height(32.5, 32.5), 0.0);
    }

    #[test]
    fn gat_walkability_follows_rule_for_non_square_map() {
        let files = build_flat_map(&spec(6, 4)).unwrap();
        let gat = GatFile::parse(&files.gat).unwrap();
        assert_eq!((gat.width, gat.height), (6, 4));
        for y in 0..4 {
            for x in 0..6 {
                assert_eq!(
                    gat.is_walkable(x, y),
                    expected_walkable(x, y, 6, 4),
                    "cell ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn smallest_map_has_no_walkable_cell() {
        let files = build_flat_map(&spec(2, 2)).unwrap();
        let gat = GatFile::parse(&files.gat).unwrap();
        assert_eq!(gat.cells.len(), 4);
        assert!(gat.cells.iter().all(|c| !c.is_walkable()));
        let gnd = GndFile::parse(&files.gnd).unwrap();
        assert_eq!((gnd.width, gnd.height), (1, 1));
    }

    #[test]
    fn gnd_describes_one_flat_surface_over_half_resolution_grid() {
        let gnd = GndFile::parse(&tanthu().gnd).unwrap();
        assert_eq!(gnd.version, (1, 7));
        assert_eq!((gnd.width, gnd.height), (32, 32));
        assert_eq!(gnd.zoom, 10.0);
        assert_eq!(gnd.textures, vec![TEXTURE.to_string()]);
        assert_eq!(gnd.cells.len(), 32 * 32);
        assert_eq!(gnd.surfaces.len(), 1);
        let s = &gnd.surfaces[0];
        assert_eq!(s.texture_id, 0);
        assert_eq!(s.lightmap_id, 0);
        assert_eq!(s.tex_u, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(s.tex_v, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(s.color_bgra, [255, 255, 255, 255]);
        for c in &gnd.cells {
            assert_eq!(
                [c.height_sw, c.height_se, c.height_nw, c.height_ne],
                [0.0; 4]
            );
            assert_eq!(c.surface_up, 0);
            assert_eq!(c.surface_south, -1);
            assert_eq!(c.surface_east, -1);
        }
    }

    #[test]
    fn gnd_lightmap_is_fully_lit_and_uncoloured() {
        let gnd = GndFile::parse(&tanthu().gnd).unwrap();
        assert_eq!(gnd.lightmaps.len(), 1);
        assert!(gnd.has_lightmap_data());
        assert!(gnd.lightmaps[0].shadow.iter().all(|&b| b == 255));
        assert!(gnd.lightmaps[0].color.iter().all(|&b| b == 0));
    }

    #[test]
    fn gnd_has_no_trailing_bytes() {
        // header 4+2+4+4+4, textures 4+4+80, lightmaps 4*4+256, surfaces 4+(32+4+4),
        // cells 32*32*(16+12)
        let expected = 18 + 88 + 16 + 256 + 4 + 40 + 32 * 32 * 28;
        assert_eq!(tanthu().gnd.len(), expected);
    }

    #[test]
    fn gnd_texture_may_be_79_bytes_but_not_80() {
        let mut s = spec(64, 64);
        s.texture = "a".repeat(79);
        let gnd = GndFile::parse(&build_flat_map(&s).unwrap().gnd).unwrap();
        assert_eq!(gnd.textures, vec!["a".repeat(79)]);

        s.texture = "a".repeat(80);
        assert!(build_flat_map(&s).is_err());
    }

    #[test]
    fn rsw_points_at_generated_files_and_disables_water() {
        let rsw = RswFile::parse(&tanthu().rsw).unwrap();
        assert_eq!(rsw.version, (1, 9));
        assert_eq!(rsw.ini_file, "");
        assert_eq!(rsw.gnd_file, "tanthu.gnd");
        assert_eq!(rsw.gat_file, "tanthu.gat");
        assert_eq!(rsw.source_file.as_deref(), Some(""));
        assert_eq!(rsw.water.water_type, Some(-1));
        assert_eq!(rsw.water.level, Some(0.0));
        assert_eq!(rsw.water.wave_height, Some(1.0));
        assert_eq!(rsw.water.wave_speed, Some(2.0));
        assert_eq!(rsw.water.wave_pitch, Some(50.0));
        assert_eq!(rsw.water.anim_speed, Some(3));
        assert!(rsw.objects.is_empty());
    }

    #[test]
    fn rsw_light_and_ground_bounds() {
        let rsw = RswFile::parse(&tanthu().rsw).unwrap();
        assert_eq!(rsw.light.longitude, Some(45));
        assert_eq!(rsw.light.latitude, Some(45));
        assert_eq!(rsw.light.diffuse, Some([1.0, 1.0, 1.0]));
        assert_eq!(rsw.light.ambient, Some([0.3, 0.3, 0.3]));
        assert_eq!(rsw.light.shadow_map_alpha, Some(1.0));
        assert_eq!(rsw.ground_top, Some(-500));
        assert_eq!(rsw.ground_bottom, Some(500));
        assert_eq!(rsw.ground_left, Some(-500));
        assert_eq!(rsw.ground_right, Some(500));
    }

    #[test]
    fn rsw_file_names_follow_spec_name() {
        let mut s = spec(64, 64);
        s.name = "flat_test".to_string();
        let rsw = RswFile::parse(&build_flat_map(&s).unwrap().rsw).unwrap();
        assert_eq!(rsw.gnd_file, "flat_test.gnd");
        assert_eq!(rsw.gat_file, "flat_test.gat");
    }

    #[test]
    fn rejects_odd_zero_or_negative_dimensions() {
        for (w, h) in [(63, 64), (64, 63), (0, 64), (64, 0), (-64, 64), (64, -2)] {
            assert!(
                build_flat_map(&spec(w, h)).is_err(),
                "{w}x{h} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_oversized_dimensions_without_allocating() {
        assert!(build_flat_map(&spec(i32::MAX - 1, i32::MAX - 1)).is_err());
        assert!(build_flat_map(&spec(1 << 20, 2)).is_err());
    }

    #[test]
    fn rejects_empty_or_too_long_texture() {
        let mut s = spec(64, 64);
        s.texture = String::new();
        assert!(build_flat_map(&s).is_err());
        // 27 Hangul syllables are 54 bytes in EUC-KR but 81 in UTF-8: only the
        // encoded length counts toward the 80-byte field.
        s.texture = "\u{c720}".repeat(27);
        assert!(build_flat_map(&s).is_ok());
        s.texture = "\u{c720}".repeat(40);
        assert!(build_flat_map(&s).is_err());
    }

    #[test]
    fn rejects_texture_that_cannot_be_encoded_or_has_nul() {
        let mut s = spec(64, 64);
        s.texture = "\u{1F600}.bmp".to_string();
        assert!(build_flat_map(&s).is_err());
        s.texture = "a\0b.bmp".to_string();
        assert!(build_flat_map(&s).is_err());
    }

    #[test]
    fn rejects_empty_or_too_long_map_name() {
        let mut s = spec(64, 64);
        s.name = String::new();
        assert!(build_flat_map(&s).is_err());
        // "<name>.gnd" must fit the 40-byte RSW field.
        s.name = "n".repeat(36);
        assert!(build_flat_map(&s).is_err());
        s.name = "n".repeat(35);
        assert!(build_flat_map(&s).is_ok());
    }
}
