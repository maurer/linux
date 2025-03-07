// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

//! DebugFS Abstraction
//!
//! C header: [`include/linux/debugfs.h`](srctree/include/linux/debugfs.h)

use crate::error::from_err_ptr;
use crate::seq_file::SeqFile;
use crate::seq_print;
use crate::types::ARef;
use crate::types::AlwaysRefCounted;
use crate::types::Opaque;
use core::fmt;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use core::marker::PhantomPinned;
use core::ops::Deref;
use core::ptr::NonNull;
use kernel::prelude::*;

/// Handle to a DebugFS directory.
pub struct Dir {
    inner: Opaque<bindings::dentry>,
}

// SAFETY: Dir is just a `dentry` under the hood, which the API promises can be transferred
// between threads.
unsafe impl Send for Dir {}

// SAFETY: All the native functions we re-export use interior locking, and the contents of the
// struct are opaque to Rust.
unsafe impl Sync for Dir {}

// SAFETY: Dir is actually `dentry`, and dget/dput are the reference counting functions
// for it.
unsafe impl AlwaysRefCounted for Dir {
    #[inline]
    fn inc_ref(&self) {
        // SAFETY: Since we have a reference to the directory,
        // it's live, so it's safe to call dget on it.
        unsafe {
            kernel::bindings::dget(self.as_ptr());
        }
    }
    #[inline]
    unsafe fn dec_ref(obj: NonNull<Self>) {
        // SAFETY: By the caller precondition on the trait, we know that the caller has a reference
        // count to the object.
        unsafe {
            kernel::bindings::dput(obj.cast().as_ptr());
        }
    }
}

impl Dir {
    /// Create a new directory at the root of DebugFS
    pub fn new(name: &CStr) -> Result<ARef<Self>> {
        // SAFETY:
        // * name argument points to a null terminated string that lives across the call, by
        //   invariants of &CStr
        // * parent accepts null pointers to mean create at root
        let dir = NonNull::new(from_err_ptr(unsafe {
            kernel::bindings::debugfs_create_dir(name.as_char_ptr(), core::ptr::null_mut())
        })?);
        // SAFETY:
        // * debugfs_create_dir either returns an error code or a legal dentry pointer, so
        //   unwrap_unchecked is safe
        // * Dir is a transparent wrapper for an Opaque<dentry>, and we received a live
        //   owning dentry from debugfs_create_dir, so we can wrap it in an ARef
        Ok(unsafe { ARef::from_raw(dir.unwrap_unchecked().cast()) })
    }
    fn as_ptr(&self) -> *mut bindings::dentry {
        self.inner.get()
    }
}

/// Implements `open` for `file_operations` via `single_open` to fill out a `seq_file`
/// # Safety
/// * inode's private pointer must point to a value of type T which will outlive the inode and will
///   not be mutated during this call
/// * file must point to a live, not-yet-initialized file object
unsafe extern "C" fn display_open<T: Display>(
    inode: *mut kernel::bindings::inode,
    file: *mut kernel::bindings::file,
) -> i32 {
    // SAFETY:
    // * file is acceptable by caller precondition
    // * print_act will be called on a seq_file with private data set to the third argument, so we
    //   meet its safety requirements
    // * The data pointer passed in the third argument is a valid T pointer that outlives this call
    //   by caller preconditions
    unsafe { kernel::bindings::single_open(file, Some(display_act::<T>), (*inode).i_private) }
}

/// Prints private data stashed in a seq_file to that seq file
// # Safety
// * seq must point to a live seq_file whose private data is a live pointer to a T which is not
//   being mutated.
unsafe extern "C" fn display_act<T: Display>(
    seq: *mut kernel::bindings::seq_file,
    _: *mut core::ffi::c_void,
) -> i32 {
    // SAFETY: By caller precondition, this pointer is live, points to a value of type T, and is
    // not being mutated.
    let data = unsafe { &*((*seq).private as *mut T) };
    // SAFETY: By caller precondition, seq_file points to a live seq_file, so we can lift it
    let seq_file = unsafe { SeqFile::from_raw(seq) };
    seq_print!(seq_file, "{}", data);
    0
}

