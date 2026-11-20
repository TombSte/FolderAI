use super::{LiveObservation, Locator};
use std::{
    ffi::{CStr, OsStr},
    fs::{self, File, Metadata, OpenOptions},
    io,
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, FromRawFd, RawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
    path::PathBuf,
    ptr,
    sync::{Mutex, OnceLock},
};

/// An open APFS handle keeps the original native object alive while FolderAI
/// checks the current path. A file ID sampled after reopening lacks this proof.
#[derive(Debug)]
pub struct LiveIdentity {
    file: File,
    volume_uuid: [u8; 16],
    deletion: OnceLock<DirectoryDeletion>,
}

impl LiveIdentity {
    pub fn open(locator: &Locator) -> io::Result<Self> {
        let observed = fs::symlink_metadata(locator.path())?;
        if !is_unambiguous(&observed) {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_EVTONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(locator.path())?;
        let current = Self::from_file(file)?;
        if !same_object(&current.file.metadata()?, &observed) {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        Ok(current)
    }

    pub(crate) fn from_file(file: File) -> io::Result<Self> {
        if !is_apfs(file.as_raw_fd())? || !is_unambiguous(&file.metadata()?) {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let volume_uuid = apfs_volume_uuid(file.as_raw_fd())?;
        Ok(Self {
            file,
            volume_uuid,
            deletion: OnceLock::new(),
        })
    }

    /// APFS keeps the link count of an open directory after rmdir. Arm a
    /// vnode observation before any relocation or removal to distinguish it
    /// from a changed path. Regular files already provide a zero-link proof.
    pub(crate) fn arm_directory_removal(&self) -> io::Result<()> {
        if self.file.metadata()?.is_dir() && self.deletion.get().is_none() {
            let observer = DirectoryDeletion::register(self.file.as_raw_fd())?;
            let _ = self.deletion.set(observer);
        }
        Ok(())
    }

    /// Returns a locator only if the path still resolves to this open object.
    pub fn observe(&self) -> io::Result<LiveObservation> {
        let original = self.file.metadata()?;
        if original.nlink() == 0 {
            return Ok(LiveObservation::Removed);
        }
        if let Some(deletion) = self.deletion.get()
            && deletion.removed()?
        {
            return Ok(LiveObservation::Removed);
        }
        if !is_unambiguous(&original) {
            return Ok(LiveObservation::Unavailable);
        }
        let mut buffer = [0u8; libc::PATH_MAX as usize];
        // F_GETPATH requires a writable buffer of at least PATH_MAX bytes.
        let result =
            unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) };
        if result == -1 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::NotFound {
                return Ok(LiveObservation::Unavailable);
            }
            return Err(error);
        }
        let path = CStr::from_bytes_until_nul(&buffer)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
        let path = PathBuf::from(OsStr::from_bytes(path.to_bytes()));
        let observed = match fs::symlink_metadata(&path) {
            Ok(observed) => observed,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(LiveObservation::Unavailable);
            }
            Err(error) => return Err(error),
        };
        if !same_object(&original, &observed) || !is_unambiguous(&observed) {
            return Ok(LiveObservation::Unavailable);
        }
        Ok(Locator::new(path).map_or(LiveObservation::Unavailable, LiveObservation::Present))
    }

    /// Persisted as a comparison key for the live observation. The bytes
    /// alone cannot establish continuity after a handle is lost.
    pub(crate) fn comparison_key(&self) -> io::Result<Vec<u8>> {
        let metadata = self.file.metadata()?;
        let birth = metadata
            .created()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| io::Error::from(io::ErrorKind::Unsupported))?;
        let mut key = b"apfs-live-v2\0".to_vec();
        key.extend(self.volume_uuid);
        key.extend(metadata.dev().to_le_bytes());
        key.extend(metadata.ino().to_le_bytes());
        key.extend(birth.as_nanos().to_le_bytes());
        Ok(key)
    }

    pub(crate) fn kind(&self) -> io::Result<&'static str> {
        Ok(if self.file.metadata()?.is_dir() {
            "directory"
        } else {
            "file"
        })
    }
}

#[derive(Debug)]
struct DirectoryDeletion {
    queue: File,
    removed: Mutex<bool>,
}

