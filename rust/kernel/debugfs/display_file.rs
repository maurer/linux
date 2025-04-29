// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2025 Google LLC.

use crate::prelude::*;
use crate::seq_file::SeqFile;
use crate::seq_print;
use crate::types::ForeignOwnable;
use core::fmt::{Display, Formatter, Result};
use core::marker::PhantomData;
use core::ops::Deref;

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

/// Adapter to implement `Display` via a callback with the same representation as `T`.
///
/// # Invariants
///
/// If an instance for `FormatAdapter<_, F>` is constructed, `F` is inhabited.
#[repr(transparent)]
pub(crate) struct FormatAdapter<D, F> {
    inner: D,
    _formatter: PhantomData<F>,
}

impl<D, F> FormatAdapter<D, F> {
    pub(crate) fn new(inner: D, _f: &'static F) -> Self {
        // INVARIANT: We were passed a reference to F, so it is inhabited.
        FormatAdapter {
            inner,
            _formatter: PhantomData,
        }
    }
}

pub(crate) struct BorrowedAdapter<'a, D: ForeignOwnable, F> {
    borrowed: D::Borrowed<'a>,
    _formatter: PhantomData<F>,
}

// SAFETY: We delegate to D's implementation of `ForeignOwnable`, so `into_foreign` produced aligned
// pointers.
unsafe impl<D: ForeignOwnable, F> ForeignOwnable for FormatAdapter<D, F> {
    type PointedTo = D::PointedTo;
    type Borrowed<'a> = BorrowedAdapter<'a, D, F>;
    type BorrowedMut<'a> = Self::Borrowed<'a>;
    fn into_foreign(self) -> *mut Self::PointedTo {
        self.inner.into_foreign()
    }
    unsafe fn from_foreign(foreign: *mut Self::PointedTo) -> Self {
        Self {
            // SAFETY: `into_foreign` is delegated, so a delegated `from_foreign` is safe.
            inner: unsafe { D::from_foreign(foreign) },
            _formatter: PhantomData,
        }
    }
    unsafe fn borrow<'a>(foreign: *mut Self::PointedTo) -> Self::Borrowed<'a> {
        BorrowedAdapter {
            // SAFETY: `into_foreign` is delegated, so a delegated `borrow` is safe.
            borrowed: unsafe { D::borrow(foreign) },
            _formatter: PhantomData,
        }
    }
    unsafe fn borrow_mut<'a>(foreign: *mut Self::PointedTo) -> Self::BorrowedMut<'a> {
        // SAFETY: `borrow_mut` has stricter requirements than `borrow`
        unsafe { Self::borrow(foreign) }
    }
}

impl<'a, D: ForeignOwnable<Borrowed<'a>: Deref<Target = T>>, T, F> Display
    for BorrowedAdapter<'a, D, F>
where
    F: Fn(&T, &mut Formatter<'_>) -> Result + 'static,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
        // SAFETY: FormatAdapter<_, F> can only be constructed if F is inhabited
        let f: &F = unsafe { materialize_zst_fmt() };
        f(&self.borrowed, fmt)
    }
}

/// For types with a unique value, produce a static reference to it.
///
/// # Safety
///
/// The caller asserts that F is inhabited
unsafe fn materialize_zst_fmt<F>() -> &'static F {
    const { assert!(core::mem::size_of::<F>() == 0) };
    let zst_dangle: core::ptr::NonNull<F> = core::ptr::NonNull::dangling();
    // SAFETY: While the pointer is dangling, it is a dangling pointer to a ZST, based on the
    // assertion above. The type is also inhabited, by the caller's assertion. This means
    // we can materialize it.
    unsafe { zst_dangle.as_ref() }
}
