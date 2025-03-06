// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2024 Google LLC.

//! Files and file descriptors.
//!
//! C headers: [`include/linux/fs.h`](srctree/include/linux/fs.h) and
//! [`include/linux/file.h`](srctree/include/linux/file.h)
//!
//! A C `struct file *` can indicate many things. The ones we currently model are:
//! * `File<()>` - A file with unknown or don't-care private data
//! * `P: ForeignOwned`, `File<P>` - A file with known private data which owns its private data.
//! * `File<Outlives<T>>` - A file with known private data, where the private data is known to
//!   outlive the file.
//! * `LocalFile<P>` - A degraded version of `File` which is not `Sync` or `Send`. This gives no
//!   additional powers relative to `File`, but helps to correctly respect `fdget_pos`
//!   optimizations.
//! * `InitFile<'a, P>` - A unique pointer to a file, borrrowed from another source. This
//!   type is provided in order to handle circumstances where code is setting up the file, but does
//!   not have the privilege to discard it. This type allows *mutation*, including changing the
//!   type of the `private_data` field. The common use case for this is implementing `open`,
//!   which will provide a file pointer that is guaranteed not in use by anyone else for
//!   initialization. The caller of the `open` implementation expects this to always remain valid
//!   after the call. This type is *not* `AlwaysRefCounted` to preserve its uniqueness.
//!   TODO edit summary
//! * `RawFile` - A basic file that does not know its private data, is not send or sync, and does
//!   not implement `AlwaysRefCounted`. All other files can degrade to this type to access basic
//!   methods present on all file-like objects.

use crate::ffi::c_void;
use crate::{
    bindings,
    cred::Credential,
    error::{code::*, Error, Result},
    types::{ARef, AlwaysRefCounted, NotThreadSafe, ForeignOwnable, Opaque},
};
use core::ptr;

pub mod operations;

/// Flags associated with a [`File`].
pub mod flags {
    /// File is opened in append mode.
    pub const O_APPEND: u32 = bindings::O_APPEND;

    /// Signal-driven I/O is enabled.
    pub const O_ASYNC: u32 = bindings::FASYNC;

    /// Close-on-exec flag is set.
    pub const O_CLOEXEC: u32 = bindings::O_CLOEXEC;

    /// File was created if it didn't already exist.
    pub const O_CREAT: u32 = bindings::O_CREAT;

    /// Direct I/O is enabled for this file.
    pub const O_DIRECT: u32 = bindings::O_DIRECT;

    /// File must be a directory.
    pub const O_DIRECTORY: u32 = bindings::O_DIRECTORY;

    /// Like [`O_SYNC`] except metadata is not synced.
    pub const O_DSYNC: u32 = bindings::O_DSYNC;

    /// Ensure that this file is created with the `open(2)` call.
    pub const O_EXCL: u32 = bindings::O_EXCL;

    /// Large file size enabled (`off64_t` over `off_t`).
    pub const O_LARGEFILE: u32 = bindings::O_LARGEFILE;

    /// Do not update the file last access time.
    pub const O_NOATIME: u32 = bindings::O_NOATIME;

    /// File should not be used as process's controlling terminal.
    pub const O_NOCTTY: u32 = bindings::O_NOCTTY;

    /// If basename of path is a symbolic link, fail open.
    pub const O_NOFOLLOW: u32 = bindings::O_NOFOLLOW;

    /// File is using nonblocking I/O.
    pub const O_NONBLOCK: u32 = bindings::O_NONBLOCK;

    /// File is using nonblocking I/O.
    ///
    /// This is effectively the same flag as [`O_NONBLOCK`] on all architectures
    /// except SPARC64.
    pub const O_NDELAY: u32 = bindings::O_NDELAY;

    /// Used to obtain a path file descriptor.
    pub const O_PATH: u32 = bindings::O_PATH;

    /// Write operations on this file will flush data and metadata.
    pub const O_SYNC: u32 = bindings::O_SYNC;