// Work around lack of generic const items
trait DisplayFile: Display + Sized {
    const VTABLE: kernel::bindings::file_operations = kernel::bindings::file_operations {
        read: Some(kernel::bindings::seq_read),
        llseek: Some(kernel::bindings::seq_lseek),
        release: Some(kernel::bindings::single_release),
        open: Some(display_open::<Self> as _),
        // SAFETY: file_operations supports zeroes in all fields
        ..unsafe { core::mem::zeroed() }
    };
}
impl<T: Display + Sized> DisplayFile for T {}

const fn get_vtable<T: DisplayFile>(_: &T) -> &'static kernel::bindings::file_operations {
    &T::VTABLE
}

/// A Dir, scoped to the lifetime for which it will exist. Unlike `&'a Dir`, this is equivariant,
/// preventing the shortening of the lifetime.
///
/// # Invariants
/// Builder will only ever be used with 'static or a universally quantified lifetime that is
/// unified only with the lifetime of data structures guaranteed to outlive it and not have mutable
/// references taken.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Builder<'a> {
    inner: &'a Dir,
    _equivariant: PhantomData<fn(&'a ()) -> &'a ()>,
}

impl<'a> Builder<'a> {
    /// # Safety
    /// Caller must promise to use this function at static lifetime or only expose it to
    /// universally quantified functions.
    unsafe fn new(inner: &'a Dir) -> Self {
        Self {
            inner,
            _equivariant: PhantomData,
        }
    }
}

impl<'a> Deref for Builder<'a> {
    type Target = Dir;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> Builder<'a> {
    //    /// Create a file in a DebugFS directory with the provided name, and contents from invoking the
    //    /// fomatter on the attached data. The attached function must be a ZST, and will cause a
    //    /// compilation error if it is not.
    //    pub fn fmt_file<F: Fn(&'a T) -> Arguments<'a>>(&self, name: &CStr, data: &'a T, f: F) -> Result<()> {

    /// Create a file in a DebugFS directory with the provided name, and contents
    /// corresponding to the Display output for data with a trailing newline.
    pub fn display_file<T: Display + 'static>(&self, name: &CStr, data: &'a T) -> Result<()> {
        fn display_newline<T: Display>(data: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(fmt, "{}\n", data)
        }
        self.fmt_file(name, data, &display_newline)
    }

    /// Create a file in a DebugFS directory with the provided name, and contents from invoking the
    /// fomatter on the attached data. The attached function must be a ZST, and will cause a
    /// compilation error if it is not.
    pub fn fmt_file<T, F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result>(
        &self,
        name: &CStr,
        data: &'a T,
        f: &'static F,
    ) -> Result<()> {
        let data_adapted = FormatAdapter::new(data, f);
        // We let the pointer go out of scope because:
        // 1. We don't have any abstractions for anything the user could do with the dentry
        //    corresponding to the created file.
        // 2. The correct type for this is ARef<dentry>, but this would cause the usual idiom of
        //    calling this function and not tracking the result to require explicit forgets. The
        //    cleanup will be handled by the recursive remove performed on the DebugFS dir created
        //    at the root.
        // SAFETY:
        // * name is a NUL-terminated string, per &CStr invariants
        // * mode is all-readable, a valid mode combo
        // * parent is a live debugfs dentry, as our own type is a wrapper around that
        // * data is a pointer to a T, which will only be used immutably by the vtable
        // * vtable is all stock seq_file implementations except for open. open's only requirement
        //   beyond what is provided to all open functions is that the inode's data pointer must
        //   point to a T that will outlive it, which we know because our wrapper struct forces
        //   lifetime equivariance.
        let _ptr = from_err_ptr(unsafe {
            kernel::bindings::debugfs_create_file_full(
                name.as_char_ptr(),
                0444,
                self.as_ptr(),
                data_adapted as *const _ as *mut _,
                core::ptr::null(),
                get_vtable(data_adapted),
            )
        })?;
        Ok(())
    }

    /// Creates a nested directory that may live as long as its parent
    pub fn dir(&self, name: &CStr) -> Result<Builder<'a>> {
        // SAFETY:
        // * name is a NUL-terminated string, per &CStr invariants
        // * parent is a valid directory pointer, as we are a wrapper around dentry.
        let dir = from_err_ptr(unsafe {
            kernel::bindings::debugfs_create_dir(name.as_char_ptr(), self.as_ptr())
        })?;
        // We intentionally don't create an ARef even though we technically have a reference here
        // because:
        // 1. Since we're inside a Builder, we know that our transitive parent is eventually inside
        //    a Values, which has a PinDrop impl that will clean up after us via a recursive
        //    removal.
        // 2. We need the equivariance provided by `Builder` to make the new directory bound to the
        //    same lifetime as the values they're going to try to use it with.
        // SAFETY: The dentry pointer returned by debugfs_create_dir is an owning pointer if it
        // wasn't an error, and we checked the error status. This means it's live until released.
        // We don't release it explicitly ever, instead only releasing it implicitly when the
        // parent directory is released. This means the directory lives as long as the parent
        // directory, and so we're matching the universal quantification requirement.
        Ok(unsafe { Self::new(&*dir.cast()) })
    }
}

