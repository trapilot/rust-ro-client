use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::FormatError;
use crate::mixcrypt;
use crate::res_name_table::ResNameTable;

const HEADER_SIZE: usize = 46;
const FILE_OFFSET: usize = 7;

struct GrfEntry {
    compressed_size: u32,
    compressed_size_aligned: u32,
    uncompressed_size: u32,
    flags: u8,
    offset: u32,
}

pub struct GrfFileInfo {
    pub name: String,
    pub compressed_size: u32,
    pub compressed_size_aligned: u32,
    pub uncompressed_size: u32,
    pub flags: u8,
}

pub struct GrfArchive {
    entries: HashMap<String, GrfEntry>,
    /// Snake form -> real entry key, for entries whose name is not snake_case.
    /// Built once when a read-only archive is opened; empty for writable ones.
    snake_index: HashMap<String, String>,
    file: Mutex<File>,
    path: PathBuf,
    file_data_end_offset: u64,
    writable: bool,
    data_dir: Option<DataDirIndex>,
    overlays: Vec<GrfArchive>,
    overrides: HashMap<String, Vec<u8>>,
    res_name_table: OnceLock<ResNameTable>,
}

/// Extracted files on disk, keyed by their normalized path relative to the
/// directory. A resource name resolves against this by dropping its leading
/// `data/` component, so a directory holding `sprite/foo.spr` overrides the
/// archive entry `data/sprite/foo.spr`.
struct DataDirIndex {
    files: HashMap<String, PathBuf>,
    snake_index: HashMap<String, String>,
}

impl DataDirIndex {
    fn build(dir: &Path) -> DataDirIndex {
        let mut files = HashMap::new();
        index_dir(dir, dir, &mut files);
        let snake_index = build_snake_index(files.keys());
        DataDirIndex { files, snake_index }
    }

    fn lookup(&self, name: &str) -> Option<&PathBuf> {
        lookup_snake(&self.files, &self.snake_index, &data_dir_key(name))
    }
}

/// Rewrites a normalized path (lowercase, `/` separators) to snake_case, one
/// segment at a time.
///
/// The extension (after the last `.`, unless the dot starts the segment) is
/// kept. In the stem, every inner run of characters other than ASCII
/// alphanumerics, `_` and non-ASCII characters becomes a single `_`, and such
/// characters at either end are dropped. Existing underscores are never
/// touched. A stem that would end up empty is kept as it was.
pub fn snake_path(name: &str) -> String {
    name.split('/')
        .map(snake_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn is_kept_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || !c.is_ascii()
}

fn snake_segment(segment: &str) -> String {
    let (stem, ext) = match segment.rfind('.') {
        Some(dot) if dot > 0 => segment.split_at(dot),
        _ => (segment, ""),
    };
    let mut out = String::with_capacity(segment.len());
    let mut pending_separator = false;
    for c in stem.chars() {
        if is_kept_char(c) {
            if pending_separator && !out.is_empty() {
                out.push('_');
            }
            pending_separator = false;
            out.push(c);
        } else {
            pending_separator = true;
        }
    }
    if out.is_empty() {
        out.push_str(stem);
    }
    out.push_str(ext);
    out
}

/// Maps the snake form of every non-snake key to that key. A snake form is
/// left out when it is itself a key, or when several keys share it, so a
/// snake request never resolves to a guess.
fn build_snake_index<'a>(keys: impl Iterator<Item = &'a String>) -> HashMap<String, String> {
    let keys: Vec<&String> = keys.collect();
    let existing: HashSet<&str> = keys.iter().map(|k| k.as_str()).collect();
    let mut index: HashMap<String, Option<String>> = HashMap::new();
    for key in keys {
        let snake = snake_path(key);
        if snake == *key || existing.contains(snake.as_str()) {
            continue;
        }
        // A second key with the same snake form makes the entry ambiguous.
        index
            .entry(snake)
            .and_modify(|slot| *slot = None)
            .or_insert_with(|| Some(key.clone()));
    }
    index
        .into_iter()
        .filter_map(|(snake, key)| key.map(|k| (snake, k)))
        .collect()
}

