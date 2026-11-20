use super::{
    FileError, captured_directory_path,
    copy::{copy_name, version},
    native::{self, Directory, EntryKind},
};
use crate::local_identity::{LiveIdentity, LiveObservation, Locator};
use flate2::read::DeflateDecoder;
use rawzip::{CompressionMethod, FileReader, ZipArchive, ZipArchiveEntryWayfinder};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};
use uuid::Uuid;

const MAX_ARCHIVE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_EXPANDED: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILE: u64 = 512 * 1024 * 1024;
const MAX_DIRECTORY: u64 = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 4096;
const MAX_PATH: usize = 4096;
const MAX_DEPTH: usize = 32;
const BUFFER_SIZE: usize = 256 * 1024;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "code",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ZipError {
    InvalidArchive,
    InvalidPath,
    UnsupportedArchive,
    Encrypted,
    LimitExceeded,
    Cancelled,
    SourceChanged,
    DestinationChanged,
    Filesystem {
        error: FileError,
    },
    CleanupFailed {
        path: PathBuf,
    },
    RecoveryPending {
        path: PathBuf,
    },
    CreationFailed {
        path: PathBuf,
        reason: Box<ZipError>,
    },
}

impl From<io::Error> for ZipError {
    fn from(error: io::Error) -> Self {
        Self::Filesystem {
            error: error.into(),
        }
    }
}

impl From<rawzip::Error> for ZipError {
    fn from(error: rawzip::Error) -> Self {
        match error.kind() {
            rawzip::ErrorKind::IO(error) => map_archive_read(error),
            rawzip::ErrorKind::BufferTooSmall { .. } => Self::LimitExceeded,
            _ => Self::InvalidArchive,
        }
    }
}

fn map_archive_read(error: &io::Error) -> ZipError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => ZipError::Filesystem {
            error: FileError::PermissionDenied,
        },
        _ => ZipError::InvalidArchive,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ZipEntryKind {
    File,
    Directory,
    Link,
    Unsupported,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ZipEntry {
    pub(super) path: String,
    pub(super) kind: ZipEntryKind,
    pub(super) size: u64,
    pub(super) compressed_size: u64,
    pub(super) encrypted: bool,
    pub(super) extractable: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ZipExtraction {
    pub(super) path: PathBuf,
    pub(super) entries: usize,
}

struct ArchiveEntry {
    display: ZipEntry,
    relative: PathBuf,
    compression: CompressionMethod,
    wayfinder: ZipArchiveEntryWayfinder,
}

struct ArchiveSource {
    file: File,
    identity: LiveIdentity,
    location: Locator,
    version: (u64, SystemTime),
    archive: ZipArchive<FileReader>,
    entries: Vec<ArchiveEntry>,
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ZipError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ZipError::Cancelled)
    } else {
        Ok(())
    }
}

fn portable_leaf(part: &str) -> bool {
    if part.is_empty()
        || matches!(part, "." | "..")
        || part.len() > 255
        || part.ends_with(['.', ' '])
        || part.chars().any(|ch| {
            ch.is_control() || matches!(ch, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*')
        })
    {
        return false;
    }
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_ascii_uppercase();
    let device = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"));
    !matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) && !matches!(
        device,
        Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
    )
}