    /// This file is an unnamed temporary regular file.
    pub const O_TMPFILE: u32 = bindings::O_TMPFILE;

    /// File should be truncated to length 0.
    pub const O_TRUNC: u32 = bindings::O_TRUNC;

    /// Bitmask for access mode flags.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::fs::file;
    /// # fn do_something() {}
    /// # let flags = 0;
    /// if (flags & file::flags::O_ACCMODE) == file::flags::O_RDONLY {
    ///     do_something();
    /// }
    /// ```
    pub const O_ACCMODE: u32 = bindings::O_ACCMODE;

    /// File is read only.
    pub const O_RDONLY: u32 = bindings::O_RDONLY;

    /// File is write only.
    pub const O_WRONLY: u32 = bindings::O_WRONLY;

    /// File can be both read and written.
    pub const O_RDWR: u32 = bindings::O_RDWR;
}

/// Wraps the kernel's `struct file`. Thread safe.
///
/// This represents an open file rather than a file on a filesystem. Processes generally reference
/// open files using file descriptors. However, file descriptors are not the same as files. A file
/// descriptor is just an integer that corresponds to a file, and a single file may be referenced
/// by multiple file descriptors.
///
/// # Refcounting
///
/// Instances of this type are reference-counted. The reference count is incremented by the
/// `fget`/`get_file` functions and decremented by `fput`. The Rust type `ARef<File>` represents a
/// pointer that owns a reference count on the file.
///
/// Whenever a process opens a file descriptor (fd), it stores a pointer to the file in its fd
/// table (`struct files_struct`). This pointer owns a reference count to the file, ensuring the
/// file isn't prematurely deleted while the file descriptor is open. In Rust terminology, the
/// pointers in `struct files_struct` are `ARef<File>` pointers.
///
/// ## Light refcounts
///
/// Whenever a process has an fd to a file, it may use something called a "light refcount" as a
/// performance optimization. Light refcounts are acquired by calling `fdget` and released with
/// `fdput`. The idea behind light refcounts is that if the fd is not closed between the calls to
/// `fdget` and `fdput`, then the refcount cannot hit zero during that time, as the `struct
/// files_struct` holds a reference until the fd is closed. This means that it's safe to access the
/// file even if `fdget` does not increment the refcount.
///
/// The requirement that the fd is not closed during a light refcount applies globally across all
/// threads - not just on the thread using the light refcount. For this reason, light refcounts are
/// only used when the `struct files_struct` is not shared with other threads, since this ensures
/// that other unrelated threads cannot suddenly start using the fd and close it. Therefore,
/// calling `fdget` on a shared `struct files_struct` creates a normal refcount instead of a light
/// refcount.
///
/// Light reference counts must be released with `fdput` before the system call returns to
/// userspace. This means that if you wait until the current system call returns to userspace, then
/// all light refcounts that existed at the time have gone away.
///
/// ### The file position
///
/// Each `struct file` has a position integer, which is protected by the `f_pos_lock` mutex.
/// However, if the `struct file` is not shared, then the kernel may avoid taking the lock as a
/// performance optimization.
///
/// The condition for avoiding the `f_pos_lock` mutex is different from the condition for using
/// `fdget`. With `fdget`, you may avoid incrementing the refcount as long as the current fd table
/// is not shared; it is okay if there are other fd tables that also reference the same `struct
/// file`. However, `fdget_pos` can only avoid taking the `f_pos_lock` if the entire `struct file`
/// is not shared, as different processes with an fd to the same `struct file` share the same
/// position.
///
/// To represent files that are not thread safe due to this optimization, the [`LocalFile`] type is
/// used.
///
/// ## Rust references
///
/// The reference type `&File` is similar to light refcounts:
///
/// * `&File` references don't own a reference count. They can only exist as long as the reference
///   count stays positive, and can only be created when there is some mechanism in place to ensure
///   this.
///
/// * The Rust borrow-checker normally ensures this by enforcing that the `ARef<File>` from which
///   a `&File` is created outlives the `&File`.
///
/// * Using the unsafe [`File::from_raw_file`] means that it is up to the caller to ensure that the
///   `&File` only exists while the reference count is positive.
///
/// * You can think of `fdget` as using an fd to look up an `ARef<File>` in the `struct
///   files_struct` and create an `&File` from it. The "fd cannot be closed" rule is like the Rust
///   rule "the `ARef<File>` must outlive the `&File`".
///
/// # Invariants
///
/// * All instances of this type are refcounted using the `f_count` field.
/// * There must not be any active calls to `fdget_pos` on this file that did not take the
///   `f_pos_lock` mutex.
/// * P must either be `()` (unknown/unmodeled) or a type for which `private_data` is compatible
///   with the representation used by `<P as ForeignOwnable>`.
#[repr(transparent)]
pub struct File<P> {
    inner: Opaque<bindings::file>,
    phantom: core::marker::PhantomData<P>,
}

