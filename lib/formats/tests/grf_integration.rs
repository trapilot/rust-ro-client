use std::io::Write;
use std::path::{Path, PathBuf};

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::imf::{ImfFile, ImfLayerOrder};
use ragnarok_formats::pal::PalFile;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_formats::rsw::RswFile;
use ragnarok_formats::spr::SprFile;
use ragnarok_formats::str_effect::StrEffectFile;

/// Pickup action (3) at facing 2, in the action-type * 8 + facing space.
const PICKUP_FACING_2: usize = 26;

fn open_grf() -> Option<GrfArchive> {
    let path = ["data/testdata", "../../data/testdata"]
        .iter()
        .map(Path::new)
        .find(|p| p.exists())?;
    Some(GrfArchive::open(path).expect("failed to open GRF"))
}

fn open_v1_grf() -> Option<GrfArchive> {
    let path = ["data/data.grf", "../../data/data.grf"]
        .iter()
        .map(Path::new)
        .find(|p| p.exists())?;
    Some(GrfArchive::open(path).expect("failed to open v1.x GRF"))
}

#[test]
fn extract_and_parse_all_formats_from_grf() {
    let Some(grf) = open_grf() else {
        eprintln!("Skipping test: data/testdata not found");
        return;
    };
    assert!(grf.file_count() > 0);

    let data = grf.read_file("data/moc_ruins.gat").unwrap();
    let gat = GatFile::parse(&data).expect("failed to parse moc_ruins.gat");
    assert!(gat.width > 0 && gat.height > 0);
    assert_eq!(gat.cells.len(), (gat.width * gat.height) as usize);

    let data = grf.read_file("data/moc_ruins.gnd").unwrap();
    let gnd = GndFile::parse(&data).expect("failed to parse moc_ruins.gnd");
    assert!(gnd.width > 0 && gnd.height > 0);
    assert_eq!(gnd.cells.len(), (gnd.width * gnd.height) as usize);

    let data = grf.read_file("data/moc_ruins.rsw").unwrap();
    let rsw = RswFile::parse(&data).expect("failed to parse moc_ruins.rsw");
    assert!(!rsw.gnd_file.is_empty());
    assert!(!rsw.gat_file.is_empty());

    let gnd_ref = format!("data/{}", rsw.gnd_file);
    GndFile::parse(&grf.read_file(&gnd_ref).unwrap())
        .unwrap_or_else(|e| panic!("failed to parse RSW-referenced {gnd_ref}: {e}"));
    let gat_ref = format!("data/{}", rsw.gat_file);
    GatFile::parse(&grf.read_file(&gat_ref).unwrap())
        .unwrap_or_else(|e| panic!("failed to parse RSW-referenced {gat_ref}: {e}"));

    let data = grf.read_file("data/model/나무잡초꽃/나무01.rsm").unwrap();
    let rsm = RsmFile::parse(&data).expect("failed to parse rsm");
    assert!(!rsm.nodes.is_empty());
    assert!(!rsm.root_node_names.is_empty());

    let data = grf.read_file("data/sprite/몬스터/mandragora.spr").unwrap();
    let spr = SprFile::parse(&data).expect("failed to parse mandragora.spr");
    assert!(spr.indexed_sprites.len() + spr.rgba_sprites.len() > 0);

    let data = grf.read_file("data/sprite/몬스터/mandragora.act").unwrap();
    let act = ActFile::parse(&data).expect("failed to parse mandragora.act");
    assert!(!act.actions.is_empty());

    let data = grf.read_file("data/sprite/이팩트/jong_mini.str").unwrap();
    let str_file = StrEffectFile::parse(&data).expect("failed to parse jong_mini.str");
    assert!(!str_file.layers.is_empty());

    let data = grf.read_file("data/palette/몸/검사_남_0.pal").unwrap();
    PalFile::parse(&data).expect("failed to parse pal");

    let data = grf.read_file("data/imf/검사_남.imf").unwrap();
    let imf = ImfFile::parse(&data).expect("failed to parse imf");
    assert!(!imf.layers.is_empty());

    let order = ImfLayerOrder::from_file(&imf).expect("failed to flatten imf");
    assert!(order.body_over_head(PICKUP_FACING_2, 0));
    assert!(order.body_over_head(PICKUP_FACING_2, 1));
    assert!(!order.body_over_head(0, 0));
}

