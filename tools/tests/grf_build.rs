//! `grf-build` end to end: library function and the binary's exit codes.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use ragnarok_asset_names::{normalize, rename};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_tools::grf_build::{self, BuildError, Options};

/// Writes a GRF 0x200 the way the original tools do: names stored exactly as
/// given (EUC-KR, any case, any separator), in the order given.
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

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("grf_build_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn options(sources: &[&Path], out: &Path) -> Options {
    Options {
        sources: sources.iter().map(|p| p.to_path_buf()).collect(),
        out: out.to_path_buf(),
        all: true,
        server: None,
        keep: Vec::new(),
        manifest: None,
        allow_collisions: false,
    }
}

fn file_hash(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_grf-build"))
}

const THREE: &[(&str, &[u8])] = &[
    ("data\\sprite\\몬스터\\Poring.SPR", b"spr-bytes"),
    ("data\\texture\\유저인터페이스\\A.BMP", b"bmp-bytes"),
    ("data/wav/_effect.wav", b"wav-bytes"),
];

#[test]
fn all_copies_every_entry_under_its_english_name() {
    let dir = Scratch::new("all");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, THREE);

    let summary = grf_build::build(&options(&[&source], &out)).unwrap();

    assert_eq!(summary.files, 3);
    assert_eq!(summary.dropped, 0);
    assert_eq!(summary.collisions, 0);
    assert!(summary.out_bytes > 0 && summary.source_bytes > 0);

    let built = GrfArchive::open(&out).unwrap();
    assert_eq!(built.file_count(), 3);
    assert!(built.entry_names().all(|n| n.is_ascii()));
    for (original, data) in THREE {
        let english = rename(original);
        assert!(built.entry_names().any(|n| n == english), "{english}");
        assert_eq!(&built.read_file(&english).unwrap(), data, "{original}");
        assert_eq!(&built.read_file(original).unwrap(), data, "{original}");
    }
}

#[test]
fn manifest_lists_original_and_new_sorted_by_new() {
    let dir = Scratch::new("manifest");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, THREE);

    let summary = grf_build::build(&options(&[&source], &out)).unwrap();

    assert_eq!(summary.manifest, dir.path("data.grf.manifest.tsv"));
    let text = std::fs::read_to_string(&summary.manifest).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");

    let mut expected: Vec<(String, String)> = THREE
        .iter()
        .map(|(original, _)| (normalize(original), original.to_string()))
        .collect();
    expected.sort();
    for (line, (new, original)) in lines.iter().zip(&expected) {
        assert_eq!(*line, format!("{original}\t{new}"));
    }
}

#[test]
fn manifest_path_can_be_chosen() {
    let dir = Scratch::new("manifest_path");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    let manifest = dir.path("names.tsv");
    write_raw_grf(&source, THREE);

    let mut opts = options(&[&source], &out);
    opts.manifest = Some(manifest.clone());
    let summary = grf_build::build(&opts).unwrap();

    assert_eq!(summary.manifest, manifest);
    assert!(manifest.is_file());
    assert!(!dir.path("data.grf.manifest.tsv").exists());
}

#[test]
fn default_manifest_path_appends_to_the_file_name() {
    assert_eq!(
        grf_build::default_manifest_path(Path::new("out/data.grf")),
        PathBuf::from("out/data.grf.manifest.tsv")
    );
    assert_eq!(
        grf_build::default_manifest_path(Path::new("data")),
        PathBuf::from("data.manifest.tsv")
    );
}

#[test]
fn source_is_never_modified() {
    let dir = Scratch::new("readonly_source");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, THREE);
    let before = file_hash(&source);

    grf_build::build(&options(&[&source], &out)).unwrap();

    assert_eq!(file_hash(&source), before);
}

#[test]
fn collision_fails_and_writes_nothing() {
    let dir = Scratch::new("collision");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, &[("a/X.spr", b"first"), ("a/x.spr", b"second")]);

    let err = grf_build::build(&options(&[&source], &out)).unwrap_err();

    match err {
        BuildError::Collisions(list) => assert_eq!(
            list,
            vec![(
                "a/x.spr".to_string(),
                "a/X.spr".to_string(),
                "a/x.spr".to_string()
            )]
        ),
        other => panic!("expected collisions, got {other:?}"),
    }
    assert!(!out.exists());
    assert!(!dir.path("data.grf.manifest.tsv").exists());
}