/// Wrap the target type in this if it's a pointer to a data structure which is guaranteed to
/// outlive the file itself, e.g. driver static data
pub struct Outlives<T>(core::marker::PhantomData<*const T>);

mod sealed {
    pub trait Sealed {}
}
/// Trait for types representing a reference which outlives the dynamic scope of the file
pub trait OutlivesRef: sealed::Sealed {
    /// Target this derefs to
    type Target;
}
impl<T> sealed::Sealed for Outlives<T> {}
impl<T> OutlivesRef for Outlives<T> {
    type Target = T;
}

// SAFETY: This file is known to not have any active `fdget_pos` calls that did not take the
// `f_pos_lock` mutex, so it is safe to transfer it between threads.
unsafe impl<P: Send> Send for File<P> {}

// SAFETY: This file is known to not have any active `fdget_pos` calls that did not take the
// `f_pos_lock` mutex, so it is safe to access its methods from several threads in parallel.
unsafe impl<P: Sync> Sync for File<P> {}

// SAFETY: The type invariants guarantee that `File` is always ref-counted. This implementation
// makes `ARef<File>` own a normal refcount.
unsafe impl<P> AlwaysRefCounted for File<P> {
    #[inline]
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { bindings::get_file(self.as_ptr()) };
    }

    #[inline]
    unsafe fn dec_ref(obj: ptr::NonNull<Self>) {
        // SAFETY: To call this method, the caller passes us ownership of a normal refcount, so we
        // may drop it. The cast is okay since `File` has the same representation as `struct file`.
        unsafe { bindings::fput(obj.cast().as_ptr()) }
    }
}

/// Wraps the kernel's `struct file`. Not thread safe.
///
/// This type represents a file that is not known to be safe to transfer across thread boundaries.
/// To obtain a thread-safe [`File`], use the [`assume_no_fdget_pos`] conversion.
///
/// See the documentation for [`File`] for more information.
///
/// # Invariants
///
/// * All instances of this type are refcounted using the `f_count` field.
/// * If there is an active call to `fdget_pos` that did not take the `f_pos_lock` mutex, then it
///   must be on the same thread as this file.
/// * P must either be `()` (unknown/unmodeled) or a type for which `private_data` is compatible
///   with the representation used by `<P as ForeignOwnable>`
///
/// [`assume_no_fdget_pos`]: LocalFile::assume_no_fdget_pos
pub struct LocalFile<P> {
    // The casts to convert this to a RawFile implicitly use this field.
    #[allow(dead_code)]
    inner: Opaque<bindings::file>,
    phantom: core::marker::PhantomData<P>,
}

// SAFETY: The type invariants guarantee that `LocalFile` is always ref-counted. This implementation
// makes `ARef<File>` own a normal refcount.
unsafe impl<P> AlwaysRefCounted for LocalFile<P> {
    #[inline]
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { bindings::get_file(self.as_ptr()) };
    }

    #[inline]
    unsafe fn dec_ref(obj: ptr::NonNull<Self>) {
        // SAFETY: To call this method, the caller passes us ownership of a normal refcount, so we
        // may drop it. The cast is okay since `File` has the same representation as `struct file`.
        unsafe { bindings::fput(obj.cast().as_ptr()) }
    }
}