/// Looks up a normalized key: exact match, then its snake form as an exact
/// key, then the snake index.
fn lookup_snake<'a, V>(
    map: &'a HashMap<String, V>,
    snake_index: &HashMap<String, String>,
    key: &str,
) -> Option<&'a V> {
    if let Some(value) = map.get(key) {
        return Some(value);
    }
    let snake = snake_path(key);
    if let Some(value) = map.get(&snake) {
        return Some(value);
    }
    snake_index.get(&snake).and_then(|real| map.get(real))
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase().replace('\\', "/")
}

fn data_dir_key(name: &str) -> String {
    let normalized = normalize_name(name);
    normalized
        .strip_prefix(ragnarok_resources::dir::DATA)
        .unwrap_or(&normalized)
        .to_string()
}

fn index_dir(root: &Path, dir: &Path, out: &mut HashMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            index_dir(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let key = normalize_name(&relative.to_string_lossy());
            out.insert(key, path);
        }
    }
}

fn parse_grf(file: &mut File) -> Result<(HashMap<String, GrfEntry>, u64), FormatError> {
    let mut header_buf = [0u8; HEADER_SIZE];
    file.read_exact(&mut header_buf)?;

    if &header_buf[..16] != b"Master of Magic\0" {
        return Err(FormatError::InvalidMagic);
    }

    let file_table_offset = u32::from_le_bytes(header_buf[30..34].try_into().unwrap());
    let reserved_files = u32::from_le_bytes(header_buf[34..38].try_into().unwrap());
    let file_count = u32::from_le_bytes(header_buf[38..42].try_into().unwrap());
    let version = u32::from_le_bytes(header_buf[42..46].try_into().unwrap());

    if !(0x100..=0x200).contains(&version) {
        return Err(FormatError::UnsupportedVersion(
            (version >> 8) as u8,
            version as u8,
        ));
    }

    let file_data_end_offset = HEADER_SIZE as u64 + file_table_offset as u64;
    let actual_file_count = (file_count - reserved_files) as usize - FILE_OFFSET;

    file.seek(SeekFrom::Start(file_data_end_offset))?;

    let entries = if version == 0x200 {
        parse_v2_entries(file, actual_file_count)?
    } else {
        parse_v1_entries(file, actual_file_count)?
    };

    Ok((entries, file_data_end_offset))
}

impl GrfArchive {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        let mut file = File::open(path).map_err(FormatError::Io)?;
        let (entries, file_data_end_offset) = parse_grf(&mut file)?;
        let snake_index = build_snake_index(entries.keys());