#[test]
fn open_v1_grf_and_read_file() {
    let Some(grf) = open_v1_grf() else { return };
    assert!(grf.file_count() > 0);

    let gat_file = grf
        .find_first_with_extension(".gat")
        .expect("v1 GRF should contain at least one .gat file");
    let data = grf
        .read_file(gat_file)
        .expect("failed to read .gat from v1 GRF");
    let gat = GatFile::parse(&data).expect("failed to parse .gat from v1 GRF");
    assert!(gat.width > 0 && gat.height > 0);
}

#[test]
fn aliased_map_resolves_through_res_name_table() {
    let Some(grf) = open_v1_grf() else { return };
    if !grf.file_exists(ragnarok_resources::table::RES_NAME) {
        eprintln!("Skipping test: archive has no resnametable.txt");
        return;
    }

    assert!(!grf.file_exists("data/pvp_n_2-2.rsw.missing"));

    let rsw = RswFile::parse(&grf.read_file("data/pvp_n_2-2.rsw").unwrap())
        .expect("pvp_n_2-2 should redirect to a parsable rsw");
    let gnd = GndFile::parse(&grf.read_file("data/pvp_n_2-2.gnd").unwrap())
        .expect("pvp_n_2-2 should redirect to a parsable gnd");
    assert_eq!(rsw.gnd_file.to_ascii_lowercase(), "job_hunter.gnd");
    assert!(gnd.width > 0 && gnd.height > 0);

    let minimap = "data/texture/유저인터페이스/map/pvp_n_2-2.bmp";
    assert!(grf.file_exists(minimap));
    assert_eq!(
        grf.read_file(minimap).unwrap(),
        grf.read_file("data/texture/유저인터페이스/map/job_hunter.bmp")
            .unwrap()
    );
}

fn temp_grf_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("test_grf_{}_{name}.grf", std::process::id()))
}

struct CleanupFile(PathBuf);
impl Drop for CleanupFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

struct CleanupDir(PathBuf);
impl Drop for CleanupDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_grf(path: &Path, files: &[(&str, &[u8])]) {
    let mut grf = GrfArchive::create(path).unwrap();
    for (name, data) in files {
        grf.add_file(name, data).unwrap();
    }
    grf.save().unwrap();
}

#[test]
fn open_layered_first_archive_wins() {
    let primary = temp_grf_path("layer_primary");
    let overlay = temp_grf_path("layer_overlay");
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupFile(overlay.clone());

    write_grf(
        &primary,
        &[
            ("data/shared.txt", b"primary"),
            ("data/only_primary.txt", b"P"),
        ],
    );
    write_grf(
        &overlay,
        &[
            ("data/shared.txt", b"overlay"),
            ("data/only_overlay.txt", b"O"),
        ],
    );

    let grf = GrfArchive::open_layered(
        &[
            primary.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
        ],
        None,
    )
    .unwrap();

    assert_eq!(grf.read_file("data/shared.txt").unwrap(), b"primary");
    assert_eq!(grf.read_file("data/only_overlay.txt").unwrap(), b"O");
    assert_eq!(grf.read_file("data/only_primary.txt").unwrap(), b"P");
    assert!(grf.file_exists("data/only_overlay.txt"));
}

#[test]
fn layered_file_list_resolves_each_name_once() {
    let primary = temp_grf_path("list_primary");
    let overlay = temp_grf_path("list_overlay");
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupFile(overlay.clone());

    write_grf(
        &primary,
        &[("data/shared.txt", b"primary"), ("data/a.txt", b"A")],
    );
    write_grf(
        &overlay,
        &[
            ("data/shared.txt", b"overlay_is_longer"),
            ("data/b.txt", b"B"),
        ],
    );

    let grf = GrfArchive::open_layered(
        &[
            primary.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
        ],
        None,
    )
    .unwrap();

    let list = grf.layered_file_list();
    let names: Vec<&str> = list.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["data/a.txt", "data/shared.txt", "data/b.txt"]);

    let shared = list.iter().find(|f| f.name == "data/shared.txt").unwrap();
    assert_eq!(shared.uncompressed_size, b"primary".len() as u32);
}