impl<P> LocalFile<P> {
    /// Constructs a new `struct file` wrapper from a file descriptor.
    ///
    /// The file descriptor belongs to the current process, and there might be active local calls
    /// to `fdget_pos` on the same file.
    ///
    /// To obtain an `ARef<File>`, use the [`assume_no_fdget_pos`] function to convert.
    ///
    /// [`assume_no_fdget_pos`]: LocalFile::assume_no_fdget_pos
    #[inline]
    pub fn fget(fd: u32) -> Result<ARef<LocalFile<P>>, BadFdError> {
        // SAFETY: FFI call, there are no requirements on `fd`.
        let ptr = ptr::NonNull::new(unsafe { bindings::fget(fd) }).ok_or(BadFdError)?;

        // SAFETY: `bindings::fget` created a refcount, and we pass ownership of it to the `ARef`.
        //
        // INVARIANT: This file is in the fd table on this thread, so either all `fdget_pos` calls
        // are on this thread, or the file is shared, in which case `fdget_pos` calls took the
        // `f_pos_lock` mutex.
        Ok(unsafe { ARef::from_raw(ptr.cast()) })
    }

    /// Creates a reference to a [`LocalFile`] from a valid pointer.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that `ptr` points at a valid file and that the file's refcount is
    ///   positive for the duration of 'a.
    /// * The caller must ensure that if there is an active call to `fdget_pos` that did not take
    ///   the `f_pos_lock` mutex, then that call is on the current thread.
    /// * The caller must ensure that `P` is the type pointed to by the file's `private_data` or
    ///   `()`.
    #[inline]
    pub unsafe fn from_raw_file<'a>(ptr: *const bindings::file) -> &'a LocalFile<P> {
        // SAFETY: The caller guarantees that the pointer is not dangling and stays valid for the
        // duration of 'a. The cast is okay because `File` is `repr(transparent)`.
        //
        // INVARIANT: The caller guarantees that there are no problematic `fdget_pos` calls.
        // INVARIANT: The caller guarantees that `P` matches `private_data`'s pointee or is `()`
        unsafe { &*ptr.cast() }
    }

    /// Assume that there are no active `fdget_pos` calls that prevent us from sharing this file.
    ///
    /// This makes it safe to transfer this file to other threads. No checks are performed, and
    /// using it incorrectly may lead to a data race on the file position if the file is shared
    /// with another thread.
    ///
    /// This method is intended to be used together with [`LocalFile::fget`] when the caller knows
    /// statically that there are no `fdget_pos` calls on the current thread. For example, you
    /// might use it when calling `fget` from an ioctl, since ioctls usually do not touch the file
    /// position.
    ///
    /// # Safety
    ///
    /// There must not be any active `fdget_pos` calls on the current thread.
    #[inline]
    pub unsafe fn assume_no_fdget_pos(me: ARef<Self>) -> ARef<File<P>> {
        // INVARIANT: There are no `fdget_pos` calls on the current thread, and by the type
        // invariants, if there is a `fdget_pos` call on another thread, then it took the
        // `f_pos_lock` mutex.
        //
        // SAFETY: `LocalFile` and `File` have the same layout.
        unsafe { ARef::from_raw(ARef::into_raw(me).cast()) }
    }
}

/// Non-owning file pointer with uniqueness (e.g. can't clone out of it, don't make one unless
/// you've got limit 1 and are giving up your own)
#[repr(transparent)]
pub struct InitFile<'a, P> {
    inner: &'a mut Opaque<bindings::file>,
    phantom: core::marker::PhantomData<P>,
}

