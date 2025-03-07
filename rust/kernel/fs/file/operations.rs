//! Traits and helpers for filling out `file_operations` from Rust

use crate::ffi::{c_int, c_long, c_uint, c_ulong};
use core::mem::MaybeUninit;
// TODO refine
use super::*;

use crate::bindings::loff_t;
use crate::error::VTABLE_DEFAULT_ERROR;
use crate::prelude::*;
use crate::seq_file::SeqFile;
use crate::uaccess::{UserSliceReader, UserSliceWriter};

#[repr(transparent)]
#[derive(PartialEq, Eq)]
/// Encode whence type
pub struct Whence(pub ffi::c_int);

/// Define fops in safish rust
/// This trait is intended to maximize flexibility. Consider one of the shortcut traits if you have
/// a simpler case.
///
/// The one exception to that is that we only support one type transition for the file's private
/// data (during open), and no opportunity to transition the inode private data type.
// Sized restriction is because this should always either be a ZST (the ref type) or a owning type
// TODO add defaults with vtable error
#[vtable]
pub trait Operations: Sized + 'static {
    /// Type that comes into open when constructing
    type Init<'a>: ForeignOwnable;
    /// Type of private_data after open
    type State: ForeignOwnable + Sync + Send + 'static;

    /// Seek impl
    fn llseek(_file: &File<Self::State>, _offset: loff_t, _whence: Whence) -> Result<loff_t> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// read impl
    fn read(
        _file: &File<Self::State>,
        _out: UserSliceWriter,
        _bytes: usize,
        _offset: &mut loff_t,
    ) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// read impl
    fn write(
        _file: &File<Self::State>,
        _in_: UserSliceReader,
        _bytes: usize,
        _offset: &mut loff_t,
    ) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// open impl
    // TODO inode access
    fn open<'a>(_file: &RawFile<Self::Init<'a>>) -> Result<Self::State> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// release impl
    /// Note - the owned private data will be dropped after this call returns. You should not do it
    /// manually.
    // TODO inode access
    fn release(_file: &File<Self::State>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// ioctl impl
    // TODO proper ioctl cmd type
    // TODO validate that removing isize as an option is right here - I think it is, because that
    // negative space will get our error encoding.
    fn ioctl(_file: &File<Self::State>, _cmd: u32, _arg: usize) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// ioctl cmopat impl
    // TODO proper ioctl cmd type
    // TODO validate that removing isize as an option is right here
    #[cfg(CONFIG_COMPAT)]
    fn compat_ioctl(_file: &File<Self::State>, _cmd: u32, _arg: usize) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// show fdinfo impl
    fn show_fdinfo(_seq_file: &SeqFile, _file: &File<Self::State>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
    /// vtable suitable for use in a file_operations slot
    const VTABLE: bindings::file_operations = bindings::file_operations {
        open: Some(fops_open::<Self>),
        release: if Self::HAS_RELEASE || core::mem::needs_drop::<Self::State>() {
            // If the drop is nontrivial, or we have a custom release, we need a pointer
            Some(fops_release::<Self>)
        } else {
            None
        },
        unlocked_ioctl: then_some(Self::HAS_IOCTL, fops_ioctl::<Self>),
        #[cfg(CONFIG_COMPAT)]
        compat_ioctl: if Self::HAS_COMPAT_IOCTL {
            Some(fops_compat_ioctl::<Self>)
        } else if Self::HAS_IOCTL {
            Some(bindings::compat_ptr_ioctl)
        } else {
            None
        },
        show_fdinfo: then_some(Self::HAS_SHOW_FDINFO, fops_show_fdinfo::<Self>),
        // SAFETY: All zeros is a valid value for `bindings::file_operations`.
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };
}

const fn then_some<T: Copy>(b: bool, t: T) -> Option<T> {
    if b {
        Some(t)
    } else {
        None
    }
}

/// # Safety
///
/// `file` and `inode` must be the file and inode for a file that is undergoing initialization.
/// The file must be associated with a `<T as Operations>::Init`.
unsafe extern "C" fn fops_open<T: Operations>(
    inode: *mut bindings::inode,
    raw_file: *mut bindings::file,
) -> c_int {
    // SAFETY: The pointers are valid and for a file being opened.
    let ret = unsafe { bindings::generic_file_open(inode, raw_file) };
    if ret != 0 {
        return ret;
    }

    // SAFETY:
    // * This underlying file is valid for (much longer than) the duration of `T::open`.
    // * There is no active fdget_pos region on the file on this thread.
    // * The file is guaranteed to use `<T as Operations>::Init` by precondition.
    let file = unsafe { InitFile::from_raw_file(raw_file) };

    let ptr = match T::open(&file) {
        Err(err) => return err.to_errno(),
        Ok(ptr) => ptr,
    };
    file.set_private(ptr);
    0
}

/// # Safety
///
/// `file` and `inode` must be the file and inode for a file that is being released. The file must
/// be associated with a `T`.
unsafe extern "C" fn fops_release<T: Operations>(
    _inode: *mut bindings::inode,
    file: *mut bindings::file,
) -> c_int {
    // SAFETY:
    // * The file is valid for the duration of this call.
    // * There is no active fdget_pos region on the file on this thread.
    let file = unsafe { File::from_raw_file(file) };
    if T::HAS_RELEASE {
        T::release(file)
    }
    // SAFETY: TODO
    drop(unsafe { file.take_private_data() } );

    0
}

/// # Safety
///
/// `file` must be a valid file that is associated with a `MiscDeviceRegistration<T>`.
unsafe extern "C" fn fops_ioctl<T: Operations>(
    file: *mut bindings::file,
    cmd: c_uint,
    arg: c_ulong,
) -> c_long {
    // SAFETY:
    // * The file is valid for the duration of this call.
    // * There is no active fdget_pos region on the file on this thread.
    // TODO amend for type
    let file = unsafe { File::from_raw_file(file) };

    match T::ioctl(file, cmd, arg) {
        Ok(ret) => ret as c_long,
        Err(err) => err.to_errno() as c_long,
    }
}

/// # Safety
///
/// `file` must be a valid file that is associated with a `MiscDeviceRegistration<T>`.
#[cfg(CONFIG_COMPAT)]
unsafe extern "C" fn fops_compat_ioctl<T: Operations>(
    file: *mut bindings::file,
    cmd: c_uint,
    arg: c_ulong,
) -> c_long {
    // SAFETY:
    // * The file is valid for the duration of this call.
    // * There is no active fdget_pos region on the file on this thread.
    // * TODO amend for type
    let file = unsafe { File::from_raw_file(file) };

    match T::compat_ioctl(file, cmd, arg) {
        Ok(ret) => ret as c_long,
        Err(err) => err.to_errno() as c_long,
    }
}

/// # Safety
///
/// - `file` must be a valid file that is associated with a `MiscDeviceRegistration<T>`.
/// - `seq_file` must be a valid `struct seq_file` that we can write to.
unsafe extern "C" fn fops_show_fdinfo<T: Operations>(
    seq_file: *mut bindings::seq_file,
    file: *mut bindings::file,
) {
    // SAFETY:
    // * The file is valid for the duration of this call.
    // * There is no active fdget_pos region on the file on this thread.
    let file = unsafe { File::from_raw_file(file) };
    // SAFETY: The caller ensures that the pointer is valid and exclusive for the duration in which
    // this method is called.
    let m = unsafe { SeqFile::from_raw(seq_file) };

    T::show_fdinfo(m, file);
}
