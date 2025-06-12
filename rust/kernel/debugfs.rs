// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2025 Google LLC.

//! DebugFS Abstraction
//!
//! C header: [`include/linux/debugfs.h`](srctree/include/linux/debugfs.h)

use crate::prelude::*;
use crate::str::CStr;
#[cfg(CONFIG_DEBUG_FS)]
use crate::sync::Arc;
use crate::types::ForeignOwnable;
use core::fmt::Display;

#[cfg(CONFIG_DEBUG_FS)]
mod display_file;
#[cfg(CONFIG_DEBUG_FS)]
mod entry;
#[cfg(CONFIG_DEBUG_FS)]
use entry::Entry;

/// Owning handle to a DebugFS directory.
///
/// This directory will be cleaned up when the handle and all child directory/file handles have
/// been dropped.
// We hold a reference to our parent if it exists to prevent the dentry we point to from being
// cleaned up when our parent is removed.
pub struct Dir(#[cfg(CONFIG_DEBUG_FS)] Option<Arc<Entry>>);

impl Dir {
    /// Create a new directory in DebugFS. If `parent` is [`None`], it will be created at the root.
    #[cfg(CONFIG_DEBUG_FS)]
    fn create(name: &CStr, parent: Option<&Dir>) -> Self {
        let parent_ptr = match parent {
            // If the parent couldn't be allocated, just early-return
            Some(Dir(None)) => return Self(None),
            Some(Dir(Some(entry))) => entry.as_ptr(),
            None => core::ptr::null_mut(),
        };
        // SAFETY:
        // * `name` argument points to a NUL-terminated string that lives across the call, by
        //   invariants of `&CStr`.
        // * If `parent` is `None`, `parent_ptr` is null to mean create at root.
        // * If `parent` is `Some`, `parent_ptr` is a live dentry debugfs pointer.
        let dir = unsafe { bindings::debugfs_create_dir(name.as_char_ptr(), parent_ptr) };

        Self(
            // If Arc creation fails, the `Entry` will be dropped, so the directory will be cleaned
            // up.
            Arc::new(
                // SAFETY: `debugfs_create_dir` either returns an error code or a legal `dentry`
                // pointer, and the parent is the same one passed to `debugfs_create_dir`
                unsafe { Entry::new(dir, parent.and_then(|dir| dir.0.clone())) },
                GFP_KERNEL,
            )
            .ok(),
        )
    }

    #[cfg(not(CONFIG_DEBUG_FS))]
    fn create(_name: &CStr, _parent: Option<&Dir>) -> Self {
        Self()
    }

    #[cfg(CONFIG_DEBUG_FS)]
    fn create_file<D: ForeignOwnable + Send + Sync>(&self, name: &CStr, data: D) -> File
    where
        for<'a> D::Borrowed<'a>: Display,
    {
        let mut file = File {
            _entry: Entry::empty(),
            _foreign: ForeignHolder::new(data),
        };

        let Some(parent) = &self.0 else {
            return file;
        };

        // SAFETY:
        // * `name` is a NUL-terminated C string, living across the call, by `CStr` invariant.
        // * `parent` is a live `dentry` since we have a reference to it.
        // * `vtable` is all stock `seq_file` implementations except for `open`.
        //   `open`'s only requirement beyond what is provided to all open functions is that the
        //   inode's data pointer must point to a `T` that will outlive it, which we know because
        //   we have an owning `D` in the `File`, and we tear down the file during `Drop`.
        let ptr = unsafe {
            bindings::debugfs_create_file_full(
                name.as_char_ptr(),
                0o444,
                parent.as_ptr(),
                file._foreign.data,
                core::ptr::null(),
                &<D as display_file::DisplayFile>::VTABLE,
            )
        };

        // SAFETY: `debugfs_create_file_full` either returns an error code or a legal
        // dentry pointer, so `Entry::new` is safe to call here.
        file._entry = unsafe { Entry::new(ptr, Some(parent.clone())) };

        file
    }

    #[cfg(not(CONFIG_DEBUG_FS))]
    fn create_file<D: ForeignOwnable>(&self, _name: &CStr, data: D) -> File
    where
        for<'a> D::Borrowed<'a>: Display,
    {
        File {
            _foreign: ForeignHolder::new(data),
        }
    }

    /// Create a DebugFS subdirectory.
    ///
    /// Subdirectory handles cannot outlive the directory handle they were created from.
    ///
    /// # Examples
    ///
    /// ```
    /// # use kernel::c_str;
    /// # use kernel::debugfs::Dir;
    /// let parent = Dir::new(c_str!("parent"));
    /// let child = parent.subdir(c_str!("child"));
    /// ```
    pub fn subdir(&self, name: &CStr) -> Self {
        Dir::create(name, Some(self))
    }

    /// Create a file in a DebugFS directory with the provided name, and contents from invoking
    /// [`Display::fmt`] on the provided reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use kernel::c_str;
    /// # use kernel::debugfs::Dir;
    /// let dir = Dir::new(c_str!("my_debugfs_dir"));
    /// dir.display_file(c_str!("foo"), &200);
    /// // "my_debugfs_dir/foo" now contains the number 200.
    /// ```
    ///
    /// ```
    /// # use kernel::c_str;
    /// # use kernel::debugfs::Dir;
    /// # use kernel::prelude::*;
    /// let val = KBox::new(300, GFP_KERNEL)?;
    /// let dir = Dir::new(c_str!("my_debugfs_dir"));
    /// dir.display_file(c_str!("foo"), val);
    /// // "my_debugfs_dir/foo" now contains the number 300.
    /// # Ok::<(), Error>(())
    /// ```
    pub fn display_file<D: ForeignOwnable + Send + Sync>(&self, name: &CStr, data: D) -> File
    where
        for<'a> D::Borrowed<'a>: Display,
    {
        self.create_file(name, data)
    }

    /// Create a new directory in DebugFS at the root.
    ///
    /// # Examples
    ///
    /// ```
    /// # use kernel::c_str;
    /// # use kernel::debugfs::Dir;
    /// let debugfs = Dir::new(c_str!("parent"));
    /// ```
    pub fn new(name: &CStr) -> Self {
        Dir::create(name, None)
    }
}

/// Handle to a DebugFS file.
pub struct File {
    // This order is load-bearing for drops - `_entry` must be dropped before `_foreign`
    #[cfg(CONFIG_DEBUG_FS)]
    _entry: Entry,
    _foreign: ForeignHolder,
}

struct ForeignHolder {
    data: *mut c_void,
    drop_hook: unsafe fn(*mut c_void),
}

// SAFETY: We only construct `ForeignHolder` using a pointer from a `ForeignOwnable` which
// is also `Sync`.
unsafe impl Sync for ForeignHolder {}
// SAFETY: We only construct `ForeignHolder` using a pointer from a `ForeignOwnable` which
// is also `Send`.
unsafe impl Send for ForeignHolder {}

/// Helper function to drop a `D`-typed foreign ownable from its foreign representation, useful for
/// cases where you want the type erased.
/// # Safety
/// * The foreign pointer passed in must have come from `D`'s `ForeignOwnable::into_foreign`
/// * There must be no outstanding `ForeignOwnable::borrow{,mut}`
/// * The pointer must not have been `ForeignOwnable::from_foreign`'d
unsafe fn drop_helper<D: ForeignOwnable>(foreign: *mut c_void) {
    // SAFETY: By safetydocs, we meet the requirements for `from_foreign`
    drop(unsafe { D::from_foreign(foreign as _) })
}

impl ForeignHolder {
    fn new<D: ForeignOwnable>(data: D) -> Self {
        Self {
            data: data.into_foreign() as _,
            drop_hook: drop_helper::<D>,
        }
    }
}

impl Drop for ForeignHolder {
    fn drop(&mut self) {
        // SAFETY: `drop_hook` corresponds to the original `ForeignOwnable` instance's `drop`.
        // This is only used in the case of `File`, so the only place borrows occur is through the
        // DebugFS file owned by `_entry`. Since `_entry` occurs earlier in the struct, it will be
        // dropped first, so no borrows will be ongoing. We know no `from_foreign` has occurred
        // because this pointer is not exposed anywhere that is called.
        unsafe { (self.drop_hook)(self.data) }
    }
}
