use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io,
    path::{Component, Path, PathBuf},
};

/// A live directory and the original address the user chose for it. Effects
/// are relative to the live object. The caller revalidates the address around
/// publication to distinguish completion at the intended path from recovery.
pub(super) struct Directory {
    file: File,
    path: PathBuf,
    // Windows maintains an enumeration cursor per open file object.
    #[cfg(windows)]
    enumeration: std::sync::Mutex<()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EntryKind {
    Directory,
    RegularFile,
    Unsupported,
}

impl Directory {
    fn capture(file: File, path: PathBuf) -> Self {
        Self {
            file,
            path,
            #[cfg(windows)]
            enumeration: std::sync::Mutex::new(()),
        }
    }

    pub(super) fn open(path: &Path) -> io::Result<Self> {
        Ok(Self::capture(
            platform::open_directory(path)?,
            path.to_owned(),
        ))
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Duplicate the witness for identity checks. The clone shares the native
    /// open file object and must leave directory enumeration to `entries`.
    pub(super) fn try_clone_file(&self) -> io::Result<File> {
        self.file.try_clone()
    }

    pub(super) fn verify(&self) -> io::Result<()> {
        let current = platform::open_directory(self.path())?;
        if platform::same_directory(&self.file, &current)? {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the path no longer names the captured directory",
            ))
        }
    }

    pub(super) fn open_directory(&self, name: &OsStr) -> io::Result<Self> {
        let file = platform::open_child_directory(&self.file, name)?;
        Ok(Self::capture(file, self.path.join(name)))
    }

    pub(super) fn open_file(&self, name: &OsStr) -> io::Result<File> {
        platform::open_file(&self.file, name)
    }

    pub(super) fn create_file(&self, name: &OsStr) -> io::Result<File> {
        platform::create_file(&self.file, name)
    }

    pub(super) fn create_directory(&self, name: &OsStr) -> io::Result<()> {
        platform::create_directory(&self.file, name)
    }

    /// Resolve entries against the captured object, including when its
    /// original path has been replaced. Names retain their native encoding.
    pub(super) fn entries(&self) -> io::Result<Vec<(OsString, EntryKind)>> {
        #[cfg(windows)]
        let _scan = self
            .enumeration
            .lock()
            .map_err(|_| io::Error::other("directory enumeration lock was poisoned"))?;
        platform::entries(&self.file)
    }
}

/// Publish a source name in the captured destination, failing on collision.
/// This performs one native same-volume rename and has no copy fallback.
pub(super) fn publish(
    source: &Directory,
    name: &OsStr,
    destination: &Directory,
    target: &OsStr,
) -> io::Result<()> {
    platform::publish(&source.file, name, &destination.file, target)
}

/// Remove a leaf in the captured directory. On macOS, unlink checks the
/// parent handle but cannot compare the leaf to a previously observed inode.
pub(super) fn remove_file(directory: &Directory, name: &OsStr) -> io::Result<()> {
    platform::remove(&directory.file, name, false)
}

/// Removes an empty child only. Callers must separately verify directory
/// ownership before using this to clean operation-owned staging.
pub(super) fn remove_directory(directory: &Directory, name: &OsStr) -> io::Result<()> {
    platform::remove(&directory.file, name, true)
}

/// Exchange two names, retaining the displaced target at the staging name.
/// The OS does not compare the target identity with a pre-swap witness.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub(super) fn swap(
    staging: &Directory,
    staged: &OsStr,
    destination: &Directory,
    target: &OsStr,
) -> io::Result<()> {
    platform::swap(&staging.file, staged, &destination.file, target)
}