#[test]
fn override_wins_over_archive_and_data_dir() {
    let primary = temp_grf_path("override_primary");
    let dir = std::env::temp_dir().join(format!("test_grf_{}_override", std::process::id()));
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupDir(dir.clone());

    write_grf(&primary, &[("data/table.txt", b"from_grf")]);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("table.txt"), b"from_disk").unwrap();

    let mut grf =
        GrfArchive::open_layered(&[primary.to_string_lossy().into_owned()], Some(&dir)).unwrap();
    assert_eq!(grf.read_file("data/table.txt").unwrap(), b"from_disk");

    grf.set_override("data/table.txt", b"merged".to_vec());
    assert_eq!(grf.read_file("data/table.txt").unwrap(), b"merged");
    assert!(grf.file_exists("data/table.txt"));
}

#[test]
fn open_layered_data_dir_overrides_archives() {
    let primary = temp_grf_path("datadir_primary");
    let dir = std::env::temp_dir().join(format!("test_grf_{}_datadir", std::process::id()));
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupDir(dir.clone());

    write_grf(
        &primary,
        &[
            ("data/sub/shared.txt", b"from_grf"),
            ("data/only_grf.txt", b"G"),
        ],
    );

    std::fs::create_dir_all(dir.join("Sub")).unwrap();
    std::fs::write(dir.join("Sub/Shared.txt"), b"from_disk").unwrap();

    let grf =
        GrfArchive::open_layered(&[primary.to_string_lossy().into_owned()], Some(&dir)).unwrap();

    // Disk file wins over the archive, matched case-insensitively without the data/ prefix.
    assert_eq!(grf.read_file("data/sub/shared.txt").unwrap(), b"from_disk");
    assert_eq!(grf.read_file("data/only_grf.txt").unwrap(), b"G");
    assert!(grf.file_exists("data/sub/shared.txt"));
}

#[test]
fn create_and_add_files_roundtrip() {
    let path = temp_grf_path("create_roundtrip");
    let _cleanup = CleanupFile(path.clone());

    let content_a = b"hello world";
    let content_b = vec![0u8; 4096];
    let content_c = b"data in subfolder";

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("readme.txt", content_a).unwrap();
        grf.add_file("data/bigfile.bin", &content_b).unwrap();
        grf.add_file("data/sub/nested.txt", content_c).unwrap();
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 3);
    assert!(grf.file_exists("readme.txt"));
    assert!(grf.file_exists("data/bigfile.bin"));
    assert!(grf.file_exists("data/sub/nested.txt"));
    assert_eq!(grf.read_file("readme.txt").unwrap(), content_a);
    assert_eq!(grf.read_file("data/bigfile.bin").unwrap(), content_b);
    assert_eq!(grf.read_file("data/sub/nested.txt").unwrap(), content_c);
}

#[test]
fn remove_file_and_reopen() {
    let path = temp_grf_path("remove");
    let _cleanup = CleanupFile(path.clone());

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("keep.txt", b"keep me").unwrap();
        grf.add_file("delete.txt", b"delete me").unwrap();
        grf.save().unwrap();
    }

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        assert_eq!(grf.file_count(), 2);
        assert!(grf.remove_file("delete.txt").unwrap());
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert!(grf.file_exists("keep.txt"));
    assert!(!grf.file_exists("delete.txt"));
    assert_eq!(grf.read_file("keep.txt").unwrap(), b"keep me");
}

#[test]
fn repack_reclaims_space() {
    let path = temp_grf_path("repack");
    let _cleanup = CleanupFile(path.clone());

    let large_data = vec![42u8; 10_000];

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("large.bin", &large_data).unwrap();
        grf.save().unwrap();
    }

    let size_before_remove = std::fs::metadata(&path).unwrap().len();

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.remove_file("large.bin").unwrap();
        grf.add_file("small.txt", b"tiny").unwrap();
        grf.save().unwrap();
    }

    let size_after_remove = std::fs::metadata(&path).unwrap().len();
    assert!(size_after_remove > 0);

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.repack().unwrap();
    }

    let size_after_repack = std::fs::metadata(&path).unwrap().len();
    assert!(size_after_repack < size_before_remove);

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("small.txt").unwrap(), b"tiny");
}

