//! Maps the client generates in memory when the GRF does not carry them.

use ragnarok_formats::flat_map::{FlatMapSpec, build_flat_map};
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;

/// Login map for new characters. Must match the server's `tanthu` mcache:
/// 64x64 cells, walkable interior, blocked one-cell border.
const TANTHU: &str = "tanthu";
const TANTHU_SIZE: i32 = 64;

/// Maps whose first ground texture is borrowed for the generated map, in order.
const TEXTURE_SOURCES: [&str; 2] = ["new_1-1", "prontera"];

fn first_texture(grf: &GrfArchive, map_name: &str) -> Option<String> {
    let path = ragnarok_resources::map::gnd(map_name);
    let data = grf.read_file(&path).ok()?;
    match GndFile::parse(&data) {
        Ok(gnd) => gnd.textures.into_iter().next().filter(|t| !t.is_empty()),
        Err(e) => {
            tracing::warn!("Cannot read ground texture from {path}: {e}");
            None
        }
    }
}

/// Installs the generated maps into `grf` as overrides and returns how many it
/// added. A map the archive already provides is left alone, and a missing
/// texture source skips the map (the client then shows its missing-map window).
pub fn register_builtin_maps(grf: &mut GrfArchive) -> usize {
    if grf.read_file(&ragnarok_resources::map::rsw(TANTHU)).is_ok() {
        tracing::info!("{TANTHU} already provided by the data source, not generating it");
        return 0;
    }

    let Some(texture) = TEXTURE_SOURCES
        .iter()
        .find_map(|name| first_texture(grf, name))
    else {
        tracing::warn!("No ground texture source found, {TANTHU} not generated");
        return 0;
    };

    let spec = FlatMapSpec {
        name: TANTHU.to_string(),
        gat_width: TANTHU_SIZE,
        gat_height: TANTHU_SIZE,
        texture,
    };
    let files = match build_flat_map(&spec) {
        Ok(files) => files,
        Err(e) => {
            tracing::warn!("Cannot generate {TANTHU}: {e}");
            return 0;
        }
    };

    grf.set_override(&ragnarok_resources::map::gat(TANTHU), files.gat);
    grf.set_override(&ragnarok_resources::map::gnd(TANTHU), files.gnd);
    grf.set_override(&ragnarok_resources::map::rsw(TANTHU), files.rsw);
    tracing::info!("Generated flat map {TANTHU} ({TANTHU_SIZE}x{TANTHU_SIZE})");
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_loader::load_map_data;
    use ragnarok_formats::flat_map::{FlatMapSpec, build_flat_map};
    use ragnarok_formats::gnd::GndFile;
    use std::path::PathBuf;

    /// Temp GRF removed on drop; the name is unique per test so runs stay independent.
    struct TempGrf {
        path: PathBuf,
    }

    impl TempGrf {
        fn path_for(test: &str) -> PathBuf {
            std::env::temp_dir().join(format!("ro_builtin_maps_{}_{test}.grf", std::process::id()))
        }

        fn create(test: &str, files: &[(&str, Vec<u8>)]) -> (Self, GrfArchive) {
            let path = Self::path_for(test);
            let _ = std::fs::remove_file(&path);
            let mut grf = GrfArchive::create(&path).expect("create temp grf");
            for (name, data) in files {
                grf.add_file(name, data).expect("add file");
            }
            grf.save().expect("save temp grf");
            (Self { path }, grf)
        }
    }

    impl Drop for TempGrf {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// A valid GND whose only texture is `texture`.
    fn gnd_with_texture(texture: &str) -> Vec<u8> {
        build_flat_map(&FlatMapSpec {
            name: "src".to_string(),
            gat_width: 4,
            gat_height: 4,
            texture: texture.to_string(),
        })
        .unwrap()
        .gnd
    }

    fn registered_texture(grf: &GrfArchive) -> String {
        let gnd = GndFile::parse(&grf.read_file("data/tanthu.gnd").unwrap()).unwrap();
        gnd.textures[0].clone()
    }

    #[test]
    fn registers_tanthu_using_new_1_1_texture() {
        let (_tmp, mut grf) = TempGrf::create(
            "registers",
            &[("data/new_1-1.gnd", gnd_with_texture("new\\floor.bmp"))],
        );
        assert_eq!(register_builtin_maps(&mut grf), 1);

        let map = load_map_data(&grf, "tanthu").expect("tanthu loads");
        let gat = map.gat.expect("gat present");
        assert_eq!((gat.width, gat.height), (64, 64));
        assert_eq!((map.gnd.width, map.gnd.height), (32, 32));
        assert_eq!(map.gnd.textures, vec!["new\\floor.bmp".to_string()]);
        assert!(gat.is_walkable(32, 32));
        assert!(!gat.is_walkable(0, 0));
    }

    #[test]
    fn existing_tanthu_rsw_is_left_untouched() {
        let (_tmp, mut grf) = TempGrf::create(
            "existing",
            &[
                ("data/tanthu.rsw", b"real map marker".to_vec()),
                ("data/new_1-1.gnd", gnd_with_texture("new\\floor.bmp")),
            ],
        );
        assert_eq!(register_builtin_maps(&mut grf), 0);
        assert_eq!(
            grf.read_file("data/tanthu.rsw").unwrap(),
            b"real map marker"
        );
        assert!(grf.read_file("data/tanthu.gnd").is_err());
        assert!(grf.read_file("data/tanthu.gat").is_err());
    }

    #[test]
    fn no_source_gnd_registers_nothing_and_does_not_panic() {
        let (_tmp, mut grf) = TempGrf::create("no_source", &[("data/other.txt", vec![1, 2, 3])]);
        assert_eq!(register_builtin_maps(&mut grf), 0);
        assert!(grf.read_file("data/tanthu.rsw").is_err());
        assert!(load_map_data(&grf, "tanthu").is_none());
    }

    #[test]
    fn falls_back_to_prontera_texture() {
        let (_tmp, mut grf) = TempGrf::create(
            "prontera",
            &[("data/prontera.gnd", gnd_with_texture("prt\\road.bmp"))],
        );
        assert_eq!(register_builtin_maps(&mut grf), 1);
        assert_eq!(registered_texture(&grf), "prt\\road.bmp");
    }

    #[test]
    fn new_1_1_wins_over_prontera() {
        let (_tmp, mut grf) = TempGrf::create(
            "precedence",
            &[
                ("data/prontera.gnd", gnd_with_texture("prt\\road.bmp")),
                ("data/new_1-1.gnd", gnd_with_texture("new\\floor.bmp")),
            ],
        );
        assert_eq!(register_builtin_maps(&mut grf), 1);
        assert_eq!(registered_texture(&grf), "new\\floor.bmp");
    }

    #[test]
    fn corrupt_new_1_1_falls_back_to_prontera() {
        let (_tmp, mut grf) = TempGrf::create(
            "corrupt",
            &[
                ("data/new_1-1.gnd", b"GRGN\x01\x07trunc".to_vec()),
                ("data/prontera.gnd", gnd_with_texture("prt\\road.bmp")),
            ],
        );
        assert_eq!(register_builtin_maps(&mut grf), 1);
        assert_eq!(registered_texture(&grf), "prt\\road.bmp");
    }

    #[test]
    fn corrupt_sources_register_nothing() {
        let (_tmp, mut grf) = TempGrf::create(
            "all_corrupt",
            &[
                ("data/new_1-1.gnd", Vec::new()),
                ("data/prontera.gnd", b"not a gnd".to_vec()),
            ],
        );
        assert_eq!(register_builtin_maps(&mut grf), 0);
        assert!(grf.read_file("data/tanthu.rsw").is_err());
    }

    #[test]
    fn source_gnd_without_textures_falls_back() {
        // Valid GND header claiming zero textures: textures[0] does not exist.
        let mut no_tex = Vec::new();
        no_tex.extend_from_slice(b"GRGN\x01\x07");
        for v in [0i32, 0] {
            no_tex.extend_from_slice(&v.to_le_bytes());
        }
        no_tex.extend_from_slice(&10.0f32.to_le_bytes());
        for v in [0i32, 80, 0, 8, 8, 1, 0] {
            no_tex.extend_from_slice(&v.to_le_bytes());
        }
        let (_tmp, mut grf) = TempGrf::create(
            "no_textures",
            &[
                ("data/new_1-1.gnd", no_tex),
                ("data/prontera.gnd", gnd_with_texture("prt\\road.bmp")),
            ],
        );
        assert_eq!(register_builtin_maps(&mut grf), 1);
        assert_eq!(registered_texture(&grf), "prt\\road.bmp");
    }

    #[test]
    fn second_call_is_a_no_op() {
        let (_tmp, mut grf) = TempGrf::create(
            "twice",
            &[("data/new_1-1.gnd", gnd_with_texture("new\\floor.bmp"))],
        );
        assert_eq!(register_builtin_maps(&mut grf), 1);
        assert_eq!(register_builtin_maps(&mut grf), 0);
        assert!(load_map_data(&grf, "tanthu").is_some());
    }
}