fn validate_leaf(name: &OsStr) -> io::Result<()> {
    let mut components = Path::new(name).components();
    if matches!(components.next(), Some(Component::Normal(leaf)) if leaf == name)
        && components.next().is_none()
    {
        Ok(())
    } else {
        Err(io::Error::from(io::ErrorKind::InvalidInput))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::{
        ffi::CString,
        fs::OpenOptions,
        mem::{MaybeUninit, offset_of},
        os::fd::{AsRawFd, FromRawFd, IntoRawFd},
        os::unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::MetadataExt,
            fs::OpenOptionsExt,
        },
        ptr, slice,
    };

    const DIRECTORY_OPEN: i32 = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW;
    const FILE_READ: i32 = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK;

    fn leaf(name: &OsStr) -> io::Result<CString> {
        validate_leaf(name)?;
        CString::new(name.as_bytes()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
    }

    pub(super) fn open_directory(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(DIRECTORY_OPEN)
            .open(path)
    }

    pub(super) fn same_directory(left: &File, right: &File) -> io::Result<bool> {
        let left = left.metadata()?;
        let right = right.metadata()?;
        Ok(left.dev() == right.dev() && left.ino() == right.ino())
    }

    fn open_relative(parent: &File, name: &OsStr, flags: i32) -> io::Result<File> {
        open_at(parent, &leaf(name)?, flags)
    }

    fn open_at(parent: &File, name: &std::ffi::CStr, flags: i32) -> io::Result<File> {
        // The directory and terminated filename both remain live for the call.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC,
                0o600 as libc::c_int,
            )
        };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            // openat returned a new descriptor and the File takes sole ownership.
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    }

    pub(super) fn open_child_directory(parent: &File, name: &OsStr) -> io::Result<File> {
        open_relative(parent, name, DIRECTORY_OPEN)
    }

    struct DirectoryStream(*mut libc::DIR);

    impl Drop for DirectoryStream {
        fn drop(&mut self) {
            // fdopendir transferred ownership of the reopened descriptor.
            unsafe { libc::closedir(self.0) };
        }
    }

    pub(super) fn entries(parent: &File) -> io::Result<Vec<(OsString, EntryKind)>> {
        // Reopen relative to the handle. Cloning the File would share its
        // file-description offset and would make successive scans incomplete.
        let file = open_at(parent, c".", DIRECTORY_OPEN)?;
        let fd = file.as_raw_fd();
        // On success fdopendir assumes ownership. On failure File closes it.
        let stream = unsafe { libc::fdopendir(fd) };
        if stream.is_null() {
            return Err(io::Error::last_os_error());
        }
        let _owned_descriptor = file.into_raw_fd();
        let stream = DirectoryStream(stream);
        let mut result = Vec::new();
        loop {
            // readdir returns null for both end-of-stream and failure.
            let entry = unsafe {
                *libc::__error() = 0;
                libc::readdir(stream.0)
            };
            if entry.is_null() {
                let error = io::Error::last_os_error();
                return if error.raw_os_error() == Some(0) {
                    Ok(result)
                } else {
                    Err(error)
                };
            }
            // Read only the fixed fields until the variable record length is
            // checked. The entry remains valid until the next readdir.
            let (length, record_length, kind) = unsafe {
                (
                    (*entry).d_namlen as usize,
                    (*entry).d_reclen as usize,
                    (*entry).d_type,
                )
            };
            let header = offset_of!(libc::dirent, d_name);
            if length == 0 || length >= 1024 || header + length >= record_length {
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }
            // The checked record has room for the declared name and its terminator.
            let name = unsafe {
                slice::from_raw_parts(ptr::addr_of!((*entry).d_name).cast::<u8>(), length + 1)
            };
            let (terminator, name) = name.split_last().unwrap();
            if *terminator != 0 || name.contains(&0) {
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }
            if name == b"." || name == b".." {
                continue;
            }
            let name = OsString::from_vec(name.to_vec());
            let kind = match kind {
                libc::DT_DIR => EntryKind::Directory,
                libc::DT_REG => EntryKind::RegularFile,
                libc::DT_UNKNOWN => stat_entry(fd, &name)?,
                _ => EntryKind::Unsupported,
            };
            result.push((name, kind));
        }
    }

    fn stat_entry(parent: i32, name: &OsStr) -> io::Result<EntryKind> {
        let name = leaf(name)?;
        let mut stat = MaybeUninit::<libc::stat>::uninit();
        // The fallback checks the entry itself even if its type is a symlink.
        check(unsafe {
            libc::fstatat(
                parent,
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        })?;
        let mode = unsafe { stat.assume_init() }.st_mode;
        Ok(match mode & libc::S_IFMT {
            libc::S_IFDIR => EntryKind::Directory,
            libc::S_IFREG => EntryKind::RegularFile,
            _ => EntryKind::Unsupported,
        })
    }

    pub(super) fn open_file(parent: &File, name: &OsStr) -> io::Result<File> {
        let file = open_relative(parent, name, FILE_READ)?;
        if file.metadata()?.is_file() {
            Ok(file)
        } else {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }
    }

    pub(super) fn create_file(parent: &File, name: &OsStr) -> io::Result<File> {
        open_relative(
            parent,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW,
        )
    }

    pub(super) fn create_directory(parent: &File, name: &OsStr) -> io::Result<()> {
        let name = leaf(name)?;
        // The directory and terminated filename both remain live for the call.
        let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
        check(result)
    }

    pub(super) fn publish(
        source: &File,
        name: &OsStr,
        destination: &File,
        target: &OsStr,
    ) -> io::Result<()> {
        rename(source, name, destination, target, libc::RENAME_EXCL)
    }

    pub(super) fn swap(
        source: &File,
        name: &OsStr,
        destination: &File,
        target: &OsStr,
    ) -> io::Result<()> {
        rename(source, name, destination, target, libc::RENAME_SWAP)
    }

    fn rename(
        source: &File,
        name: &OsStr,
        destination: &File,
        target: &OsStr,
        flags: u32,
    ) -> io::Result<()> {
        let name = leaf(name)?;
        let target = leaf(target)?;
        // Both path components and the two open directory handles remain live.
        let result = unsafe {
            libc::renameatx_np(
                source.as_raw_fd(),
                name.as_ptr(),
                destination.as_raw_fd(),
                target.as_ptr(),
                flags,
            )
        };
        check(result)
    }

    pub(super) fn remove(parent: &File, name: &OsStr, directory: bool) -> io::Result<()> {
        let name = leaf(name)?;
        let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
        // The directory and terminated filename both remain live for the call.
        let result = unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), flags) };
        check(result)
    }

    fn check(result: i32) -> io::Result<()> {
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{
        ffi::c_void,
        fs::OpenOptions,
        mem::{offset_of, size_of},
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            fs::OpenOptionsExt,
            io::{AsRawHandle, FromRawHandle, RawHandle},
        },
        ptr, slice,
    };
    use windows_sys::Win32::Foundation::ERROR_NO_MORE_FILES;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_TAG_INFO, FILE_DISPOSITION_FLAG_DELETE,
        FILE_DISPOSITION_FLAG_POSIX_SEMANTICS, FILE_DISPOSITION_INFO_EX,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_GENERIC_WRITE, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FILE_TRAVERSE, FileAttributeTagInfo, FileDispositionInfoEx, FileIdInfo,
        FileRenameInfo, GetFileInformationByHandleEx, SYNCHRONIZE, SetFileInformationByHandle,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DEVICE, FILE_ID_BOTH_DIR_INFO, FILE_LIST_DIRECTORY, FileIdBothDirectoryInfo,
        FileIdBothDirectoryRestartInfo,
    };

    // winternl.h constants for a synchronous NtCreateFile relative to a handle.
    const OBJ_CASE_INSENSITIVE: u32 = 0x40;
    const FILE_OPEN: u32 = 1;
    const FILE_CREATE: u32 = 2;
    const FILE_DIRECTORY_FILE: u32 = 1;
    const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x20;
    const FILE_OPEN_REPARSE_POINT: u32 = 0x20_0000;
    const SHARED: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
    const DIRECTORY_ACCESS: u32 = FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES;

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }

    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: RawHandle,
        object_name: *mut UnicodeString,
        attributes: u32,
        security_descriptor: *mut c_void,
        security_quality_of_service: *mut c_void,
    }

    #[repr(C)]
    struct IoStatusBlock {
        // The first union is NTSTATUS or a pointer. Its alignment and size are
        // those of a pointer, so reserving one usize covers either member.
        status: usize,
        information: usize,
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtCreateFile(
            file_handle: *mut RawHandle,
            access: u32,
            attributes: *mut ObjectAttributes,
            status: *mut IoStatusBlock,
            allocation_size: *const i64,
            file_attributes: u32,
            share_access: u32,
            disposition: u32,
            options: u32,
            ea_buffer: *const c_void,
            ea_length: u32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }

    fn leaf(name: &OsStr) -> io::Result<Vec<u16>> {
        validate_leaf(name)?;
        let name: Vec<u16> = name.encode_wide().collect();
        // The native APIs bypass Win32 basename normalization. Reject drives,
        // data streams, DOS path syntax and trailing aliases before that bypass.
        if name
            .iter()
            .any(|&unit| unit < 32 || matches!(unit, 34 | 42 | 47 | 58 | 60 | 62 | 63 | 92 | 124))
            || matches!(name.last(), Some(32 | 46))
        {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        if name.len() > 255 {
            return Err(io::Error::from(io::ErrorKind::InvalidFilename));
        }
        Ok(name)
    }

    fn value<T: Default>(file: &File, class: i32) -> io::Result<T> {
        let mut result = T::default();
        // Each caller pairs the information class with the matching C structure.
        let success = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                class,
                ptr::from_mut(&mut result).cast(),
                size_of::<T>() as u32,
            )
        };
        check(success)?;
        Ok(result)
    }

    fn attributes(file: &File) -> io::Result<u32> {
        let data: FILE_ATTRIBUTE_TAG_INFO = value(file, FileAttributeTagInfo)?;
        Ok(data.FileAttributes)
    }

    fn require_directory(file: File) -> io::Result<File> {
        let attributes = attributes(&file)?;
        reject_reparse(attributes)?;
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            Ok(file)
        } else {
            Err(io::Error::from(io::ErrorKind::NotADirectory))
        }
    }

    fn reject_reparse(attributes: u32) -> io::Result<()> {
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        } else {
            Ok(())
        }
    }

    pub(super) fn open_directory(path: &Path) -> io::Result<File> {
        require_directory(
            OpenOptions::new()
                .access_mode(DIRECTORY_ACCESS)
                .share_mode(SHARED)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(path)?,
        )
    }

    pub(super) fn same_directory(left: &File, right: &File) -> io::Result<bool> {
        let left: FILE_ID_INFO = value(left, FileIdInfo)?;
        let right: FILE_ID_INFO = value(right, FileIdInfo)?;
        Ok(left.VolumeSerialNumber == right.VolumeSerialNumber
            && left.FileId.Identifier == right.FileId.Identifier)
    }

    fn open_relative(
        parent: &File,
        name: &OsStr,
        access: u32,
        disposition: u32,
        options: u32,
    ) -> io::Result<File> {
        let mut leaf = leaf(name)?;
        let length = (2 * leaf.len()) as u16;
        let mut name = UnicodeString {
            length,
            maximum_length: length,
            buffer: leaf.as_mut_ptr(),
        };
        let mut attributes = ObjectAttributes {
            length: size_of::<ObjectAttributes>() as u32,
            root_directory: parent.as_raw_handle(),
            object_name: &mut name,
            attributes: OBJ_CASE_INSENSITIVE,
            security_descriptor: ptr::null_mut(),
            security_quality_of_service: ptr::null_mut(),
        };
        let mut status = IoStatusBlock {
            status: 0,
            information: 0,
        };
        let mut handle = ptr::null_mut();
        // The handle and all buffers are live for synchronous NtCreateFile.
        // Windows resolves the one leaf relative to the held parent handle.
        let result = unsafe {
            NtCreateFile(
                &mut handle,
                access | SYNCHRONIZE,
                &mut attributes,
                &mut status,
                ptr::null(),
                FILE_ATTRIBUTE_NORMAL,
                SHARED,
                disposition,
                options | FILE_SYNCHRONOUS_IO_NONALERT,
                ptr::null(),
                0,
            )
        };
        if result < 0 {
            // Translate kernel status to the Win32 errors used by std::io.
            return Err(io::Error::from_raw_os_error(
                unsafe { RtlNtStatusToDosError(result) } as i32,
            ));
        }
        if handle.is_null() {
            return Err(io::Error::from(io::ErrorKind::Other));
        }
        // Windows returned a new owned handle.
        Ok(unsafe { File::from_raw_handle(handle) })
    }

    pub(super) fn open_child_directory(parent: &File, name: &OsStr) -> io::Result<File> {
        require_directory(open_relative(
            parent,
            name,
            DIRECTORY_ACCESS,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT,
        )?)
    }

    pub(super) fn entries(parent: &File) -> io::Result<Vec<(OsString, EntryKind)>> {
        // FILE_ID_BOTH_DIR_INFO requires eight-byte alignment. The caller
        // serializes scans on this file object, which owns the kernel cursor.
        let mut buffer = vec![0_u64; 8192];
        let bytes = buffer.len() * size_of::<u64>();
        let mut class = FileIdBothDirectoryRestartInfo;
        let mut result = Vec::new();
        loop {
            buffer.fill(0);
            // The buffer lives until the synchronous query and parse finish.
            let success = unsafe {
                GetFileInformationByHandleEx(
                    parent.as_raw_handle(),
                    class,
                    buffer.as_mut_ptr().cast(),
                    bytes as u32,
                )
            };
            if success == 0 {
                let error = io::Error::last_os_error();
                return if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                    Ok(result)
                } else {
                    Err(error)
                };
            }
            // The slice covers only the initialized allocation.
            let records = unsafe { slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), bytes) };
            parse_entries(records, &mut result)?;
            class = FileIdBothDirectoryInfo;
        }
    }

    fn parse_entries(mut data: &[u8], output: &mut Vec<(OsString, EntryKind)>) -> io::Result<()> {
        const HEADER: usize = offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
        let invalid = || io::Error::from(io::ErrorKind::InvalidData);
        loop {
            let header = data.get(..HEADER).ok_or_else(invalid)?;
            let field = |offset| {
                let bytes = header[offset..offset + size_of::<u32>()]
                    .try_into()
                    .unwrap();
                u32::from_ne_bytes(bytes)
            };
            let next = field(offset_of!(FILE_ID_BOTH_DIR_INFO, NextEntryOffset)) as usize;
            let size = field(offset_of!(FILE_ID_BOTH_DIR_INFO, FileNameLength)) as usize;
            let attributes = field(offset_of!(FILE_ID_BOTH_DIR_INFO, FileAttributes));
            let end = HEADER.checked_add(size).ok_or_else(invalid)?;
            if size == 0
                || !size.is_multiple_of(2)
                || (next != 0 && (!next.is_multiple_of(8) || next < end || next >= data.len()))
            {
                return Err(invalid());
            }
            let name = data.get(HEADER..end).ok_or_else(invalid)?;
            let name: Vec<u16> = name
                .as_chunks::<2>()
                .0
                .iter()
                .copied()
                .map(u16::from_ne_bytes)
                .collect();
            if name.contains(&0) {
                return Err(invalid());
            }
            if name != [u16::from(b'.')] && name != [u16::from(b'.'); 2] {
                let kind =
                    if attributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DEVICE) != 0 {
                        EntryKind::Unsupported
                    } else if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                        EntryKind::Directory
                    } else {
                        EntryKind::RegularFile
                    };
                output.push((OsString::from_wide(&name), kind));
            }
            if next == 0 {
                return Ok(());
            }
            data = &data[next..];
        }
    }

    pub(super) fn open_file(parent: &File, name: &OsStr) -> io::Result<File> {
        let file = open_relative(
            parent,
            name,
            FILE_GENERIC_READ,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT,
        )?;
        let attributes = attributes(&file)?;
        reject_reparse(attributes)?;
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return Err(io::Error::from(io::ErrorKind::IsADirectory));
        }
        Ok(file)
    }

    pub(super) fn create_file(parent: &File, name: &OsStr) -> io::Result<File> {
        open_relative(
            parent,
            name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_CREATE,
            FILE_OPEN_REPARSE_POINT,
        )
    }

    pub(super) fn create_directory(parent: &File, name: &OsStr) -> io::Result<()> {
        open_relative(
            parent,
            name,
            FILE_READ_ATTRIBUTES,
            FILE_CREATE,
            FILE_DIRECTORY_FILE,
        )?;
        Ok(())
    }

    #[repr(C)]
    struct RenameInfo {
        // An all-zero Win32 FILE_RENAME_INFO union selects ReplaceIfExists=FALSE.
        flags: u32,
        root_directory: RawHandle,
        filename_length: u32,
        filename: [u16; 256],
    }

    pub(super) fn publish(
        source: &File,
        name: &OsStr,
        destination: &File,
        target: &OsStr,
    ) -> io::Result<()> {
        let target = leaf(target)?;
        let entry = open_relative(
            source,
            name,
            DELETE | FILE_READ_ATTRIBUTES,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT,
        )?;
        reject_reparse(attributes(&entry)?)?;
        let mut info = RenameInfo {
            flags: 0,
            root_directory: destination.as_raw_handle(),
            filename_length: (2 * target.len()) as u32,
            filename: [0; 256],
        };
        info.filename[..target.len()].copy_from_slice(&target);
        let bytes = offset_of!(RenameInfo, filename) + (target.len() + 1) * size_of::<u16>();
        // The buffer contains FILE_RENAME_INFO plus a terminated UTF-16 leaf.
        // RootDirectory is the held destination, so no by-path fallback is used.
        let result = unsafe {
            SetFileInformationByHandle(
                entry.as_raw_handle(),
                FileRenameInfo,
                ptr::from_mut(&mut info).cast(),
                bytes as u32,
            )
        };
        check(result)
    }

    pub(super) fn remove(parent: &File, name: &OsStr, directory: bool) -> io::Result<()> {
        let entry = open_relative(
            parent,
            name,
            DELETE | FILE_READ_ATTRIBUTES,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT,
        )?;
        let attributes = attributes(&entry)?;
        reject_reparse(attributes)?;
        let is_directory = attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
        if directory != is_directory {
            return Err(io::Error::from(if is_directory {
                io::ErrorKind::IsADirectory
            } else {
                io::ErrorKind::NotADirectory
            }));
        }
        let mut disposition = FILE_DISPOSITION_INFO_EX {
            Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        };
        // Mark this opened entry for deletion. Closing the handle releases the
        // link; other handles to the same file can still read its content.
        let result = unsafe {
            SetFileInformationByHandle(
                entry.as_raw_handle(),
                FileDispositionInfoEx,
                ptr::from_mut(&mut disposition).cast(),
                size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
            )
        };
        check(result)
    }

    fn check(result: i32) -> io::Result<()> {
        if result != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::{Read, Write},
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Case(PathBuf);

    impl Case {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            loop {
                let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("folderai-native-{}-{id}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("could not create the test directory: {error}"),
                }
            }
        }

        fn directory(&self, name: &str) -> Directory {
            let path = self.0.join(name);
            fs::create_dir(&path).unwrap();
            Directory::open(&path).unwrap()
        }
    }

    impl Drop for Case {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn publication_and_creation_preserve_preexisting_names() {
        let fixture = Case::new();
        let source = fixture.directory("source");
        let target = fixture.directory("target");
        source
            .create_file(OsStr::new("caffè staged"))
            .unwrap()
            .write_all(b"prepared")
            .unwrap();
        fs::write(target.path().join("existing"), b"original").unwrap();

        let error = match target.create_file(OsStr::new("existing")) {
            Ok(_) => panic!("exclusive creation unexpectedly replaced an existing file"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            publish(
                &source,
                OsStr::new("caffè staged"),
                &target,
                OsStr::new("existing")
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read(target.path().join("existing")).unwrap(),
            b"original"
        );
        assert_eq!(
            fs::read(source.path().join("caffè staged")).unwrap(),
            b"prepared"
        );

        target.create_directory(OsStr::new("folder")).unwrap();
        let nested = target.open_directory(OsStr::new("folder")).unwrap();
        publish(
            &source,
            OsStr::new("caffè staged"),
            &nested,
            OsStr::new("new"),
        )
        .unwrap();
        let mut data = Vec::new();
        nested
            .open_file(OsStr::new("new"))
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"prepared");
        assert!(!source.path().join("caffè staged").exists());
        drop(nested);
        remove_file(
            &target.open_directory(OsStr::new("folder")).unwrap(),
            OsStr::new("new"),
        )
        .unwrap();
        remove_directory(&target, OsStr::new("folder")).unwrap();
    }

    #[test]
    fn enumeration_reads_multiple_native_batches() {
        let fixture = Case::new();
        let directory = fixture.directory("directory");
        let expected = (0..900)
            .map(|number| {
                let name = OsString::from(format!("entry-{number:04}-caffè.txt"));
                directory.create_file(&name).unwrap();
                (name, EntryKind::RegularFile)
            })
            .collect::<Vec<_>>();
        let mut entries = directory.entries().unwrap();
        entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(entries, expected);
    }

    #[test]
    fn publication_keeps_both_capture_anchors_after_path_substitution() {
        let fixture = Case::new();
        let source = fixture.directory("source");
        let target = fixture.directory("target");
        fs::write(source.path().join("staged"), b"prepared").unwrap();
        fs::rename(source.path(), fixture.0.join("source-original")).unwrap();
        fs::rename(target.path(), fixture.0.join("target-original")).unwrap();
        fs::create_dir(source.path()).unwrap();
        fs::write(source.path().join("staged"), b"unrelated source").unwrap();
        fs::create_dir(target.path()).unwrap();
        fs::write(target.path().join("other"), b"unrelated target").unwrap();

        assert!(source.verify().is_err());
        assert!(target.verify().is_err());
        let cloned = target.try_clone_file().unwrap();
        assert!(cloned.metadata().unwrap().is_dir());
        assert_eq!(
            source.entries().unwrap(),
            vec![(OsString::from("staged"), EntryKind::RegularFile)]
        );
        assert!(target.entries().unwrap().is_empty());
        publish(&source, OsStr::new("staged"), &target, OsStr::new("result")).unwrap();
        assert_eq!(
            fs::read(fixture.0.join("target-original/result")).unwrap(),
            b"prepared"
        );
        assert_eq!(
            fs::read(source.path().join("staged")).unwrap(),
            b"unrelated source"
        );
        assert_eq!(
            fs::read(target.path().join("other")).unwrap(),
            b"unrelated target"
        );
        assert!(!target.path().join("result").exists());
        assert!(target.verify().is_err());
        for _ in 0..2 {
            assert_eq!(
                target.entries().unwrap(),
                vec![(OsString::from("result"), EntryKind::RegularFile)]
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_link_open_and_retains_the_target_after_exchange() {
        use std::os::unix::fs::symlink;

        let fixture = Case::new();
        let directory = fixture.directory("directory");
        fs::write(directory.path().join("old"), b"old").unwrap();
        fs::write(directory.path().join("staged"), b"new").unwrap();
        symlink("old", directory.path().join("file-link")).unwrap();
        symlink(directory.path(), fixture.0.join("directory-link")).unwrap();
        directory
            .create_directory(OsStr::new("child-folder"))
            .unwrap();
        symlink("child-folder", directory.path().join("child-link")).unwrap();
        assert!(directory.open_file(OsStr::new("file-link")).is_err());
        assert!(Directory::open(&fixture.0.join("directory-link")).is_err());
        let entries = directory.entries().unwrap();
        assert!(entries.contains(&(OsString::from("old"), EntryKind::RegularFile)));
        assert!(entries.contains(&(OsString::from("child-folder"), EntryKind::Directory)));
        assert!(entries.contains(&(OsString::from("file-link"), EntryKind::Unsupported)));
        assert!(entries.contains(&(OsString::from("child-link"), EntryKind::Unsupported)));

        swap(
            &directory,
            OsStr::new("staged"),
            &directory,
            OsStr::new("old"),
        )
        .unwrap();
        assert_eq!(fs::read(directory.path().join("old")).unwrap(), b"new");
        assert_eq!(fs::read(directory.path().join("staged")).unwrap(), b"old");
    }
}