#[test]
fn add_file_overwrites_existing() {
    let path = temp_grf_path("overwrite");
    let _cleanup = CleanupFile(path.clone());

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("data/test.txt", b"version A").unwrap();
        grf.save().unwrap();
    }

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.add_file("data/test.txt", b"version B").unwrap();
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("data/test.txt").unwrap(), b"version B");
}

// ---------------------------------------------------------------------------
// Asset name normalization: every name entering or leaving the archive layer
// goes through `ragnarok_asset_names::normalize`.
// ---------------------------------------------------------------------------

use ragnarok_asset_names::{normalize, rename};

/// Writes a GRF 0x200 the way the original tools do: names are stored exactly
/// as given (EUC-KR, any case, any separator) in the order given. `add_file`
/// cannot make these archives because it normalizes.
fn write_raw_grf(path: &Path, files: &[(&str, &[u8])]) {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    const HEADER: usize = 46;
    let zlib = |data: &[u8]| {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    };

    let mut body = Vec::new();
    let mut table = Vec::new();
    for (name, data) in files {
        let compressed = zlib(data);
        let (encoded, _, _) = encoding_rs::EUC_KR.encode(name);
        table.extend_from_slice(&encoded);
        table.push(0);
        table.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        table.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        table.extend_from_slice(&(data.len() as u32).to_le_bytes());
        table.push(0x01);
        table.extend_from_slice(&(body.len() as u32).to_le_bytes());
        body.extend_from_slice(&compressed);
    }
    let table_compressed = zlib(&table);

    let mut header = [0u8; HEADER];
    header[..16].copy_from_slice(b"Master of Magic\0");
    header[30..34].copy_from_slice(&(body.len() as u32).to_le_bytes());
    header[38..42].copy_from_slice(&((files.len() + 7) as u32).to_le_bytes());
    header[42..46].copy_from_slice(&0x200u32.to_le_bytes());

    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&body);
    out.extend_from_slice(&(table_compressed.len() as u32).to_le_bytes());
    out.extend_from_slice(&(table.len() as u32).to_le_bytes());
    out.extend_from_slice(&table_compressed);
    std::fs::write(path, out).unwrap();
}

#[test]
fn raw_helper_writes_an_archive_the_reader_opens() {
    let path = temp_grf_path("raw_helper");
    let _c = CleanupFile(path.clone());
    write_raw_grf(&path, &[("data/plain.txt", b"abc")]);

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("data/plain.txt").unwrap(), b"abc");
}

#[test]
fn korean_entry_is_found_by_korean_and_by_english_name() {
    let path = temp_grf_path("korean_lookup");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[("data\\texture\\유저인터페이스\\a.bmp", b"ui-art")],
    );
    let grf = GrfArchive::open(&path).unwrap();

    let english = format!("data/texture/{}/a.bmp", rename("유저인터페이스"));
    assert_eq!(grf.read_file(&english).unwrap(), b"ui-art");
    assert_eq!(
        grf.read_file("data\\texture\\유저인터페이스\\A.BMP")
            .unwrap(),
        b"ui-art"
    );
    assert!(grf.file_exists(&english));
    assert!(grf.file_exists("data/texture/유저인터페이스/a.bmp"));
}

#[test]
fn entry_names_of_an_old_archive_are_normalized() {
    let path = temp_grf_path("old_names");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[
            ("data\\sprite\\몬스터\\Poring.SPR", b"1"),
            ("data/wav/_effect.wav", b"2"),
        ],
    );
    let grf = GrfArchive::open(&path).unwrap();

    let mut names: Vec<&str> = grf.entry_names().collect();
    names.sort();
    let mut expected = vec![
        normalize("data/sprite/몬스터/poring.spr"),
        "data/wav/_effect.wav".to_string(),
    ];
    expected.sort();
    assert_eq!(names, expected);
    assert!(names.iter().all(|n| n.is_ascii()));
}