impl<'a, P> InitFile<'a, P> {
    /// Solemnly swear you are the only owner of this file, and create an initializer with the
    /// current existing type.
    /// # Safety
    /// * You must be the only owner of this file.
    /// * You must meant the LocalFile from_raw_file reqs.
    pub unsafe fn from_raw_file(ptr: *mut bindings::file) -> Self {
        // Safety: TODO
        InitFile {inner: unsafe { &mut *ptr.cast() }, phantom: core::marker::PhantomData}
    }

    /// We're done initing, convert to a full object
    pub fn init(self) -> &'a File<P> {
        // Safety:
        // Since we take `self` by ownership, all outstanding borrows/edits have already happened.
        // Since we were the sole owner of the file, and we don't degrade to a `LocalFile`, no
        // unlocked `fdget_pos` should have been allowed.
        unsafe { File::from_raw_file(self.inner as *mut _ as *const _) }
    }

    /// Convenience access to private data field
    #[inline]
    unsafe fn set_raw_private_data(&mut self, private_data: *mut c_void) {
        // Safety TODO
        unsafe { (*self.as_ptr()).private_data = private_data }
    }
}

impl <'a, P: ForeignOwnable> InitFile<'a, P> {
    /// Replace an owned or () private_data with a new owned value
    pub fn set_private<Q: ForeignOwnable>(mut self, val: Q) -> InitFile<'a, Q> {
        drop(unsafe { self.take_private_data() });
        unsafe { self.set_raw_private_data(val.into_foreign()) };
        unsafe { core::mem::transmute(self) }
    }
    /// Replace an owned or () private_data with a reference that outlives the
    /// file.
    pub fn set_private_outlives<Q: OutlivesRef>(mut self, val: &'static Q::Target) -> InitFile<'a, Q> {
        drop(unsafe { self.take_private_data() });
        unsafe { self.set_raw_private_data(val as *const _ as *const c_void as *mut c_void) };
        unsafe { core::mem::transmute(self) }
    }
}

impl <'a, P: OutlivesRef> InitFile<'a, P> {
    /// Replace an outliving reference with owned private data
    pub fn overwrite_private<Q: ForeignOwnable>(mut self, val: Q) -> InitFile<'a, Q> {
        unsafe { self.set_raw_private_data(val.into_foreign()) };
        unsafe { core::mem::transmute(self) }
    }
    /// Replace an outliving reference with a new one
    pub fn overwrite_private_outlives<Q: OutlivesRef>(mut self, val: &'static Q::Target) -> InitFile<'a, Q> {
        unsafe { self.set_raw_private_data(val as *const _ as *const c_void as *mut c_void) };
        unsafe { core::mem::transmute(self) }
    }
}

/// RawFile is a wrapped `struct file *` with no access to refcounts or Sync/Send properties
/// You probably don't want to use this.
// TODO consider moving these functions into a trait that all File types implement through a
// blanket deref impl?
#[repr(transparent)]
pub struct RawFile<P> {
    inner: Opaque<bindings::file>,
    phantom: core::marker::PhantomData<P>,
}

// Make RawFile methods available to File and LocalFile
impl<P> core::ops::Deref for LocalFile<P> {
    type Target = RawFile<P>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: The caller provides a `&File`, and since it is a reference, it must point at a
        // valid file for the desired duration.
        //
        // By the type invariants, there are no `fdget_pos` calls that did not take the
        // `f_pos_lock` mutex.
        //
        // By the type invariants, we already have an appropriate `P`.
        unsafe { RawFile::from_raw_file(self as *const Self as *const bindings::file) }
    }
}

// Make RawFile methods available to InitFile
impl<'a, P> core::ops::Deref for InitFile<'a, P> {
    type Target = RawFile<P>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: The caller provides a `&File`, and since it is a reference, it must point at a
        // valid file for the desired duration.
        //
        // By the type invariants, there are no `fdget_pos` calls that did not take the
        // `f_pos_lock` mutex.
        //
        // By the type invariants, we already have an appropriate `P`.
        unsafe { RawFile::from_raw_file(self as *const Self as *const bindings::file) }
    }
}