        Ok(GrfArchive {
            entries,
            snake_index,
            file: Mutex::new(file),
            path: path.to_path_buf(),
            file_data_end_offset,
            writable: false,
            data_dir: None,
            overlays: Vec::new(),
            overrides: HashMap::new(),
            res_name_table: OnceLock::new(),
        })
    }

    /// Open a priority-ordered stack of archives plus an optional directory of
    /// extracted files. `grf_paths[0]` is the primary archive; later paths only
    /// provide files the earlier ones lack. `data_dir`, when present, overrides
    /// every archive.
    pub fn open_layered(
        grf_paths: &[String],
        data_dir: Option<&Path>,
    ) -> Result<Self, FormatError> {
        let (first, rest) = grf_paths
            .split_first()
            .ok_or(FormatError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "grf_paths is empty",
            )))?;

        let mut archive = GrfArchive::open(Path::new(first))?;

        for path in rest {
            match GrfArchive::open(Path::new(path)) {
                Ok(overlay) => archive.overlays.push(overlay),
                Err(e) => tracing::error!("Failed to open overlay GRF {path}: {e}"),
            }
        }

        if let Some(dir) = data_dir {
            if dir.is_dir() {
                archive.data_dir = Some(DataDirIndex::build(dir));
            } else {
                tracing::warn!("data_dir {} is not a directory, ignoring", dir.display());
            }
        }

        Ok(archive)
    }

    pub fn open_rw(path: &Path) -> Result<Self, FormatError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(FormatError::Io)?;
        let (entries, file_data_end_offset) = parse_grf(&mut file)?;

        Ok(GrfArchive {
            entries,
            snake_index: HashMap::new(),
            file: Mutex::new(file),
            path: path.to_path_buf(),
            file_data_end_offset,
            writable: true,
            data_dir: None,
            overlays: Vec::new(),
            overrides: HashMap::new(),
            res_name_table: OnceLock::new(),
        })
    }

    pub fn create(path: &Path) -> Result<Self, FormatError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(FormatError::Io)?;

        let mut header = [0u8; HEADER_SIZE];
        header[..16].copy_from_slice(b"Master of Magic\0");
        header[30..34].copy_from_slice(&0u32.to_le_bytes());
        header[34..38].copy_from_slice(&0u32.to_le_bytes());
        header[38..42].copy_from_slice(&(FILE_OFFSET as u32).to_le_bytes());
        header[42..46].copy_from_slice(&0x200u32.to_le_bytes());
        file.write_all(&header)?;

        let empty_table = Vec::new();
        let compressed = zlib_compress(&empty_table)?;
        file.write_all(&(compressed.len() as u32).to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(&compressed)?;
        file.flush()?;

        Ok(GrfArchive {
            entries: HashMap::new(),
            snake_index: HashMap::new(),
            file: Mutex::new(file),
            path: path.to_path_buf(),
            file_data_end_offset: HEADER_SIZE as u64,
            writable: true,
            data_dir: None,
            overlays: Vec::new(),
            overrides: HashMap::new(),
            res_name_table: OnceLock::new(),
        })
    }

    pub fn is_writable(&self) -> bool {
        self.writable
    }

    /// Reads `name`, falling back to the `resnametable.txt` alias when nothing
    /// carries that name. Maps such as `pvp_n_2-2` ship no geometry of their
    /// own and exist only as aliases.
    pub fn read_file(&self, name: &str) -> Result<Vec<u8>, FormatError> {
        match self.read_file_direct(name) {
            Ok(data) => Ok(data),
            Err(e) => {
                let Some(alias) = self.alias_table().resolve(name) else {
                    return Err(e);
                };
                tracing::debug!("Resource {name} redirected to {alias}");
                self.read_file_direct(&alias)
            }
        }
    }

    /// Every layer contributes its own table; the higher-priority layer wins a
    /// key both define.
    fn alias_table(&self) -> &ResNameTable {
        self.res_name_table.get_or_init(|| {
            let mut table = ResNameTable::default();
            let path = ragnarok_resources::table::RES_NAME;
            if let Some(disk) = self.data_dir.as_ref().and_then(|d| d.lookup(path))
                && let Ok(data) = std::fs::read(disk)
            {
                table.extend_from(ResNameTable::parse(&data));
            }
            for layer in std::iter::once(self).chain(self.overlays.iter()) {
                if let Ok(data) = layer.read_archive_entry(path) {
                    table.extend_from(ResNameTable::parse(&data));
                }
            }
            tracing::info!("Loaded resource name table ({} entries)", table.len());
            table
        })
    }

    /// Replaces what `read_file` returns for `name`, ahead of `data_dir` and
    /// every layer. `grf-merge` installs the tables it merged across layers
    /// here so the reachability walk sees the merged rows.
    pub fn set_override(&mut self, name: &str, data: Vec<u8>) {
        self.overrides.insert(normalize_name(name), data);
    }

    fn read_file_direct(&self, name: &str) -> Result<Vec<u8>, FormatError> {
        if !self.overrides.is_empty()
            && let Some(data) = self.overrides.get(&normalize_name(name))
        {
            return Ok(data.clone());
        }

        if let Some(path) = self.data_dir.as_ref().and_then(|d| d.lookup(name)) {
            return std::fs::read(path).map_err(FormatError::Io);
        }

        if self.find_entry(name).is_none() {
            for overlay in &self.overlays {
                if overlay.find_entry(name).is_some() {
                    return overlay.read_archive_entry(name);
                }
            }
        }
        self.read_archive_entry(name)
    }

    /// This archive's own entry for `name`, exact or by snake form.
    fn find_entry(&self, name: &str) -> Option<&GrfEntry> {
        lookup_snake(&self.entries, &self.snake_index, &normalize_name(name))
    }

    /// This archive's own entries, ignoring `data_dir` and overlays.
    fn read_archive_entry(&self, name: &str) -> Result<Vec<u8>, FormatError> {
        let Some(entry) = self.find_entry(name) else {
            return Err(FormatError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file not found in archive: {name}"),
            )));
        };

        let position = entry.offset as u64 + HEADER_SIZE as u64;
        let mut compressed = vec![0u8; entry.compressed_size_aligned as usize];

        {
            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(position))?;
            file.read_exact(&mut compressed)?;
        }

        mixcrypt::decrypt_file(entry.flags, entry.compressed_size, &mut compressed);

        let mut decompressed = Vec::with_capacity(entry.uncompressed_size as usize);
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| FormatError::DecompressionFailed(e.to_string()))?;

        Ok(decompressed)
    }

    pub fn add_file(&mut self, name: &str, data: &[u8]) -> Result<(), FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }

        let name_lower = normalize_name(name);
        let compressed = zlib_compress(data)?;
        let compressed_len = compressed.len() as u32;

        let file = self.file.get_mut().unwrap();
        file.seek(SeekFrom::Start(self.file_data_end_offset))?;
        file.write_all(&compressed)?;

        let entry = GrfEntry {
            compressed_size: compressed_len,
            compressed_size_aligned: compressed_len,
            uncompressed_size: data.len() as u32,
            flags: 0x01,
            offset: (self.file_data_end_offset - HEADER_SIZE as u64) as u32,
        };

        self.file_data_end_offset += compressed_len as u64;
        self.entries.insert(name_lower, entry);
        Ok(())
    }

    pub fn remove_file(&mut self, name: &str) -> Result<bool, FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }
        let name_lower = normalize_name(name);
        Ok(self.entries.remove(&name_lower).is_some())
    }

    pub fn save(&mut self) -> Result<(), FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }

        let table_raw = build_file_table(&self.entries);
        let table_compressed = zlib_compress(&table_raw)?;

        let file = self.file.get_mut().unwrap();
        file.seek(SeekFrom::Start(self.file_data_end_offset))?;
        file.write_all(&(table_compressed.len() as u32).to_le_bytes())?;
        file.write_all(&(table_raw.len() as u32).to_le_bytes())?;
        file.write_all(&table_compressed)?;

        let end_pos = self.file_data_end_offset + 8 + table_compressed.len() as u64;
        file.set_len(end_pos)?;

        let file_table_offset = (self.file_data_end_offset - HEADER_SIZE as u64) as u32;
        let file_count = (self.entries.len() + FILE_OFFSET) as u32;

        file.seek(SeekFrom::Start(30))?;
        file.write_all(&file_table_offset.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(&file_count.to_le_bytes())?;
        file.flush()?;

        Ok(())
    }

    pub fn repack(&mut self) -> Result<(), FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }

        let mut blobs: Vec<(String, GrfEntry, Vec<u8>)> = Vec::with_capacity(self.entries.len());
        {
            let file = self.file.get_mut().unwrap();
            for (name, entry) in &self.entries {
                let position = entry.offset as u64 + HEADER_SIZE as u64;
                let mut blob = vec![0u8; entry.compressed_size_aligned as usize];
                file.seek(SeekFrom::Start(position))?;
                file.read_exact(&mut blob)?;
                blobs.push((
                    name.clone(),
                    GrfEntry {
                        compressed_size: entry.compressed_size,
                        compressed_size_aligned: entry.compressed_size_aligned,
                        uncompressed_size: entry.uncompressed_size,
                        flags: entry.flags,
                        offset: 0,
                    },
                    blob,
                ));
            }
        }

        let file = self.file.get_mut().unwrap();
        let mut write_pos = HEADER_SIZE as u64;
        self.entries.clear();

        for (name, mut entry, blob) in blobs {
            file.seek(SeekFrom::Start(write_pos))?;
            file.write_all(&blob)?;
            entry.offset = (write_pos - HEADER_SIZE as u64) as u32;
            write_pos += blob.len() as u64;
            self.entries.insert(name, entry);
        }

        self.file_data_end_offset = write_pos;
        self.save()
    }

    pub fn file_exists(&self, name: &str) -> bool {
        self.entry_exists(name)
            || self
                .alias_table()
                .resolve(name)
                .is_some_and(|alias| self.entry_exists(&alias))
    }

    fn entry_exists(&self, name: &str) -> bool {
        if self
            .data_dir
            .as_ref()
            .and_then(|d| d.lookup(name))
            .is_some()
        {
            return true;
        }
        self.overrides.contains_key(&normalize_name(name))
            || self.find_entry(name).is_some()
            || self.overlays.iter().any(|o| o.find_entry(name).is_some())
    }

    /// Counts only the primary archive; overlays and `data_dir` are not
    /// included. The same holds for the other listing methods below.
    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn find_first_with_extension(&self, extension: &str) -> Option<&str> {
        let ext = extension.to_lowercase();
        self.entries
            .keys()
            .find(|name| name.ends_with(&ext))
            .map(|s| s.as_str())
    }

    pub fn files_with_extension(&self, extension: &str) -> Vec<&str> {
        let ext = extension.to_lowercase();
        self.entries
            .keys()
            .filter(|name| name.ends_with(&ext))
            .map(|s| s.as_str())
            .collect()
    }

    pub fn file_list(&self) -> Vec<GrfFileInfo> {
        let mut list: Vec<GrfFileInfo> = self
            .entries
            .iter()
            .map(|(name, entry)| GrfFileInfo {
                name: name.clone(),
                compressed_size: entry.compressed_size,
                compressed_size_aligned: entry.compressed_size_aligned,
                uncompressed_size: entry.uncompressed_size,
                flags: entry.flags,
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Every name in the layer stack, listed once and resolved the way
    /// `read_file` resolves it: a name several archives carry is reported with
    /// the sizes of the earliest one that has it. Ordered by layer, then name.
    /// `data_dir` and overrides are not listed.
    pub fn layered_file_list(&self) -> Vec<GrfFileInfo> {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut list = Vec::new();
        for layer in std::iter::once(self).chain(self.overlays.iter()) {
            let mut names: Vec<&String> = layer.entries.keys().collect();
            names.sort();
            for name in names {
                if !seen.insert(name.as_str()) {
                    continue;
                }
                let entry = &layer.entries[name];
                list.push(GrfFileInfo {
                    name: name.clone(),
                    compressed_size: entry.compressed_size,
                    compressed_size_aligned: entry.compressed_size_aligned,
                    uncompressed_size: entry.uncompressed_size,
                    flags: entry.flags,
                });
            }
        }
        list
    }

    pub fn file_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn zlib_compress(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish().map_err(FormatError::Io)
}

fn build_file_table(entries: &HashMap<String, GrfEntry>) -> Vec<u8> {
    let mut table = Vec::new();
    for (name, entry) in entries {
        let backslash_name = name.replace('/', "\\");
        let (encoded, _, _) = encoding_rs::EUC_KR.encode(&backslash_name);
        table.extend_from_slice(&encoded);
        table.push(0);
        table.extend_from_slice(&entry.compressed_size.to_le_bytes());
        table.extend_from_slice(&entry.compressed_size_aligned.to_le_bytes());
        table.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        table.push(entry.flags);
        table.extend_from_slice(&entry.offset.to_le_bytes());
    }
    table
}

fn read_u32_le(file: &mut File) -> Result<u32, FormatError> {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn parse_v2_entries(
    file: &mut File,
    file_count: usize,
) -> Result<HashMap<String, GrfEntry>, FormatError> {
    let table_compressed_size = read_u32_le(file)?;
    let table_uncompressed_size = read_u32_le(file)?;

    let mut compressed_table = vec![0u8; table_compressed_size as usize];
    file.read_exact(&mut compressed_table)?;

    let mut table = Vec::with_capacity(table_uncompressed_size as usize);
    let mut decoder = ZlibDecoder::new(compressed_table.as_slice());
    decoder
        .read_to_end(&mut table)
        .map_err(|e| FormatError::DecompressionFailed(e.to_string()))?;

    let mut entries = HashMap::with_capacity(file_count);
    let mut pos = 0;
    for _ in 0..file_count {
        let name_end = table[pos..]
            .iter()
            .position(|&b| b == 0)
            .ok_or(FormatError::UnexpectedEof)?;
        let name_bytes = &table[pos..pos + name_end];
        let (name_decoded, _, _) = encoding_rs::EUC_KR.decode(name_bytes);
        let name = name_decoded.into_owned().to_lowercase().replace('\\', "/");
        pos += name_end + 1;

        if pos + 17 > table.len() {
            return Err(FormatError::UnexpectedEof);
        }

        let compressed_size = u32::from_le_bytes(table[pos..pos + 4].try_into().unwrap());
        let compressed_size_aligned =
            u32::from_le_bytes(table[pos + 4..pos + 8].try_into().unwrap());
        let uncompressed_size = u32::from_le_bytes(table[pos + 8..pos + 12].try_into().unwrap());
        let flags = table[pos + 12];
        let offset = u32::from_le_bytes(table[pos + 13..pos + 17].try_into().unwrap());
        pos += 17;

        entries.insert(
            name,
            GrfEntry {
                compressed_size,
                compressed_size_aligned,
                uncompressed_size,
                flags,
                offset,
            },
        );
    }
    Ok(entries)
}

fn parse_v1_entries(
    file: &mut File,
    file_count: usize,
) -> Result<HashMap<String, GrfEntry>, FormatError> {
    let mut table = Vec::new();
    file.read_to_end(&mut table)?;

    let mut entries = HashMap::with_capacity(file_count);
    let mut ofs = 0;
    for _ in 0..file_count {
        if ofs + 6 > table.len() {
            return Err(FormatError::UnexpectedEof);
        }
        let entry_len = u32::from_le_bytes(table[ofs..ofs + 4].try_into().unwrap()) as usize;
        let name_len = (entry_len as u8).wrapping_sub(6) as usize;
        if ofs + 6 + name_len > table.len() {
            return Err(FormatError::UnexpectedEof);
        }
        let decrypt_len = (name_len + 7) & !7;
        let mut name_buf = vec![0u8; decrypt_len];
        let copy_len = name_len.min(table.len() - (ofs + 6));
        name_buf[..copy_len].copy_from_slice(&table[ofs + 6..ofs + 6 + copy_len]);
        mixcrypt::decrypt_filename(&mut name_buf);

        let name_end = name_buf.iter().position(|&b| b == 0).unwrap_or(name_len);
        let (name_decoded, _, _) = encoding_rs::EUC_KR.decode(&name_buf[..name_end]);
        let name = name_decoded.into_owned().to_lowercase().replace('\\', "/");

        let ofs2 = ofs + entry_len + 4;
        if ofs2 + 17 > table.len() {
            return Err(FormatError::UnexpectedEof);
        }

        let raw_compressed = u32::from_le_bytes(table[ofs2..ofs2 + 4].try_into().unwrap());
        let raw_aligned = u32::from_le_bytes(table[ofs2 + 4..ofs2 + 8].try_into().unwrap());
        let uncompressed_size = u32::from_le_bytes(table[ofs2 + 8..ofs2 + 12].try_into().unwrap());
        let entry_type = table[ofs2 + 12];
        let offset = u32::from_le_bytes(table[ofs2 + 13..ofs2 + 17].try_into().unwrap());

        let compressed_size = raw_compressed
            .wrapping_sub(uncompressed_size)
            .wrapping_sub(715);
        let compressed_size_aligned = raw_aligned.wrapping_sub(37579);
        let flags = if entry_type & 0x01 != 0 {
            encryption_flags_for_extension(&name) | 0x01
        } else {
            entry_type
        };

        ofs = ofs2 + 17;

        if entry_type & 0x01 == 0 {
            continue;
        }

        entries.insert(
            name,
            GrfEntry {
                compressed_size,
                compressed_size_aligned,
                uncompressed_size,
                flags,
                offset,
            },
        );
    }
    Ok(entries)
}

fn encryption_flags_for_extension(filename: &str) -> u8 {
    match filename.rsplit('.').next() {
        Some("gnd" | "gat" | "act" | "str") => mixcrypt::GRF_FLAG_HEADER_DES_CRYPT,
        _ => mixcrypt::GRF_FLAG_FULL_MIX_CRYPT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("grf_snake_{}_{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn open_with(tag: &str, files: &[(&str, &[u8])]) -> GrfArchive {
        let dir = temp_path(tag);
        let path = dir.join("test.grf");
        let mut grf = GrfArchive::create(&path).unwrap();
        for (name, data) in files {
            grf.add_file(name, data).unwrap();
        }
        grf.save().unwrap();
        drop(grf);
        GrfArchive::open(&path).unwrap()
    }

    #[test]
    fn snake_path_vectors() {
        let vectors = [
            ("1@cata.gat", "1_cata.gat"),
            ("lua files", "lua_files"),
            ("cru_grand cross.wav", "cru_grand_cross.wav"),
            ("acidus_.spr", "acidus_.spr"),
            ("acidus__attack.wav", "acidus__attack.wav"),
            ("_attack_bow.wav", "_attack_bow.wav"),
            ("blast_mine##clock.bmp", "blast_mine_clock.bmp"),
            ("c-big(side).bmp", "c_big_side.bmp"),
            ("mo-castle-w3+.bmp", "mo_castle_w3.bmp"),
            ("moc_wall08-.bmp", "moc_wall08.bmp"),
            ("wl_soulexpansion .bmp", "wl_soulexpansion.bmp"),
            ("skilldesctable.txt.bak", "skilldesctable_txt.bak"),
            ("##.bmp", "##.bmp"),
            (".hidden", "hidden"),
            ("여-낙엽.act", "여_낙엽.act"),
            ("pvp_n_2-2", "pvp_n_2_2"),
        ];
        for (input, expected) in vectors {
            assert_eq!(snake_path(input), expected, "snake_path({input:?})");
        }
    }

    #[test]
    fn snake_path_applies_per_segment() {
        assert_eq!(
            snake_path("data/lua files/a b.lub"),
            "data/lua_files/a_b.lub"
        );
    }

    #[test]
    fn archive_resolves_snake_request_to_non_snake_entry() {
        let grf = open_with("archive_fwd", &[(r"data\lua files\a.lub", b"one")]);
        assert_eq!(grf.read_file("data/lua_files/a.lub").unwrap(), b"one");
        assert_eq!(grf.read_file("data/lua files/a.lub").unwrap(), b"one");
        assert!(grf.file_exists(r"DATA\Lua_Files\A.lub"));
        assert_eq!(grf.file_names(), vec!["data/lua files/a.lub"]);
    }

    #[test]
    fn archive_resolves_non_snake_request_to_snake_entry() {
        let grf = open_with("archive_rev", &[("data/lua_files/a.lub", b"two")]);
        assert_eq!(grf.read_file("data/lua files/a.lub").unwrap(), b"two");
        assert!(grf.file_exists("data/lua files/a.lub"));
        assert_eq!(grf.file_names(), vec!["data/lua_files/a.lub"]);
    }

    #[test]
    fn archive_ambiguous_snake_form_is_not_resolved() {
        let grf = open_with(
            "archive_amb",
            &[("data/a b.txt", b"space"), ("data/a-b.txt", b"hyphen")],
        );
        assert_eq!(grf.read_file("data/a b.txt").unwrap(), b"space");
        assert_eq!(grf.read_file("data/a-b.txt").unwrap(), b"hyphen");
        assert!(!grf.file_exists("data/a_b.txt"));
        assert!(grf.read_file("data/a_b.txt").is_err());
    }

    #[test]
    fn archive_exact_snake_key_wins_over_non_snake_twin() {
        let grf = open_with(
            "archive_twin",
            &[("data/a b.txt", b"space"), ("data/a_b.txt", b"snake")],
        );
        assert_eq!(grf.read_file("data/a b.txt").unwrap(), b"space");
        assert_eq!(grf.read_file("data/a_b.txt").unwrap(), b"snake");
    }

    #[test]
    fn data_dir_resolves_snake_file_for_non_snake_request() {
        let dir = temp_path("data_dir");
        std::fs::write(dir.join("1_cata.rsw"), b"rsw").unwrap();
        let grf_path = temp_path("data_dir_grf").join("test.grf");
        drop(GrfArchive::create(&grf_path).unwrap());
        let grf = GrfArchive::open_layered(&[grf_path.to_string_lossy().into_owned()], Some(&dir))
            .unwrap();
        assert_eq!(grf.read_file("data/1@cata.rsw").unwrap(), b"rsw");
        assert!(grf.file_exists("data/1@cata.rsw"));
    }

    #[test]
    fn data_dir_resolves_non_snake_file_for_snake_request() {
        let dir = temp_path("data_dir_rev");
        std::fs::write(dir.join("1@cata.rsw"), b"rsw").unwrap();
        let index = DataDirIndex::build(&dir);
        assert!(index.lookup("data/1_cata.rsw").is_some());
        assert!(index.lookup("data/1@cata.rsw").is_some());
    }

    #[test]
    fn data_dir_ambiguous_pair_is_found_only_by_exact_name() {
        let dir = temp_path("data_dir_amb");
        std::fs::write(dir.join("a b.txt"), b"space").unwrap();
        std::fs::write(dir.join("a-b.txt"), b"hyphen").unwrap();
        let index = DataDirIndex::build(&dir);
        assert!(index.lookup("data/a b.txt").is_some());
        assert!(index.lookup("data/a-b.txt").is_some());
        assert!(index.lookup("data/a_b.txt").is_none());
    }
}