#[pin_data(PinnedDrop)]
/// A DebugFS directory combined with a backing store for data to implement it
pub struct Values<T> {
    #[pin]
    backing: T,
    dir: ARef<Dir>,
    // Since the files present under our directory may point into backing, we are !Unpin
    #[pin]
    _pin: PhantomPinned,
}

impl<T> Values<T> {
    /// Attach backing data to a DebugFS directory. When the resulting object is destroyed, the
    /// DebugFS directory will be recursively removed as well.
    pub fn attach(backing: impl PinInit<T, Error>, dir: ARef<Dir>) -> impl PinInit<Self, Error> {
        try_pin_init! { Self {
            backing <- backing,
            dir: dir,
            _pin: PhantomPinned,
        }}
    }

    /// Runs a closure which has access to the backing data and a builder that will allow you to
    /// build a DebugFS structure off the backing data using its methods.
    pub fn build<U, F: for<'a> FnOnce(&'a T, Builder<'a>) -> U>(self: Pin<&Self>, f: F) -> U {
        // SAFETY: The Builder produced here is technically at the lifetime of self, but is
        // being used only in a universal context, so that information is immediately erased and
        // replaced with the universally quantified 'a. By taking a Pin<&Self>, we enforce that
        // self.backing remains alive for any 'a less than the lifetime of the struct. By not
        // providing any mutable access to self.backing, we ensure that it's always safe to
        // materialize a read-only reference to &self.backing for any 'a less than the lifetime of
        // the struct.
        f(&self.backing, unsafe { Builder::new(&self.dir) })
    }
}

#[pinned_drop]
impl<T> PinnedDrop for Values<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe { kernel::bindings::debugfs_remove(self.dir.as_ptr()) }
    }
}

// INVARIANT: F is inhabited
#[repr(transparent)]
struct FormatAdapter<T, F> {
    inner: T,
    _formatter: PhantomData<F>,
}

impl<T, F> FormatAdapter<T, F> {
    fn new<'a>(inner: &'a T, _f: &'static F) -> &'a Self {
        // SAFETY: FormatAdapater is a repr(transparent) wrapper around T, so
        // casting a reference is legal
        // INVARIANT: We were passed a reference to F, so it is inhabited.
        unsafe { core::mem::transmute(inner) }
    }
}

impl<T, F: Fn(&T, &mut Formatter<'_>) -> fmt::Result + 'static> Display for FormatAdapter<T, F> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let f: &F = unsafe { materialize_zst_fmt() };
        f(&self.inner, fmt)
    }
}

/// # Safety
/// The caller asserts that F is inhabited
unsafe fn materialize_zst_fmt<F>() -> &'static F {
    // We don't have generic_const_exprs, and const items inside the function get promoted out and
    // lose type variables, so we need to do the old-style assert to check for ZSTness
    [(); 1][core::mem::size_of::<F>()];
    let zst_dangle: NonNull<F> = NonNull::dangling();
    // SAFETY:
    // While the pointer is dangling, it is a dangling pointer to a ZST, based on the array
    // assertion above. The type is also inhabited, by the caller's assertion. This means
    // we can materialize it.
    unsafe { zst_dangle.as_ref() }
}

#[macro_export]
/// Allows defining a debugfs file with a format string directly
macro_rules! debugfs_fmt_file {
    ($dir:expr, $name:expr, $data:expr, $binding:ident, $($arg:tt),*) => {
        $dir.fmt_file($name, $data, &|$binding, fmt| write!(fmt, $($arg),*))
    }
}
