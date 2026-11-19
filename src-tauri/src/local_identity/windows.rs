use super::{LiveObservation, Locator};
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_BASIC_INFO,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_NAME_NORMALIZED,
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO,
    FileAttributeTagInfo, FileBasicInfo, FileIdInfo, FileStandardInfo, GetDriveTypeW,
    GetFileInformationByHandleEx, GetFinalPathNameByHandleW, GetVolumeInformationByHandleW,
    VOLUME_NAME_DOS,
};

#[derive(Debug)]
pub struct LiveIdentity(File);

impl LiveIdentity {
    pub fn open(locator: &Locator) -> io::Result<Self> {
        if !is_fixed_drive(locator.path()) {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let file = open_reference(locator.path())?;
        Self::from_file(file)
    }

    pub(crate) fn from_file(file: File) -> io::Result<Self> {
        let info = details(&file)?;
        if !info.is_unambiguous() || !is_ntfs(&file)? {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let candidate = Self(file);
        match candidate.observe()? {
            LiveObservation::Present(_) => Ok(candidate),
            _ => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }

    pub fn observe(&self) -> io::Result<LiveObservation> {
        let original = details(&self.0)?;
        if original.standard.DeletePending && original.standard.NumberOfLinks == 0 {
            return Ok(LiveObservation::Removed);
        }
        if original.standard.DeletePending || !original.is_unambiguous() {
            return Ok(LiveObservation::Unavailable);
        }
        let path = match final_path(&self.0) {
            Ok(path) if is_fixed_drive(&path) => path,
            Ok(_) => return Ok(LiveObservation::Unavailable),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(LiveObservation::Unavailable);
            }
            Err(error) => return Err(error),
        };
        let current = match open_reference(&path) {
            Ok(file) => details(&file)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(LiveObservation::Unavailable);
            }
            Err(error) => return Err(error),
        };
        if !current.is_unambiguous()
            || original.id.VolumeSerialNumber != current.id.VolumeSerialNumber
            || original.id.FileId.Identifier != current.id.FileId.Identifier
            || original.standard.Directory != current.standard.Directory
        {
            return Ok(LiveObservation::Unavailable);
        }
        Ok(Locator::new(ordinary_local_path(&path)?)
            .map_or(LiveObservation::Unavailable, LiveObservation::Present))
    }

    pub(crate) fn comparison_key(&self) -> io::Result<Vec<u8>> {
        let info = details(&self.0)?;
        let id = info.id;
        if info.basic.CreationTime <= 0 {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let mut key = b"ntfs-live-v2\0".to_vec();
        key.extend(id.VolumeSerialNumber.to_le_bytes());
        key.extend(id.FileId.Identifier);
        key.extend(info.basic.CreationTime.to_le_bytes());
        Ok(key)
    }

    pub(crate) fn kind(&self) -> io::Result<&'static str> {
        Ok(if details(&self.0)?.standard.Directory {
            "directory"
        } else {
            "file"
        })
    }
}

pub(super) fn ordinary_local_path(path: &Path) -> io::Result<PathBuf> {
    match path.components().next() {
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::VerbatimDisk(_)) => {
            let words: Vec<u16> = path.as_os_str().encode_wide().skip(4).collect();
            Ok(PathBuf::from(OsString::from_wide(&words)))
        }
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)) => {
            Ok(path.to_owned())
        }
        _ => Err(io::Error::from(io::ErrorKind::Unsupported)),
    }
}

struct Details {
    id: FILE_ID_INFO,
    basic: FILE_BASIC_INFO,
    standard: FILE_STANDARD_INFO,
    tags: FILE_ATTRIBUTE_TAG_INFO,
}

impl Details {
    fn is_unambiguous(&self) -> bool {
        self.tags.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT == 0
            && self.id.FileId.Identifier != [0; 16]
            && (self.standard.Directory || self.standard.NumberOfLinks == 1)
    }
}

fn details(file: &File) -> io::Result<Details> {
    let mut id = FILE_ID_INFO::default();
    let mut basic = FILE_BASIC_INFO::default();
    let mut standard = FILE_STANDARD_INFO::default();
    let mut tags = FILE_ATTRIBUTE_TAG_INFO::default();
    for (class, pointer, size) in [
        (
            FileIdInfo,
            ptr::from_mut(&mut id).cast(),
            size_of::<FILE_ID_INFO>(),
        ),
        (
            FileBasicInfo,
            ptr::from_mut(&mut basic).cast(),
            size_of::<FILE_BASIC_INFO>(),
        ),
        (
            FileStandardInfo,
            ptr::from_mut(&mut standard).cast(),
            size_of::<FILE_STANDARD_INFO>(),
        ),
        (
            FileAttributeTagInfo,
            ptr::from_mut(&mut tags).cast(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>(),
        ),
    ] {
        // Each class receives exactly the initialized structure type and size
        // specified by GetFileInformationByHandleEx.
        if unsafe {
            GetFileInformationByHandleEx(file.as_raw_handle(), class, pointer, size as u32)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(Details {
        id,
        basic,
        standard,
        tags,
    })
}

fn open_reference(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

fn final_path(file: &File) -> io::Result<PathBuf> {
    let mut output = vec![0u16; 1024];
    loop {
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                output.as_mut_ptr(),
                output.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        } as usize;
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        if length < output.len() {
            return Ok(PathBuf::from(OsString::from_wide(&output[..length])));
        }
        if length > 32_768 {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        output.resize(length + 1, 0);
    }
}

fn is_fixed_drive(path: &Path) -> bool {
    const DRIVE_FIXED: u32 = 3;
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return false;
    };
    let letter = match prefix.kind() {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
        _ => return false,
    };
    let root = [u16::from(letter), u16::from(b':'), u16::from(b'\\'), 0];
    unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_FIXED }
}

fn is_ntfs(file: &File) -> io::Result<bool> {
    let mut format = [0u16; 16];
    let result = unsafe {
        GetVolumeInformationByHandleW(
            file.as_raw_handle(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            format.as_mut_ptr(),
            format.len() as u32,
        )
    };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(format.starts_with(&[b'N' as u16, b'T' as u16, b'F' as u16, b'S' as u16, 0]))
}