#[test]
fn data_dir_beats_archive_across_korean_and_english_names() {
    let primary = temp_grf_path("datadir_korean");
    let dir = std::env::temp_dir().join(format!("test_grf_{}_datadir_ko", std::process::id()));
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupDir(dir.clone());

    write_raw_grf(
        &primary,
        &[("data\\texture\\유저인터페이스\\a.bmp", b"from_grf")],
    );
    let english_dir = dir.join("texture").join(rename("유저인터페이스"));
    std::fs::create_dir_all(&english_dir).unwrap();
    std::fs::write(english_dir.join("a.bmp"), b"from_disk").unwrap();

    let grf =
        GrfArchive::open_layered(&[primary.to_string_lossy().into_owned()], Some(&dir)).unwrap();

    assert_eq!(
        grf.read_file("data/texture/유저인터페이스/a.bmp").unwrap(),
        b"from_disk"
    );
    let english = format!("data/texture/{}/a.bmp", rename("유저인터페이스"));
    assert_eq!(grf.read_file(&english).unwrap(), b"from_disk");
}

#[test]
fn data_dir_files_with_korean_names_are_indexed_by_english_key() {
    let dir = std::env::temp_dir().join(format!("test_grf_{}_datadir_ko2", std::process::id()));
    let primary = temp_grf_path("datadir_ko2");
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupDir(dir.clone());

    write_grf(&primary, &[("data/other.txt", b"x")]);
    std::fs::create_dir_all(dir.join("texture/유저인터페이스")).unwrap();
    std::fs::write(dir.join("texture/유저인터페이스/B.BMP"), b"disk").unwrap();

    let grf =
        GrfArchive::open_layered(&[primary.to_string_lossy().into_owned()], Some(&dir)).unwrap();
    let english = format!("data/texture/{}/b.bmp", rename("유저인터페이스"));
    assert_eq!(grf.read_file(&english).unwrap(), b"disk");
    assert_eq!(
        grf.read_file("data/texture/유저인터페이스/b.bmp").unwrap(),
        b"disk"
    );
}

#[test]
fn entries_that_normalize_to_one_key_are_reported_and_first_wins() {
    let path = temp_grf_path("collide_case");
    let _c = CleanupFile(path.clone());
    write_raw_grf(&path, &[("a/X.spr", b"first"), ("a/x.spr", b"second")]);
    let grf = GrfArchive::open(&path).unwrap();

    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("a/x.spr").unwrap(), b"first");
    assert_eq!(
        grf.name_collisions(),
        &[(
            "a/x.spr".to_string(),
            "a/X.spr".to_string(),
            "a/x.spr".to_string()
        )]
    );
}

#[test]
fn same_original_name_twice_is_not_a_collision_and_the_last_wins() {
    let path = temp_grf_path("dup_same_name");
    let _c = CleanupFile(path.clone());
    write_raw_grf(&path, &[("data\\a.txt", b"old"), ("data\\a.txt", b"new")]);
    let grf = GrfArchive::open(&path).unwrap();

    assert!(
        grf.name_collisions().is_empty(),
        "{:?}",
        grf.name_collisions()
    );
    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("data/a.txt").unwrap(), b"new");
}

#[test]
fn re_appended_name_wins_over_a_case_variant_and_the_collision_is_kept_once() {
    let path = temp_grf_path("dup_same_then_variant");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[("a/X.spr", b"1"), ("a/x.spr", b"2"), ("a/X.spr", b"3")],
    );
    let grf = GrfArchive::open(&path).unwrap();

    assert_eq!(grf.read_file("a/x.spr").unwrap(), b"3");
    assert_eq!(
        grf.name_collisions(),
        &[(
            "a/x.spr".to_string(),
            "a/X.spr".to_string(),
            "a/x.spr".to_string()
        )]
    );
}

#[test]
fn a_dropped_variant_repeated_is_recorded_only_once() {
    let path = temp_grf_path("dup_variant_twice");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[("a/X.spr", b"1"), ("a/x.spr", b"2"), ("a/x.spr", b"3")],
    );
    let grf = GrfArchive::open(&path).unwrap();

    assert_eq!(grf.read_file("a/x.spr").unwrap(), b"1");
    assert_eq!(
        grf.name_collisions(),
        &[(
            "a/x.spr".to_string(),
            "a/X.spr".to_string(),
            "a/x.spr".to_string()
        )]
    );
}

