//! TODO debugfs docs
use crate::error::from_err_ptr;
use crate::seq_file::SeqFile;
use crate::types::ARef;
use crate::types::AlwaysRefCounted;
use crate::types::Opaque;
use core::ptr::NonNull;
use kernel::prelude::*;

/// TODO Doc
pub struct DebugfsDir {
    inner: Opaque<bindings::dentry>,
}

// TODO safety
unsafe impl Send for DebugfsDir {}

// TODO safety
unsafe impl Sync for DebugfsDir {}

// TODO safety
unsafe impl AlwaysRefCounted for DebugfsDir {
    #[inline]
    fn inc_ref(&self) {
        // TODO safety
        unsafe {
            kernel::bindings::dget(self.as_ptr());
        }
    }
    #[inline]
    unsafe fn dec_ref(obj: NonNull<Self>) {
        // TODO safety
        unsafe {
            kernel::bindings::dput(obj.cast().as_ptr());
        }
    }
}

// TODO better umode living in the right place
#[allow(non_camel_case_types)]
type umode_t = u16;

/// Types with a known way to register as a debug value with the kernel
/// TODO better
pub trait DebugValue {
    /// The existing native registration
    unsafe extern "C" fn register(
        name: *const kernel::ffi::c_char,
        mode: umode_t,
        dentry: *mut kernel::bindings::dentry,
        value: *mut Self,
    );
}

/// u32 but hex for value format
#[repr(transparent)]
pub struct X32(u32);

impl X32 {
    /// X32 has same layout as u32, this lets you cast
    pub fn from_u32_ref(v: &u32) -> &Self {
        // TODO SAFETY
        unsafe { (v as *const _ as *const Self).as_ref().unwrap_unchecked() }
    }
}

impl DebugValue for u32 {
    unsafe extern "C" fn register(
        name: *const kernel::ffi::c_char,
        mode: umode_t,
        dentry: *mut kernel::bindings::dentry,
        value: *mut Self,
    ) {
        // TODO SAFETY
        unsafe { kernel::bindings::debugfs_create_u32(name, mode, dentry, value) };
    }
}

impl DebugValue for X32 {
    unsafe extern "C" fn register(
        name: *const kernel::ffi::c_char,
        mode: umode_t,
        dentry: *mut kernel::bindings::dentry,
        value: *mut Self,
    ) {
        // TODO SAFETY
        unsafe { kernel::bindings::debugfs_create_x32(name, mode, dentry, value as _) };
    }
}

impl DebugfsDir {
    /// TODO Doc
    pub fn new(name: &CStr) -> Result<ARef<Self>> {
        // TODO safety
        let dir = NonNull::new(from_err_ptr(unsafe {
            kernel::bindings::debugfs_create_dir(name.as_char_ptr(), core::ptr::null_mut())
        })?);
        // TODO safety
        Ok(unsafe { ARef::from_raw(dir.unwrap_unchecked().cast()) })
    }
    /// TODO Doc
    // TODO lifetime'd value?
    pub fn value_file<T: DebugValue>(&self, name: &CStr, value: &'static T) {
        // TODO safety
        unsafe {
            T::register(
                name.as_char_ptr(),
                0444,
                self.as_ptr(),
                value as *const _ as *mut _,
            )
        }
    }
    fn as_ptr(&self) -> *mut bindings::dentry {
        self.inner.get()
    }
}

/// A debugfs wrapper for universal restrictions
pub struct DebugfsBuilder<'a>(&'a DebugfsDir);

impl<'a> core::ops::Deref for DebugfsBuilder<'a> {
    type Target = DebugfsDir;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe extern "C" fn print_open<T: Printer>(
    inode: *mut kernel::bindings::inode,
    file: *mut kernel::bindings::file,
) -> i32 {
    unsafe { kernel::bindings::single_open(file, Some(print_act::<T>), (*inode).i_private) }
}

unsafe extern "C" fn print_act<T: Printer>(
    seq: *mut kernel::bindings::seq_file,
    _: *mut core::ffi::c_void,
) -> i32 {
    // TODO safety
    let data = unsafe { &*((*seq).private as *mut T) };
    // TODO safety
    let seq_file = unsafe { SeqFile::from_raw(seq) };
    data.print(seq_file)
}

/// TODO name + docs
// Sized needs to be here because we use a C pointer to hold the &Self during single_open.
// This means it can't be a wide ref, so it needs to be sized.
pub trait Printer: Sized {
    /// Write desired contents to seq_file
    // TODO switch from i32 to Err, convert err
    fn print(&self, seq_file: &SeqFile) -> i32;
    /// Provided vtable
    const VTABLE: kernel::bindings::file_operations = kernel::bindings::file_operations {
        read: Some(kernel::bindings::seq_read),
        llseek: Some(kernel::bindings::seq_lseek),
        release: Some(kernel::bindings::single_release),
        open: Some(print_open::<Self> as _),
        // TODO safety
        ..unsafe { core::mem::zeroed() }
    };
}

impl<'a> DebugfsBuilder<'a> {
    /// TODO doc
    // TODO does this
    pub fn value_file<T: DebugValue>(&self, name: &CStr, value: &'a T) {
        // SAFETY: Since the value provided has a lifetime that matches a universal quantifier, it
        // is either 'static, and outlives everything, or it came from the backing, which outlives
        // the debugfs node. `'static` can't ever move, and the backing is pinned, so it can't move
        // either.
        unsafe {
            T::register(
                name.as_char_ptr(),
                0444,
                self.as_ptr(),
                value as *const _ as *mut _,
            )
        }
    }
    /// TODO doc
    pub fn seq_file<T: Printer>(&self, name: &CStr, data: &'a T) {
        unsafe {
            // TODO error
            kernel::bindings::debugfs_create_file_full(
                name.as_char_ptr(),
                0444,
                self.as_ptr(),
                data as *const _ as *mut _,
                core::ptr::null(),
                &<T as Printer>::VTABLE,
            );
        }
    }
}

#[pin_data(PinnedDrop)]
/// A debugfs that contains the value backing and and a handle to the debugfs, with the backing
/// pinned, removes the debugfs before turning down.
pub struct DebugfsValues<T> {
    #[pin]
    backing: T,
    debugfs: ARef<DebugfsDir>,
}

impl<T> DebugfsValues<T> {
    /// Construct with value backing - the attached debugfs dir will be torn down when done
    pub fn attach(
        backing: impl PinInit<T, Error>,
        debugfs: ARef<DebugfsDir>,
    ) -> impl PinInit<Self, Error> {
        try_pin_init! { Self {
            backing <- backing,
            debugfs: debugfs,
        }}
    }
    /// Provide a closure/function which works on any lifetime, and you can build value_files
    /// knowing that they must only be referencing infinite-length stuff or stuff in backing.
    pub fn build<F: for<'a> FnOnce(&'a T, DebugfsBuilder<'a>)>(&self, f: F) {
        f(&self.backing, DebugfsBuilder(&self.debugfs))
    }
}

#[pinned_drop]
impl<T> PinnedDrop for DebugfsValues<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe { kernel::bindings::debugfs_remove(self.debugfs.as_ptr()) }
    }
}