impl<P> RawFile<P> {
    /// Creates a reference to a [`RawFile`] from a valid pointer.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that `ptr` points at a valid file and that the file's refcount is
    ///   positive for the duration of 'a.
    /// * The caller must ensure that if there is an active call to `fdget_pos` that did not take
    ///   the `f_pos_lock` mutex, then that call is on the current thread.
    #[inline]
    pub unsafe fn from_raw_file<'a>(ptr: *const bindings::file) -> &'a Self {
        // SAFETY: The caller guarantees that the pointer is not dangling and stays valid for the
        // duration of 'a. The cast is okay because `File` is `repr(transparent)`.
        //
        // INVARIANT: The caller guarantees that there are no problematic `fdget_pos` calls.
        // INVARIANT: The caller guarantees that `P` matches `private_data`'s pointee or is `()`
        unsafe { &*ptr.cast() }
    }


    /// Returns a raw pointer to the inner C struct.
    #[inline]
    pub fn as_ptr(&self) -> *mut bindings::file {
        self.inner.get()
    }

    /// Returns the credentials of the task that originally opened the file.
    pub fn cred(&self) -> &Credential {
        // SAFETY: It's okay to read the `f_cred` field without synchronization because `f_cred` is
        // never changed after initialization of the file.
        let ptr = unsafe { (*self.as_ptr()).f_cred };

        // SAFETY: The signature of this function ensures that the caller will only access the
        // returned credential while the file is still valid, and the C side ensures that the
        // credential stays valid at least as long as the file.
        unsafe { Credential::from_ptr(ptr) }
    }

    /// Returns the flags associated with the file.
    ///
    /// The flags are a combination of the constants in [`flags`].
    #[inline]
    pub fn flags(&self) -> u32 {
        // This `read_volatile` is intended to correspond to a READ_ONCE call.
        //
        // SAFETY: The file is valid because the shared reference guarantees a nonzero refcount.
        //
        // FIXME(read_once): Replace with `read_once` when available on the Rust side.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).f_flags).read_volatile() }
    }

    /// Convenience access to private data field
    #[inline]
    fn raw_private_data(&self) -> *mut c_void {
        // Safety TODO
        unsafe { (*self.as_ptr()).private_data }
    }
}

impl<P: ForeignOwnable> RawFile<P> {
    /// Borrow private data
    #[inline]
    pub fn private_data<'a>(&'a self) -> P::Borrowed<'a> {
        unsafe { P::borrow(self.raw_private_data()) }
    }
    /// Take private data
    #[inline]
    pub unsafe fn take_private_data<'a>(&'a self) -> P {
        unsafe { P::from_foreign( self.raw_private_data()) }
    }
}

impl<P: OutlivesRef> RawFile<P> {
    #[inline]
    /// Borrow the private data
    pub fn private_data_outlives(&self) -> &P::Target {
        unsafe { (*self.as_ptr()).private_data.cast::<P::Target>().as_ref().unwrap_unchecked() }
    }
}


impl<P> File<P> {
    /// Creates a reference to a [`File`] from a valid pointer.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that `ptr` points at a valid file and that the file's refcount is
    ///   positive for the duration of 'a.
    /// * The caller must ensure that if there are active `fdget_pos` calls on this file, then they
    ///   took the `f_pos_lock` mutex.
    /// * The caller must ensure that `P` is the type pointed to by the file's `private_data` or
    ///   `()`.
    #[inline]
    pub unsafe fn from_raw_file<'a>(ptr: *const bindings::file) -> &'a Self {
        // SAFETY: The caller guarantees that the pointer is not dangling and stays valid for the
        // duration of 'a. The cast is okay because `File` is `repr(transparent)`.
        //
        // INVARIANT: The caller guarantees that there are no problematic `fdget_pos` calls.
        // INVARIANT: The caller guarantees that `P` matches `private_data`'s pointee or is `()`
        unsafe { &*ptr.cast() }
    }
}

