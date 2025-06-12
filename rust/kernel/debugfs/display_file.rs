// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2025 Google LLC.

use crate::prelude::*;
use crate::seq_file::SeqFile;
use crate::seq_print;
use crate::types::ForeignOwnable;
use core::fmt::Display;

/// Implements `open` for `file_operations` via `single_open` to fill out a `seq_file`.
///
/// # Safety
///
/// * `inode`'s private pointer must be the foreign representation of `D`, and no mutable borrows
///   are outstanding.
/// * `file` must point to a live, not-yet-initialized file object.
pub(crate) unsafe extern "C" fn display_open<D: ForeignOwnable + Sync>(
    inode: *mut bindings::inode,
    file: *mut bindings::file,
) -> c_int
where
    for<'a> D::Borrowed<'a>: Display,
{
    // SAFETY:
    // * `file` is acceptable by caller precondition.
    // * `print_act` will be called on a `seq_file` with private data set to the third argument,
    //   so we meet its safety requirements.
    // * The `data` pointer passed in the third argument is valid by caller preconditions.
    unsafe { bindings::single_open(file, Some(display_act::<D>), (*inode).i_private) }
}

/// Prints private data stashed in a seq_file to that seq file.
///
/// # Safety
///
/// `seq` must point to a live `seq_file` whose private data is the foreign representation of `D`,
/// and no mutable borrows are outsstanding.
pub(crate) unsafe extern "C" fn display_act<D: ForeignOwnable + Sync>(
    seq: *mut bindings::seq_file,
    _: *mut c_void,
) -> c_int
where
    for<'a> D::Borrowed<'a>: Display,
{
    // SAFETY: By caller precondition, this pointer is live, has the right type, and has no mutable
    // borrows outstanding.
    let data = unsafe { D::borrow((*seq).private as _) };
    // SAFETY: By caller precondition, `seq` points to a live `seq_file`, so we can lift
    // it.
    let seq_file = unsafe { SeqFile::from_raw(seq) };
    seq_print!(seq_file, "{}", data);
    0
}

// Work around lack of generic const items.
pub(crate) trait DisplayFile {
    const VTABLE: bindings::file_operations;
}

impl<D: ForeignOwnable + Sync> DisplayFile for D
where
    for<'a> D::Borrowed<'a>: Display,
{
    const VTABLE: bindings::file_operations = bindings::file_operations {
        read: Some(bindings::seq_read),
        llseek: Some(bindings::seq_lseek),
        release: Some(bindings::single_release),
        open: Some(display_open::<D>),
        // SAFETY: `file_operations` supports zeroes in all fields.
        ..unsafe { core::mem::zeroed() }
    };
}