#[test]
fn korean_name_and_its_translation_collide() {
    let path = temp_grf_path("collide_korean");
    let _c = CleanupFile(path.clone());
    let english = format!("a/{}.spr", rename("검사"));
    write_raw_grf(
        &path,
        &[("a/검사.spr", b"korean"), (english.as_str(), b"english")],
    );
    let grf = GrfArchive::open(&path).unwrap();

    assert_eq!(grf.name_collisions().len(), 1);
    let (key, kept, dropped) = &grf.name_collisions()[0];
    assert_eq!(key, &normalize("a/검사.spr"));
    assert_eq!(kept, "a/검사.spr");
    assert_eq!(dropped, &english);
    assert_eq!(grf.read_file(&english).unwrap(), b"korean");
}

#[test]
fn distinct_names_report_no_collisions() {
    let path = temp_grf_path("collide_none");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[("a/x.spr", b"1"), ("a/y.spr", b"2"), ("a/검사.spr", b"3")],
    );
    let grf = GrfArchive::open(&path).unwrap();

    assert!(grf.name_collisions().is_empty());
    assert!(grf.layered_name_collisions().is_empty());
    assert_eq!(grf.file_count(), 3);
}

#[test]
fn layered_collisions_cover_overlays_but_a_name_in_two_layers_is_not_one() {
    let primary = temp_grf_path("collide_layer_p");
    let overlay = temp_grf_path("collide_layer_o");
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupFile(overlay.clone());
    write_raw_grf(&primary, &[("data/shared.txt", b"p")]);
    write_raw_grf(
        &overlay,
        &[
            ("data/shared.txt", b"o"),
            ("data/Dup.txt", b"1"),
            ("data/dup.txt", b"2"),
        ],
    );

    let grf = GrfArchive::open_layered(
        &[
            primary.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
        ],
        None,
    )
    .unwrap();

    assert!(grf.name_collisions().is_empty());
    assert_eq!(
        grf.layered_name_collisions(),
        vec![(
            "data/dup.txt".to_string(),
            "data/Dup.txt".to_string(),
            "data/dup.txt".to_string()
        )]
    );
}

#[test]
fn add_file_stores_only_normalized_ascii_names() {
    let path = temp_grf_path("add_normalized");
    let _c = CleanupFile(path.clone());
    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("data/몬스터/x.spr", b"body").unwrap();
        grf.add_file("DATA\\Wav\\Y.WAV", b"wav").unwrap();
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    let mut names: Vec<&str> = grf.entry_names().collect();
    names.sort();
    assert!(names.iter().all(|n| n.is_ascii()), "{names:?}");
    assert_eq!(
        names,
        vec![normalize("data/몬스터/x.spr").as_str(), "data/wav/y.wav"]
    );
    assert_eq!(grf.read_file("data/몬스터/x.spr").unwrap(), b"body");
}

#[test]
fn remove_file_accepts_a_korean_name() {
    let path = temp_grf_path("remove_korean");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[("data/몬스터/x.spr", b"1"), ("data/keep.txt", b"2")],
    );

    let mut grf = GrfArchive::open_rw(&path).unwrap();
    assert!(grf.remove_file("DATA\\몬스터\\x.spr").unwrap());
    assert!(!grf.remove_file("data/몬스터/x.spr").unwrap());
    assert_eq!(grf.file_count(), 1);
}

#[test]
fn saving_an_old_archive_rewrites_its_names_as_ascii() {
    let path = temp_grf_path("resave_old");
    let _c = CleanupFile(path.clone());
    write_raw_grf(&path, &[("data/몬스터/x.spr", b"body")]);
    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert!(grf.entry_names().all(|n| n.is_ascii()));
    assert_eq!(grf.read_file("data/몬스터/x.spr").unwrap(), b"body");
}