// Allow methods which only require a `&LocalFile` to accept a `&File`.
impl<P> core::ops::Deref for File<P> {
    type Target = LocalFile<P>;
    #[inline]
    fn deref(&self) -> &LocalFile<P> {
        // SAFETY: The caller provides a `&File`, and since it is a reference, it must point at a
        // valid file for the desired duration.
        //
        // By the type invariants, there are no `fdget_pos` calls that did not take the
        // `f_pos_lock` mutex.
        //
        // By the type invariants, we already have an appropriate `P`.
        unsafe { LocalFile::from_raw_file(self as *const File<P> as *const bindings::file) }
    }
}

/// A file descriptor reservation.
///
/// This allows the creation of a file descriptor in two steps: first, we reserve a slot for it,
/// then we commit or drop the reservation. The first step may fail (e.g., the current process ran
/// out of available slots), but commit and drop never fail (and are mutually exclusive).
///
/// Dropping the reservation happens in the destructor of this type.
///
/// # Invariants
///
/// The fd stored in this struct must correspond to a reserved file descriptor of the current task.
pub struct FileDescriptorReservation {
    fd: u32,
    /// Prevent values of this type from being moved to a different task.
    ///
    /// The `fd_install` and `put_unused_fd` functions assume that the value of `current` is
    /// unchanged since the call to `get_unused_fd_flags`. By adding this marker to this type, we
    /// prevent it from being moved across task boundaries, which ensures that `current` does not
    /// change while this value exists.
    _not_send: NotThreadSafe,
}

impl FileDescriptorReservation {
    /// Creates a new file descriptor reservation.
    pub fn get_unused_fd_flags(flags: u32) -> Result<Self> {
        // SAFETY: FFI call, there are no safety requirements on `flags`.
        let fd: i32 = unsafe { bindings::get_unused_fd_flags(flags) };
        if fd < 0 {
            return Err(Error::from_errno(fd));
        }
        Ok(Self {
            fd: fd as u32,
            _not_send: NotThreadSafe,
        })
    }

    /// Returns the file descriptor number that was reserved.
    pub fn reserved_fd(&self) -> u32 {
        self.fd
    }

    /// Commits the reservation.
    ///
    /// The previously reserved file descriptor is bound to `file`. This method consumes the
    /// [`FileDescriptorReservation`], so it will not be usable after this call.
    pub fn fd_install<P>(self, file: ARef<File<P>>) {
        // SAFETY: `self.fd` was previously returned by `get_unused_fd_flags`. We have not yet used
        // the fd, so it is still valid, and `current` still refers to the same task, as this type
        // cannot be moved across task boundaries.
        //
        // Furthermore, the file pointer is guaranteed to own a refcount by its type invariants,
        // and we take ownership of that refcount by not running the destructor below.
        // Additionally, the file is known to not have any non-shared `fdget_pos` calls, so even if
        // this process starts using the file position, this will not result in a data race on the
        // file position.
        unsafe { bindings::fd_install(self.fd, file.as_ptr()) };

        // `fd_install` consumes both the file descriptor and the file reference, so we cannot run
        // the destructors.
        core::mem::forget(self);
        core::mem::forget(file);
    }
}

impl Drop for FileDescriptorReservation {
    fn drop(&mut self) {
        // SAFETY: By the type invariants of this type, `self.fd` was previously returned by
        // `get_unused_fd_flags`. We have not yet used the fd, so it is still valid, and `current`
        // still refers to the same task, as this type cannot be moved across task boundaries.
        unsafe { bindings::put_unused_fd(self.fd) };
    }
}

/// Represents the `EBADF` error code.
///
/// Used for methods that can only fail with `EBADF`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BadFdError;

impl From<BadFdError> for Error {
    #[inline]
    fn from(_: BadFdError) -> Error {
        EBADF
    }
}

impl core::fmt::Debug for BadFdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad("EBADF")
    }
}
