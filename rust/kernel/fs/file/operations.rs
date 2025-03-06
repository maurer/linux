//! Traits and helpers for filling out `file_operations` from Rust

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
pub trait Operations: Sized {
    /// Type that comes into open when constructing
    type Init;

    /// Seek impl
    fn llseek(_file: &File<Self>, _offset: loff_t, _whence: Whence) -> Result<loff_t> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// read impl
    fn read(
        _file: &File<Self>,
        _out: UserSliceWriter,
        _bytes: usize,
        _offset: &mut loff_t,
    ) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// read impl
    fn write(
        _file: &File<Self>,
        _in_: UserSliceReader,
        _bytes: usize,
        _offset: &mut loff_t,
    ) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// open impl
    // TODO inode access
    fn open(_file: &RawFile<Self::Init>) -> Result<Self> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// release impl
    // TODO inode access
    fn release(_file: &File<Self>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// ioctl impl
    // TODO proper ioctl cmd type
    // TODO validate that removing isize as an option is right here - I think it is, because that
    // negative space will get our error encoding.
    fn ioctl(_file: &File<Self>, _cmd: u32, _arg: usize) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// ioctl cmopat impl
    // TODO proper ioctl cmd type
    // TODO validate that removing isize as an option is right here
    #[cfg(CONFIG_COMPAT)]
    fn compat_ioctl(_file: &File<Self>, _cmd: u32, _arg: usize) -> Result<usize> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// show fdinfo impl
    fn show_fdinfo(_seq_file: &SeqFile, _file: &File<Self>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}