#[test]
fn collision_between_overlay_entries_also_fails() {
    let dir = Scratch::new("collision_layer");
    let primary = dir.path("p.grf");
    let overlay = dir.path("o.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&primary, &[("data/ok.txt", b"1")]);
    write_raw_grf(&overlay, &[("data/D.txt", b"2"), ("data/d.txt", b"3")]);

    let err = grf_build::build(&options(&[&primary, &overlay], &out)).unwrap_err();

    assert!(
        matches!(err, BuildError::Collisions(ref l) if l.len() == 1),
        "{err:?}"
    );
    assert!(!out.exists());
}

#[test]
fn allow_collisions_keeps_the_first_entry() {
    let dir = Scratch::new("allow_collision");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, &[("a/X.spr", b"first"), ("a/x.spr", b"second")]);

    let mut opts = options(&[&source], &out);
    opts.allow_collisions = true;
    let summary = grf_build::build(&opts).unwrap();

    assert_eq!(summary.files, 1);
    assert_eq!(summary.collisions, 1);
    let built = GrfArchive::open(&out).unwrap();
    assert_eq!(built.read_file("a/x.spr").unwrap(), b"first");
}

#[test]
fn out_equal_to_a_source_is_refused_and_the_source_is_untouched() {
    let dir = Scratch::new("same_path");
    let source = dir.path("old.grf");
    write_raw_grf(&source, THREE);
    let before = file_hash(&source);

    let err = grf_build::build(&options(&[&source], &source)).unwrap_err();

    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");
    assert_eq!(file_hash(&source), before);
}

#[test]
fn out_that_reaches_a_source_through_dots_is_refused() {
    let dir = Scratch::new("same_path_dots");
    let source = dir.path("old.grf");
    write_raw_grf(&source, THREE);
    let before = file_hash(&source);
    let sneaky = dir.path("sub/../old.grf");
    std::fs::create_dir_all(dir.path("sub")).unwrap();

    let err = grf_build::build(&options(&[&source], &sneaky)).unwrap_err();

    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");
    assert_eq!(file_hash(&source), before);
}

#[test]
fn existing_out_file_is_not_overwritten() {
    let dir = Scratch::new("existing_out");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, THREE);
    std::fs::write(&out, b"precious").unwrap();

    let err = grf_build::build(&options(&[&source], &out)).unwrap_err();

    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");
    assert_eq!(std::fs::read(&out).unwrap(), b"precious");
}

#[test]
fn missing_or_corrupt_source_fails_without_output() {
    let dir = Scratch::new("bad_source");
    let out = dir.path("data.grf");

    let err = grf_build::build(&options(&[&dir.path("nope.grf")], &out)).unwrap_err();
    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");

    let junk = dir.path("junk.grf");
    std::fs::write(&junk, b"not a grf at all").unwrap();
    let err = grf_build::build(&options(&[&junk], &out)).unwrap_err();
    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");

    assert!(!out.exists());
}

#[test]
fn corrupt_overlay_is_an_error_not_a_silent_skip() {
    // `open_layered` only logs an overlay it cannot open; a build that quietly
    // lacked a source would produce an incomplete data.grf.
    let dir = Scratch::new("bad_overlay");
    let primary = dir.path("p.grf");
    let junk = dir.path("junk.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&primary, THREE);
    std::fs::write(&junk, b"not a grf at all").unwrap();

    let err = grf_build::build(&options(&[&primary, &junk], &out)).unwrap_err();

    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");
    assert!(!out.exists());
}

#[test]
fn no_sources_is_an_error() {
    let dir = Scratch::new("no_sources");
    let err = grf_build::build(&options(&[], &dir.path("data.grf"))).unwrap_err();
    assert!(matches!(err, BuildError::Failed(_)), "{err:?}");
}

#[test]
fn empty_source_builds_an_empty_archive() {
    let dir = Scratch::new("empty");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, &[]);

    let summary = grf_build::build(&options(&[&source], &out)).unwrap();

    assert_eq!(summary.files, 0);
    assert_eq!(GrfArchive::open(&out).unwrap().file_count(), 0);
    assert_eq!(std::fs::read_to_string(&summary.manifest).unwrap(), "");
}