fn manifest_path(name: &str, is_directory: bool) -> Result<PathBuf, ZipError> {
    if name.len() > MAX_PATH {
        return Err(ZipError::LimitExceeded);
    }
    let relative = if is_directory {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    let mut path = PathBuf::new();
    for (depth, part) in relative.split('/').enumerate() {
        if depth == MAX_DEPTH {
            return Err(ZipError::LimitExceeded);
        }
        if !portable_leaf(part) {
            return Err(ZipError::InvalidPath);
        }
        path.push(part);
    }
    Ok(path)
}

fn read_manifest(
    archive: &ZipArchive<FileReader>,
    length: u64,
    cancelled: &AtomicBool,
) -> Result<Vec<ArchiveEntry>, ZipError> {
    let central_start = archive.directory_offset();
    let catalog_size = archive
        .eocd_offset()
        .checked_sub(central_start)
        .ok_or(ZipError::InvalidArchive)?;
    if archive.entries_hint() > MAX_ENTRIES as u64 || catalog_size > MAX_DIRECTORY {
        return Err(ZipError::LimitExceeded);
    }
    if archive.end_offset() != length {
        return Err(ZipError::InvalidArchive);
    }

    let mut directory_buf = vec![0; rawzip::MAX_CENTRAL_DIRECTORY_RECORD_SIZE];
    let mut local_buf = vec![0; 2 * u16::MAX as usize];
    let mut records = archive.entries(&mut directory_buf);
    let mut entries = Vec::new();
    let mut occupied = Vec::new();
    let mut names = BTreeSet::new();
    let mut directories = BTreeSet::new();
    let mut total = 0_u64;
    while let Some(record) = records.next_entry()? {
        check_cancelled(cancelled)?;
        if entries.len() == MAX_ENTRIES {
            return Err(ZipError::LimitExceeded);
        }

        let name = record.file_path();
        let name = std::str::from_utf8(name.as_ref()).map_err(|_| ZipError::UnsupportedArchive)?;
        let mode = record.mode();
        let is_directory = record.is_dir() || mode.value() & 0o170000 == 0o040000;
        let relative = manifest_path(name, is_directory)?;
        if !names.insert(relative.clone()) {
            return Err(ZipError::InvalidPath);
        }
        let kind = if mode.is_symlink() {
            ZipEntryKind::Link
        } else if record.external_attributes() & 0x400 != 0
            || !matches!(mode.value() & 0o170000, 0 | 0o100000 | 0o040000)
        {
            ZipEntryKind::Unsupported
        } else if is_directory {
            ZipEntryKind::Directory
        } else {
            ZipEntryKind::File
        };

        let size = record.uncompressed_size_hint();
        let compressed = record.compressed_size_hint();
        if size > MAX_FILE {
            return Err(ZipError::LimitExceeded);
        }
        total = total
            .checked_add(size)
            .filter(|&sum| sum <= MAX_EXPANDED)
            .ok_or(ZipError::LimitExceeded)?;
        if kind == ZipEntryKind::Directory && size != 0 {
            return Err(ZipError::InvalidArchive);
        }
        for parent in relative
            .ancestors()
            .skip(1)
            .take_while(|path| !path.as_os_str().is_empty())
        {
            directories.insert(parent.to_owned());
            if directories.len() > MAX_ENTRIES {
                return Err(ZipError::LimitExceeded);
            }
        }

        let wayfinder = record.wayfinder();
        let entry = archive.get_entry(wayfinder)?;
        let (start, end) = entry.compressed_data_range();
        let local_start = record.local_header_offset();
        if local_start >= start || end > central_start {
            return Err(ZipError::InvalidArchive);
        }
        occupied.push(local_start..end);
        let local = entry.local_header(&mut local_buf)?;
        let flags = record.flags();
        let compression = record.compression_method();
        if local.file_path().as_ref() != name.as_bytes()
            || local.flags() != flags
            || local.compression_method() != compression
            || (!flags.has_data_descriptor() && local.crc32() != record.crc32())
        {
            return Err(ZipError::InvalidArchive);
        }

        let encrypted = flags.is_encrypted() || flags.has_strong_encryption() || flags.is_masked();
        let extractable = !encrypted
            && matches!(kind, ZipEntryKind::File | ZipEntryKind::Directory)
            && matches!(
                compression,
                CompressionMethod::STORE | CompressionMethod::DEFLATE
            );
        entries.push(ArchiveEntry {
            display: ZipEntry {
                path: name.to_owned(),
                kind,
                size,
                compressed_size: compressed,
                encrypted,
                extractable,
            },
            relative,
            compression,
            wayfinder,
        });
    }
    if entries.len() as u64 != archive.entries_hint() {
        return Err(ZipError::InvalidArchive);
    }
    occupied.sort_unstable_by_key(|range| range.start);
    if occupied.windows(2).any(|pair| pair[0].end > pair[1].start) {
        return Err(ZipError::InvalidArchive);
    }
    if entries.iter().any(|entry| {
        entry.display.kind != ZipEntryKind::Directory && directories.contains(&entry.relative)
    }) {
        return Err(ZipError::InvalidPath);
    }
    // Explicit and implicit paths together bound the actual number of staged entries.
    if names.union(&directories).count() > MAX_ENTRIES {
        return Err(ZipError::LimitExceeded);
    }
    Ok(entries)
}

impl ArchiveSource {
    fn open(path: &Path, cancelled: &AtomicBool) -> Result<Self, ZipError> {
        check_cancelled(cancelled)?;
        let parent = Directory::open(path.parent().ok_or(ZipError::InvalidPath)?)?;
        let name = path.file_name().ok_or(ZipError::InvalidPath)?;
        let file = parent.open_file(name)?;
        let source_version = version(&file.metadata()?)?;
        if source_version.0 > MAX_ARCHIVE {
            return Err(ZipError::LimitExceeded);
        }
        let identity = LiveIdentity::from_file(file.try_clone()?)?;
        let location = Locator::new(
            captured_directory_path(&parent)
                .map_err(|error| ZipError::Filesystem { error })?
                .join(name),
        )
        .ok_or(ZipError::InvalidPath)?;
        let mut buffer = vec![0; rawzip::RECOMMENDED_BUFFER_SIZE];
        let archive = ZipArchive::from_file(file.try_clone()?, &mut buffer)?;
        let entries = read_manifest(&archive, source_version.0, cancelled)?;
        let source = Self {
            file,
            identity,
            location,
            version: source_version,
            archive,
            entries,
        };
        source.verify()?;
        Ok(source)
    }

    fn verify(&self) -> Result<(), ZipError> {
        if self.identity.observe()? != LiveObservation::Present(self.location.clone())
            || version(&self.file.metadata()?)? != self.version
        {
            return Err(ZipError::SourceChanged);
        }
        Ok(())
    }
}

pub(crate) fn list_zip(path: &Path) -> Result<Vec<ZipEntry>, ZipError> {
    let source = ArchiveSource::open(path, &AtomicBool::new(false))?;
    Ok(source
        .entries
        .into_iter()
        .map(|entry| entry.display)
        .collect())
}

/// Hold every destination component and open each child without following a
/// link or a reparse point. Rechecking the entire chain detects substituted
/// ancestors even if the selected leaf itself remains accessible.
struct Destination(Vec<Directory>);

impl Destination {
    fn open(path: &Path) -> Result<Self, ZipError> {
        let mut ancestors = path.ancestors().collect::<Vec<_>>();
        let root = ancestors
            .pop()
            .filter(|root| root.is_absolute())
            .ok_or(ZipError::InvalidPath)?;
        let mut chain = vec![Directory::open(root)?];
        while let Some(path) = ancestors.pop() {
            let name = path.file_name().ok_or(ZipError::InvalidPath)?;
            chain.push(
                chain
                    .last()
                    .expect("root already opened")
                    .open_directory(name)?,
            );
        }
        let destination = Self(chain);
        destination.verify()?;
        Ok(destination)
    }

    fn verify(&self) -> Result<(), ZipError> {
        for directory in &self.0 {
            directory
                .verify()
                .map_err(|_| ZipError::DestinationChanged)?;
        }
        Ok(())
    }

    fn leaf(&self) -> &Directory {
        self.0.last().expect("destination always has a root")
    }
}

enum StagedObject {
    Directory(Directory),
    File(LiveIdentity),
}

struct StagedEntry {
    parent: usize,
    name: OsString,
    object: StagedObject,
}

struct Stage {
    name: OsString,
    entries: Vec<StagedEntry>,
    directories: BTreeMap<PathBuf, usize>,
}

impl Stage {
    fn create(destination: &Destination) -> Result<Self, ZipError> {
        destination.verify()?;
        let name = OsString::from(format!(".folderai-zip-{}", Uuid::new_v4()));
        let leaf = destination.leaf();
        let root = leaf
            .create_staging_directory(&name)?
            .map_err(|_| ZipError::CleanupFailed {
                path: leaf.path().join(&name),
            })?;
        Ok(Self {
            entries: vec![StagedEntry {
                parent: 0,
                name: name.clone(),
                object: StagedObject::Directory(root),
            }],
            directories: BTreeMap::from([(PathBuf::new(), 0)]),
            name,
        })
    }

    fn directory_at(&self, index: usize) -> &Directory {
        match &self.entries[index].object {
            StagedObject::Directory(directory) => directory,
            StagedObject::File(_) => unreachable!("staging parent must be a directory"),
        }
    }

    fn verify_chain(&self, destination: &Destination, mut parent: usize) -> Result<(), ZipError> {
        destination.verify()?;
        loop {
            self.directory_at(parent)
                .verify()
                .map_err(|_| ZipError::DestinationChanged)?;
            if parent == 0 {
                return Ok(());
            }
            parent = self.entries[parent].parent;
        }
    }

    fn ensure_directory(
        &mut self,
        destination: &Destination,
        path: &Path,
    ) -> Result<usize, ZipError> {
        let mut parent = 0;
        let mut relative = PathBuf::new();
        for component in path.components() {
            let Component::Normal(name) = component else {
                return Err(ZipError::InvalidPath);
            };
            relative.push(name);
            if let Some(&existing) = self.directories.get(&relative) {
                parent = existing;
                continue;
            }
            self.verify_chain(destination, parent)?;
            let directory = self.directory_at(parent);
            let child =
                directory
                    .create_staging_directory(name)?
                    .map_err(|_| ZipError::CleanupFailed {
                        path: directory.path().join(name),
                    })?;
            let index = self.entries.len();
            self.entries.push(StagedEntry {
                parent,
                name: name.to_owned(),
                object: StagedObject::Directory(child),
            });
            self.directories.insert(relative.clone(), index);
            parent = index;
        }
        Ok(parent)
    }

    fn create_file(
        &mut self,
        destination: &Destination,
        path: &Path,
    ) -> Result<(usize, File), ZipError> {
        let parent_path = path.parent().ok_or(ZipError::InvalidPath)?;
        let name = path.file_name().ok_or(ZipError::InvalidPath)?;
        let parent = self.ensure_directory(destination, parent_path)?;
        self.verify_chain(destination, parent)?;
        let directory = self.directory_at(parent);
        let file = directory.create_file(name)?;
        let identity = LiveIdentity::from_file(file.try_clone()?)?;
        let index = self.entries.len();
        self.entries.push(StagedEntry {
            parent,
            name: name.to_owned(),
            object: StagedObject::File(identity),
        });
        Ok((index, file))
    }

    fn verify_file(&self, index: usize) -> Result<(), ZipError> {
        let entry = &self.entries[index];
        let parent = self.directory_at(entry.parent);
        let StagedObject::File(witness) = &entry.object else {
            unreachable!("file verification requires a file witness");
        };
        let present = parent
            .open_file(&entry.name)
            .and_then(LiveIdentity::from_file)
            .map_err(|_| ZipError::DestinationChanged)?;
        if present.comparison_key()? != witness.comparison_key()? {
            return Err(ZipError::DestinationChanged);
        }
        Ok(())
    }

    fn verify_inventory(&self, destination: &Destination) -> Result<(), ZipError> {
        destination.verify()?;
        let mut inventory = vec![Vec::new(); self.entries.len()];
        for (index, entry) in self.entries.iter().enumerate() {
            let kind = match &entry.object {
                StagedObject::Directory(directory) => {
                    directory
                        .verify()
                        .map_err(|_| ZipError::DestinationChanged)?;
                    EntryKind::Directory
                }
                StagedObject::File(_) => {
                    self.verify_file(index)?;
                    EntryKind::RegularFile
                }
            };
            if index != 0 {
                inventory[entry.parent].push((entry.name.clone(), kind));
            }
        }
        for (index, children) in inventory.iter_mut().enumerate() {
            if let StagedObject::Directory(directory) = &self.entries[index].object {
                children.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                let mut present = directory.entries()?;
                present.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                if present != *children {
                    return Err(ZipError::DestinationChanged);
                }
            }
        }
        Ok(())
    }

    fn cleanup(&self, destination: &Destination) -> Result<(), ZipError> {
        self.verify_inventory(destination)?;
        for (index, entry) in self.entries.iter().enumerate().skip(1).rev() {
            self.verify_chain(destination, entry.parent)?;
            let parent = self.directory_at(entry.parent);
            match &entry.object {
                StagedObject::Directory(directory) => {
                    directory.verify()?;
                    native::remove_directory(parent, &entry.name)?;
                }
                StagedObject::File(_) => {
                    self.verify_file(index)?;
                    native::remove_file(parent, &entry.name)?;
                }
            }
        }
        self.verify_chain(destination, 0)?;
        native::remove_directory(destination.leaf(), &self.name)?;
        Ok(())
    }

    fn retained_path(&self) -> PathBuf {
        let root = self.directory_at(0);
        let observed = root
            .try_clone_file()
            .and_then(LiveIdentity::from_file)
            .and_then(|identity| identity.observe());
        match observed {
            Ok(LiveObservation::Present(path)) => path.path().to_owned(),
            _ => root.path().to_owned(),
        }
    }
}

fn stage_contents(
    source: &ArchiveSource,
    stage: &mut Stage,
    destination: &Destination,
    cancelled: &AtomicBool,
) -> Result<(), ZipError> {
    let mut buffer = vec![0; BUFFER_SIZE];
    let mut expanded = 0_u64;
    for entry in &source.entries {
        check_cancelled(cancelled)?;
        source.verify()?;
        let archive_entry = source.archive.get_entry(entry.wayfinder)?;
        let raw = archive_entry.reader();
        let compressed: Box<dyn Read> = match entry.compression {
            CompressionMethod::STORE => Box::new(raw),
            CompressionMethod::DEFLATE => Box::new(DeflateDecoder::new(raw)),
            _ => return Err(ZipError::UnsupportedArchive),
        };
        let mut reader = archive_entry.verifying_reader(compressed);
        let mut target = if entry.display.kind == ZipEntryKind::Directory {
            stage.ensure_directory(destination, &entry.relative)?;
            None
        } else {
            Some(stage.create_file(destination, &entry.relative)?)
        };

        let mut written = 0_u64;
        loop {
            check_cancelled(cancelled)?;
            let count = reader
                .read(&mut buffer)
                .map_err(|error| map_archive_read(&error))?;
            if count == 0 {
                break;
            }
            written = written
                .checked_add(count as u64)
                .filter(|&count| count <= entry.display.size && count <= MAX_FILE)
                .ok_or(ZipError::LimitExceeded)?;
            expanded = expanded
                .checked_add(count as u64)
                .filter(|&size| size <= MAX_EXPANDED)
                .ok_or(ZipError::LimitExceeded)?;
            let (index, file) = target.as_mut().ok_or(ZipError::InvalidArchive)?;
            let parent = stage.entries[*index].parent;
            stage.verify_chain(destination, parent)?;
            stage.verify_file(*index)?;
            file.write_all(&buffer[..count])?;
        }
        if written != entry.display.size {
            return Err(ZipError::InvalidArchive);
        }
        if let Some((index, file)) = target {
            stage.verify_chain(destination, stage.entries[index].parent)?;
            stage.verify_file(index)?;
            file.sync_all()?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn preview_and_extract_without_overwriting() {
    tauri::async_runtime::block_on(async {
        let directory = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(Uuid::new_v4().to_string());
        std::fs::create_dir(&directory).unwrap();
        let archive = directory.join("fixture.ZIP");
        let hex = "504b03041400000000001b5c745d86a6103605000000050000000900000068656c6c6f2e74787468656c6c6f504b010214031400000000001b5c745d86a61036050000000500000009000000000000000000000080010000000068656c6c6f2e747874504b05060000000001000100370000002c0000000000";
        let bytes: Vec<_> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        std::fs::write(&archive, bytes).unwrap();
        let source = archive.to_str().unwrap().to_owned();
        let entries = inspect_zip(source.clone()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "hello.txt");
        let first = extract_zip_here(source.clone()).await.unwrap();
        let second = extract_zip_here(source).await.unwrap();
        assert_ne!(first.path, second.path);
        for output in [first.path, second.path] {
            assert_eq!(std::fs::read(output.join("hello.txt")).unwrap(), b"hello");
        }
        let folder = directory.join("source");
        std::fs::create_dir_all(folder.join("empty")).unwrap();
        std::fs::write(folder.join("note.txt"), b"folder content").unwrap();
        let source = folder.to_str().unwrap().to_owned();
        let compressed = compress_entry_zip(source.clone(), true).await.unwrap();
        assert_eq!(compressed, directory.join("source.zip"));
        assert_ne!(compressed, compress_entry_zip(source, true).await.unwrap());
        let restored = extract_zip_here(compressed.to_str().unwrap().to_owned())
            .await
            .unwrap();
        assert!(restored.path.join("source/empty").is_dir());
        assert_eq!(
            std::fs::read(restored.path.join("source/note.txt")).unwrap(),
            b"folder content"
        );
        let file = folder.join("note.txt");
        let archive = compress_entry_zip(file.to_str().unwrap().into(), false)
            .await
            .unwrap();
        assert_eq!(archive, folder.join("note.txt.zip"));
        let restored = extract_zip_here(archive.to_str().unwrap().into())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(restored.path.join("note.txt")).unwrap(),
            b"folder content"
        );
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[tauri::command]
pub(crate) async fn extract_zip_here(source: String) -> Result<ZipExtraction, ZipError> {
    let path = super::parse_path(source).map_err(|error| ZipError::Filesystem { error })?;
    if !path
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        return Err(ZipError::InvalidPath);
    }
    tauri::async_runtime::spawn_blocking(move || {
        let destination = path.parent().ok_or(ZipError::InvalidPath)?;
        extract_zip(&path, destination, &AtomicBool::new(false))
    })
    .await
    .map_err(|_| ZipError::InvalidArchive)?
}

fn write_folder(
    writer: &mut zip::ZipWriter<File>,
    directory: &Directory,
    prefix: &str,
    depth: usize,
    entries: &mut usize,
    bytes: &mut u64,
) -> Result<(), ZipError> {
    if depth > MAX_DEPTH {
        return Err(ZipError::LimitExceeded);
    }
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let inventory = directory.entries()?;
    for (name, kind) in &inventory {
        *entries += 1;
        if *entries > MAX_ENTRIES {
            return Err(ZipError::LimitExceeded);
        }
        let leaf = name
            .to_str()
            .filter(|name| portable_leaf(name))
            .ok_or(ZipError::InvalidPath)?;
        let path = format!("{prefix}{leaf}");
        if path.len() > MAX_PATH {
            return Err(ZipError::LimitExceeded);
        }
        match kind {
            EntryKind::Directory => {
                let prefix = format!("{path}/");
                writer
                    .add_directory(&prefix, options)
                    .map_err(write_error)?;
                write_folder(
                    writer,
                    &directory.open_directory(name)?,
                    &prefix,
                    depth + 1,
                    entries,
                    bytes,
                )?;
            }
            EntryKind::RegularFile => {
                write_zip_file(writer, directory.open_file(name)?, &path, bytes)?;
            }
            EntryKind::Unsupported => return Err(ZipError::UnsupportedArchive),
        }
    }
    let mut current = directory.entries()?;
    current.sort_by(|a, b| a.0.cmp(&b.0));
    let mut inventory = inventory;
    inventory.sort_by(|a, b| a.0.cmp(&b.0));
    if inventory != current || directory.verify().is_err() {
        return Err(ZipError::SourceChanged);
    }
    Ok(())
}

fn write_error(error: zip::result::ZipError) -> ZipError {
    match error {
        zip::result::ZipError::Io(error) => error.into(),
        _ => ZipError::InvalidArchive,
    }
}

fn write_zip_file(
    writer: &mut zip::ZipWriter<File>,
    mut file: File,
    name: &str,
    bytes: &mut u64,
) -> Result<(), ZipError> {
    let before = version(&file.metadata()?)?;
    let size = before.0;
    if size > MAX_FILE || *bytes + size > MAX_EXPANDED {
        return Err(ZipError::LimitExceeded);
    }
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer.start_file(name, options).map_err(write_error)?;
    if io::copy(&mut (&mut file).take(size + 1), writer)? != size
        || version(&file.metadata()?)? != before
    {
        return Err(ZipError::SourceChanged);
    }
    *bytes += size;
    Ok(())
}

fn compress_entry(path: &Path, is_directory: bool) -> Result<PathBuf, ZipError> {
    let parent = Directory::open(path.parent().ok_or(ZipError::InvalidPath)?)?;
    let name = path.file_name().ok_or(ZipError::InvalidPath)?;
    let (folder, source) = if is_directory {
        (Some(parent.open_directory(name)?), None)
    } else {
        (None, Some(parent.open_file(name)?))
    };
    let name = name
        .to_str()
        .filter(|name| portable_leaf(name))
        .ok_or(ZipError::InvalidPath)?;
    let desired = OsString::from(format!("{name}.zip"));
    let temporary = OsString::from(format!(".folderai-zip-{}.tmp", Uuid::new_v4()));
    let file = parent.create_file(&temporary)?;
    let staging_path = parent.path().join(&temporary);
    let witness = LiveIdentity::from_file(file.try_clone()?)?;
    let location = Locator::new(staging_path.clone()).ok_or(ZipError::InvalidPath)?;
    let result = (|| {
        let mut writer = zip::ZipWriter::new(file);
        let mut bytes = 0;
        if let Some(folder) = folder {
            let prefix = format!("{name}/");
            writer
                .add_directory(&prefix, zip::write::SimpleFileOptions::default())
                .map_err(write_error)?;
            let mut entries = 1;
            write_folder(&mut writer, &folder, &prefix, 1, &mut entries, &mut bytes)?;
        } else if let Some(source) = source {
            write_zip_file(&mut writer, source, name, &mut bytes)?;
        }
        let file = writer.finish().map_err(write_error)?;
        if file.metadata()?.len() > MAX_ARCHIVE {
            return Err(ZipError::LimitExceeded);
        }
        file.sync_all()?;
        for sequence in 0..1000 {
            parent.verify()?;
            if witness.observe()? != LiveObservation::Present(location.clone()) {
                return Err(ZipError::RecoveryPending {
                    path: staging_path.clone(),
                });
            }
            let target = copy_name(&desired, sequence, false);
            match native::publish(&parent, &temporary, &parent, &target) {
                Ok(()) => {
                    let output = parent.path().join(target);
                    if parent.verify().is_err()
                        || !Locator::new(output.clone()).is_some_and(|location| {
                            witness.observe().ok() == Some(LiveObservation::Present(location))
                        })
                    {
                        return Err(ZipError::RecoveryPending { path: output });
                    }
                    return Ok(output);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(ZipError::LimitExceeded)
    })();
    // Do not unlink a staging name after a failed operation: it may no longer
    // denote the file we created. Surface its location for manual recovery.
    match result {
        Ok(path) => Ok(path),
        Err(error @ ZipError::RecoveryPending { .. }) => Err(error),
        Err(error) => Err(ZipError::CreationFailed {
            path: staging_path,
            reason: Box::new(error),
        }),
    }
}

#[tauri::command]
pub(crate) async fn compress_entry_zip(
    source: String,
    is_directory: bool,
) -> Result<PathBuf, ZipError> {
    let path = super::parse_path(source).map_err(|error| ZipError::Filesystem { error })?;
    tauri::async_runtime::spawn_blocking(move || compress_entry(&path, is_directory))
        .await
        .map_err(|_| ZipError::InvalidArchive)?
}

#[tauri::command]
pub(crate) async fn inspect_zip(source: String) -> Result<Vec<ZipEntry>, ZipError> {
    let path = super::parse_path(source).map_err(|error| ZipError::Filesystem { error })?;
    tauri::async_runtime::spawn_blocking(move || list_zip(&path))
        .await
        .map_err(|_| ZipError::InvalidArchive)?
}

pub(crate) fn extract_zip(
    path: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
) -> Result<ZipExtraction, ZipError> {
    extract_with_hook(path, destination, cancelled, || {})
}

fn extract_with_hook(
    path: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
    before_publish: impl FnOnce(),
) -> Result<ZipExtraction, ZipError> {
    let output_name = path
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| portable_leaf(name))
        .ok_or(ZipError::InvalidPath)?;
    let destination = Destination::open(destination)?;
    let source = ArchiveSource::open(path, cancelled)?;
    if source.entries.iter().any(|entry| entry.display.encrypted) {
        return Err(ZipError::Encrypted);
    }
    if source
        .entries
        .iter()
        .any(|entry| !entry.display.extractable)
    {
        return Err(ZipError::UnsupportedArchive);
    }
    check_cancelled(cancelled)?;
    let mut stage = Stage::create(&destination)?;
    let result = (|| {
        stage_contents(&source, &mut stage, &destination, cancelled)?;
        before_publish();
        stage.verify_inventory(&destination)?;
        let parent = destination.leaf();
        for number in 0..10_000 {
            check_cancelled(cancelled)?;
            source.verify()?;
            stage.verify_chain(&destination, 0)?;
            let name = copy_name(OsStr::new(output_name), number, true);
            match native::publish(parent, &stage.name, parent, &name) {
                Ok(()) => {
                    let path = parent.path().join(name);
                    if destination.verify().is_err()
                        || stage.directory_at(0).relocate(path.clone()).is_err()
                    {
                        return Err(ZipError::RecoveryPending {
                            path: stage.retained_path(),
                        });
                    }
                    return Ok(ZipExtraction {
                        path,
                        entries: source.entries.len(),
                    });
                }
                Err(error) => {
                    if stage.verify_chain(&destination, 0).is_err() {
                        return Err(ZipError::RecoveryPending {
                            path: stage.retained_path(),
                        });
                    }
                    if error.kind() != io::ErrorKind::AlreadyExists {
                        return Err(error.into());
                    }
                }
            }
        }
        Err(ZipError::Filesystem {
            error: FileError::AlreadyExists,
        })
    })();
    match result {
        Err(ZipError::RecoveryPending { .. }) | Ok(_) => result,
        Err(error) => match stage.cleanup(&destination) {
            Ok(()) => Err(error),
            Err(_) => Err(ZipError::CleanupFailed {
                path: stage.retained_path(),
            }),
        },
    }
}