impl DirectoryDeletion {
    fn register(directory: RawFd) -> io::Result<Self> {
        // kqueue has no path arguments and returns a new owned descriptor.
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // The successful call above transferred ownership of this descriptor.
        let queue = unsafe { File::from_raw_fd(fd) };
        // Do not retain an extra reference across an exec of a different app.
        if unsafe { libc::fcntl(queue.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            return Err(io::Error::last_os_error());
        }
        let interest = libc::kevent {
            ident: directory as usize,
            filter: libc::EVFILT_VNODE,
            flags: libc::EV_ADD | libc::EV_CLEAR,
            fflags: libc::NOTE_DELETE,
            data: 0,
            udata: ptr::null_mut(),
        };
        // The directory remains open for the lifetime of the monitored identity.
        let result = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                &interest,
                1,
                ptr::null_mut(),
                0,
                ptr::null(),
            )
        };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            queue,
            removed: Mutex::new(false),
        })
    }

    fn removed(&self) -> io::Result<bool> {
        let mut removed = self
            .removed
            .lock()
            .map_err(|_| io::Error::other("directory removal observation lock was poisoned"))?;
        if !*removed {
            let mut event = MaybeUninit::<libc::kevent>::uninit();
            let immediate = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            // One output slot with a zero timeout never waits for a later event.
            let count = unsafe {
                libc::kevent(
                    self.queue.as_raw_fd(),
                    ptr::null(),
                    0,
                    event.as_mut_ptr(),
                    1,
                    &immediate,
                )
            };
            match count {
                0 => {}
                1 => {
                    // kevent wrote exactly one event.
                    let event = unsafe { event.assume_init() };
                    if event.flags & libc::EV_ERROR != 0 {
                        return Err(io::Error::from_raw_os_error(event.data as i32));
                    }
                    *removed = event.fflags & libc::NOTE_DELETE != 0;
                }
                _ => return Err(io::Error::last_os_error()),
            }
        }
        Ok(*removed)
    }
}

fn apfs_volume_uuid(fd: RawFd) -> io::Result<[u8; 16]> {
    let mut attributes = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: 0,
        volattr: libc::ATTR_VOL_INFO | libc::ATTR_VOL_UUID,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    let mut bytes = [0u8; 20];
    // The returned buffer consists of a four-byte length and one UUID.
    let result = unsafe {
        libc::fgetattrlist(
            fd,
            (&mut attributes as *mut libc::attrlist).cast(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            0,
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    let (length, uuid) = bytes.split_at(4);
    let uuid: [u8; 16] = uuid.try_into().unwrap();
    if u32::from_ne_bytes(length.try_into().unwrap()) != 20 || uuid == [0; 16] {
        return Err(io::Error::from(io::ErrorKind::Unsupported));
    }
    Ok(uuid)
}

fn is_apfs(fd: RawFd) -> io::Result<bool> {
    let mut info = MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(fd, info.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // A successful fstatfs initializes the structure and terminates f_fstypename.
    let info = unsafe { info.assume_init() };
    Ok(unsafe { CStr::from_ptr(info.f_fstypename.as_ptr()) }.to_bytes() == b"apfs")
}

fn same_object(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino() && left.file_type() == right.file_type()
}

fn is_unambiguous(metadata: &Metadata) -> bool {
    metadata.is_dir() || (metadata.is_file() && metadata.nlink() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_handle_identifies_moved_descendant_and_rejects_replacement() {
        let root = std::env::temp_dir().join(format!("folderai-live-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir(&source).unwrap();
        let child = source.join("caffè.txt");
        fs::write(&child, "original").unwrap();
        let captured = LiveIdentity::open(&Locator::new(child).unwrap()).unwrap();
        fs::rename(&source, &destination).unwrap();
        let moved = destination.join("caffè.txt");
        let LiveObservation::Present(observed) = captured.observe().unwrap() else {
            panic!("renamed item should still be reachable");
        };
        assert!(observed.path().ends_with("destination/caffè.txt"));
        assert_eq!(
            fs::canonicalize(observed.path()).unwrap(),
            fs::canonicalize(&moved).unwrap()
        );

        let alias = destination.join("alias");
        fs::hard_link(&moved, &alias).unwrap();
        assert_eq!(captured.observe().unwrap(), LiveObservation::Unavailable);
        fs::remove_file(alias).unwrap();
        let replacement = destination.join("replacement");
        fs::write(&replacement, "foreign").unwrap();
        fs::rename(&replacement, &moved).unwrap();
        assert_eq!(captured.observe().unwrap(), LiveObservation::Removed);
    }
}