#[test]
fn entries_with_original_names_pairs_key_and_decoded_name() {
    let path = temp_grf_path("originals");
    let _c = CleanupFile(path.clone());
    write_raw_grf(
        &path,
        &[
            ("data\\texture\\유저인터페이스\\A.BMP", b"1"),
            ("data/plain.txt", b"2"),
        ],
    );
    let grf = GrfArchive::open(&path).unwrap();

    let mut pairs: Vec<(&str, &str)> = grf.entries_with_original_names().collect();
    pairs.sort();
    let mut expected = vec![
        (
            normalize("data/texture/유저인터페이스/a.bmp"),
            "data\\texture\\유저인터페이스\\A.BMP".to_string(),
        ),
        ("data/plain.txt".to_string(), "data/plain.txt".to_string()),
    ];
    expected.sort();
    let expected: Vec<(&str, &str)> = expected
        .iter()
        .map(|(k, o)| (k.as_str(), o.as_str()))
        .collect();
    assert_eq!(pairs, expected);
}

#[test]
fn korean_overlay_entry_is_shadowed_by_the_english_primary_entry() {
    let primary = temp_grf_path("shadow_p");
    let overlay = temp_grf_path("shadow_o");
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupFile(overlay.clone());
    let english = format!("data/{}.txt", rename("검사"));
    write_raw_grf(&primary, &[(english.as_str(), b"primary")]);
    write_raw_grf(&overlay, &[("data/검사.txt", b"overlay")]);

    let grf = GrfArchive::open_layered(
        &[
            primary.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
        ],
        None,
    )
    .unwrap();

    assert_eq!(grf.read_file("data/검사.txt").unwrap(), b"primary");
    assert_eq!(grf.layered_file_list().len(), 1);
}

#[test]
fn res_name_table_alias_to_a_korean_name_still_resolves() {
    let path = temp_grf_path("alias_korean");
    let _c = CleanupFile(path.clone());
    // The table is EUC-KR text, like the real one.
    let (table, _, _) = encoding_rs::EUC_KR
        .encode("유저인터페이스\\map\\pvp_n_2-2.bmp#유저인터페이스\\map\\job_hunter.bmp#\n");
    write_raw_grf(
        &path,
        &[
            ("data/resnametable.txt", &table),
            ("data/texture/유저인터페이스/map/job_hunter.bmp", b"minimap"),
        ],
    );
    assert_eq!(
        ragnarok_resources::table::RES_NAME,
        "data/resnametable.txt",
        "test archive must use the registry's table name"
    );
    let grf = GrfArchive::open(&path).unwrap();

    let alias = "data/texture/유저인터페이스/map/pvp_n_2-2.bmp";
    assert!(grf.file_exists(alias));
    assert_eq!(grf.read_file(alias).unwrap(), b"minimap");
}

#[test]
fn empty_and_unknown_names_are_not_found() {
    let path = temp_grf_path("not_found");
    let _c = CleanupFile(path.clone());
    write_raw_grf(&path, &[("data/a.txt", b"1")]);
    let grf = GrfArchive::open(&path).unwrap();

    assert!(grf.read_file("").is_err());
    assert!(!grf.file_exists(""));
    assert!(grf.read_file("data/없는파일.txt").is_err());
}

#[test]
fn layered_entries_with_original_names_resolves_each_key_once() {
    let primary = temp_grf_path("layered_orig_p");
    let overlay = temp_grf_path("layered_orig_o");
    let _c1 = CleanupFile(primary.clone());
    let _c2 = CleanupFile(overlay.clone());
    let english = format!("data/{}.txt", rename("검사"));
    write_raw_grf(&primary, &[("data/B.txt", b"1"), (english.as_str(), b"2")]);
    write_raw_grf(
        &overlay,
        &[
            ("DATA\\b.TXT", b"3"),
            ("data/검사.txt", b"4"),
            ("data/c.txt", b"5"),
        ],
    );
    let grf = GrfArchive::open_layered(
        &[
            primary.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
        ],
        None,
    )
    .unwrap();

    let pairs = grf.layered_entries_with_original_names();
    let mut expected = vec![
        ("data/b.txt".to_string(), "data/B.txt".to_string()),
        (normalize("data/검사.txt"), english.clone()),
        ("data/c.txt".to_string(), "data/c.txt".to_string()),
    ];
    expected.sort();
    let expected: Vec<(&str, &str)> = expected
        .iter()
        .map(|(k, o)| (k.as_str(), o.as_str()))
        .collect();
    assert_eq!(pairs, expected);
}