#[test]
fn layered_sources_use_the_first_archive_for_shared_names() {
    let dir = Scratch::new("layers");
    let primary = dir.path("p.grf");
    let overlay = dir.path("o.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&primary, &[("data/shared.txt", b"primary")]);
    write_raw_grf(
        &overlay,
        &[("data/shared.txt", b"overlay"), ("data/only.txt", b"only")],
    );

    let summary = grf_build::build(&options(&[&primary, &overlay], &out)).unwrap();

    assert_eq!(summary.files, 2);
    let built = GrfArchive::open(&out).unwrap();
    assert_eq!(built.read_file("data/shared.txt").unwrap(), b"primary");
    assert_eq!(built.read_file("data/only.txt").unwrap(), b"only");
}

#[test]
fn default_mode_drops_unreferenced_textures_and_keep_protects_them() {
    let dir = Scratch::new("prune");
    let source = dir.path("old.grf");
    let cursors = ragnarok_resources::sprite::CURSORS_SPR;
    let animation = ragnarok_resources::sprite::CURSORS_ACT;
    write_raw_grf(
        &source,
        &[
            (cursors, b"spr"),
            (animation, b"act"),
            ("data/texture/nobody_wants_me.bmp", b"bmp"),
            ("data/texture/kept_by_flag.bmp", b"bmp"),
        ],
    );

    let mut opts = options(&[&source], &dir.path("pruned.grf"));
    opts.all = false;
    opts.keep = vec![normalize("data/texture/kept_by_flag")];
    let summary = grf_build::build(&opts).unwrap();

    assert_eq!(summary.dropped, 1);
    assert_eq!(summary.files, 3);
    let built = GrfArchive::open(&dir.path("pruned.grf")).unwrap();
    assert!(built.file_exists(cursors));
    assert!(built.file_exists(animation));
    assert!(built.file_exists("data/texture/kept_by_flag.bmp"));
    assert!(!built.file_exists("data/texture/nobody_wants_me.bmp"));

    let mut opts = options(&[&source], &dir.path("everything.grf"));
    opts.all = true;
    let summary = grf_build::build(&opts).unwrap();
    assert_eq!(summary.files, 4);
}

#[test]
fn ui_artwork_with_a_korean_source_name_is_kept_by_default() {
    // Interface art cannot be enumerated, so the default mode must not drop it
    // even though its name is English once normalized (and no registry entry
    // names this file, so only the category keeps it).
    let dir = Scratch::new("ui_kept");
    let source = dir.path("old.grf");
    write_raw_grf(
        &source,
        &[(
            "data\\texture\\유저인터페이스\\zz_unlisted_art.bmp",
            b"ui",
        )],
    );

    let mut opts = options(&[&source], &dir.path("out.grf"));
    opts.all = false;
    let summary = grf_build::build(&opts).unwrap();

    assert_eq!(summary.files, 1);
    assert_eq!(summary.dropped, 0);
}

#[test]
fn binary_builds_and_reports_a_summary() {
    let dir = Scratch::new("bin_ok");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, THREE);

    let result = bin()
        .arg(&source)
        .arg("--out")
        .arg(&out)
        .arg("--all")
        .output()
        .unwrap();

    assert!(result.status.success(), "{result:?}");
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("3 files"), "{stdout}");
    assert_eq!(GrfArchive::open(&out).unwrap().file_count(), 3);
}

#[test]
fn binary_exits_2_on_collision_and_writes_nothing() {
    let dir = Scratch::new("bin_collision");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf(&source, &[("a/X.spr", b"1"), ("a/x.spr", b"2")]);

    let result = bin()
        .arg(&source)
        .arg("--out")
        .arg(&out)
        .arg("--all")
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(2), "{result:?}");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("a/X.spr") && stderr.contains("a/x.spr"),
        "{stderr}"
    );
    assert!(!out.exists());
    assert!(!dir.path("data.grf.manifest.tsv").exists());

    let allowed = bin()
        .arg(&source)
        .arg("--out")
        .arg(&out)
        .arg("--all")
        .arg("--allow-collisions")
        .output()
        .unwrap();
    assert!(allowed.status.success(), "{allowed:?}");
}

#[test]
fn binary_exits_1_when_out_is_a_source() {
    let dir = Scratch::new("bin_same");
    let source = dir.path("old.grf");
    write_raw_grf(&source, THREE);
    let before = file_hash(&source);

    let result = bin()
        .arg(&source)
        .arg("--out")
        .arg(&source)
        .arg("--all")
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(file_hash(&source), before);
}

#[test]
fn binary_exits_64_on_usage_errors() {
    let dir = Scratch::new("bin_usage");
    let source = dir.path("old.grf");
    write_raw_grf(&source, THREE);

    for args in [
        vec![],
        vec!["--out".to_string(), "x.grf".to_string()],
        vec![source.to_string_lossy().into_owned()],
        vec![source.to_string_lossy().into_owned(), "--out".to_string()],
        vec![
            source.to_string_lossy().into_owned(),
            "--out".to_string(),
            "x.grf".to_string(),
            "--bogus".to_string(),
        ],
    ] {
        let result = bin().args(&args).output().unwrap();
        assert_eq!(result.status.code(), Some(64), "{args:?}: {result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("usage:"),
            "{args:?}"
        );
    }
}
