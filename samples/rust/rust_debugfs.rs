// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

//! Sample DebugFS exporting module

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use kernel::c_str;
use kernel::debugfs::{Dir, File};
use kernel::miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration};
use kernel::prelude::*;
use kernel::rbtree::RBTree;
use kernel::sync::ArcBorrow;
use kernel::sync::{new_mutex, Arc, Mutex};
use kernel::uaccess::{UserSlice, UserSliceReader};

module! {
    type: RustDebugFs,
    name: "rust_debugfs",
    authors: ["Matthew Maurer"],
    description: "Rust DebugFS usage sample",
    license: "GPL",
}

struct RustDebugFs {
    // As we only hold these for drop effect (to remove the directory/files) we have a leading
    // underscore to indicate to the compiler that we don't expect to use this field directly.
    _debugfs: Dir,
    _subdir: Dir,
    _file: File,
    _file_2: File,
    _miscdev: Pin<KBox<MiscDeviceRegistration<DynDebugFs>>>,
}

use kernel::ioctl::{_IOC_SIZE, _IOW};

#[repr(C)]
struct PropUpdate {
    id: usize,
    atomic: usize,
    mutex: usize,
}

const CREATE_DEV: u32 = _IOW::<usize>('|' as u32, 1);
const REMOVE_DEV: u32 = _IOW::<usize>('|' as u32, 2);
const SET_DEV_PROP: u32 = _IOW::<PropUpdate>('|' as u32, 3);

#[vtable]
impl MiscDevice for DynDebugFs {
    type Ptr = Arc<Self>;
    type Data = Arc<Self>;
    fn open(_file: &kernel::fs::File, misc: &MiscDeviceRegistration<Self>) -> Result<Arc<Self>> {
        Ok(misc.data().clone())
    }
    fn ioctl(
        me: ArcBorrow<'_, Self>,
        _file: &kernel::fs::File,
        cmd: u32,
        arg: usize,
    ) -> Result<isize> {
        let user_arg = UserSlice::new(arg, _IOC_SIZE(cmd)).reader();
        match cmd {
            CREATE_DEV => me.create_dev(user_arg),
            REMOVE_DEV => me.remove_dev(user_arg),
            SET_DEV_PROP => me.set_dev_prop(user_arg),
            _ => Err(EINVAL),
        }
    }
}

#[pin_data]
struct DynDebugFs {
    dir: Dir,
    #[pin]
    devs: Mutex<RBTree<usize, Dev>>,
}

impl DynDebugFs {
    fn create_dev(&self, mut arg: UserSliceReader) -> Result<isize> {
        let dev_id: usize = arg.read()?;
        let mut devs = self.devs.lock();
        if devs.get(&dev_id).is_some() {
            return Err(EEXIST);
        }
        devs.try_create_and_insert(dev_id, Dev::new(&self.dir)?, GFP_KERNEL)?;
        Ok(0)
    }
    fn remove_dev(&self, mut arg: UserSliceReader) -> Result<isize> {
        let dev_id: usize = arg.read()?;
        let mut devs = self.devs.lock();
        if devs.remove(&dev_id).is_none() {
            return Err(ENOENT);
        }
        Ok(0)
    }
    fn set_dev_prop(&self, mut arg: UserSliceReader) -> Result<isize> {
        let dev_id: usize = arg.read()?;
        let prop_atomic: usize = arg.read()?;
        let prop_mutex: usize = arg.read()?;
        let devs = self.devs.lock();
        let Some(dev) = devs.get(&dev_id) else {
            return Err(ENOENT);
        };

        dev.data.atomic.store(prop_atomic, Ordering::Relaxed);
        *dev.data.mutex.lock() = prop_mutex;

        Ok(0)
    }
    fn new(base: &Dir) -> Result<Arc<Self>> {
        Arc::pin_init(
            pin_init! {
                Self {
                    dir: base.subdir(c_str!("dyn-debugfs")),
                    devs <- new_mutex!(RBTree::new()),
                }
            },
            GFP_KERNEL,
        )
    }
}

#[pin_data]
struct DevData {
    atomic: AtomicUsize,
    #[pin]
    mutex: Mutex<usize>,
}

struct DevDebug {
    _atomic: File,
    _mutex: File,
    _combined: File,
}

struct Dev {
    data: Arc<DevData>,
    _debug: DevDebug,
}

impl Dev {
    fn new(dir: &Dir) -> Result<Self> {
        let data = Arc::pin_init(DevData::new(), GFP_KERNEL)?;
        Ok(Self {
            _debug: DevDebug::new(dir, &data),
            data,
        })
    }
}

impl DevData {
    fn new() -> impl PinInit<Self> {
        pin_init! {
            Self {
                atomic: AtomicUsize::new(0),
                mutex <- new_mutex!(0),
            }
        }
    }
}

impl DevDebug {
    fn new(dir: &Dir, data: &Arc<DevData>) -> Self {
        Self {
            _mutex: dir.fmt_file(c_str!("mutex"), data.clone(), &|x, w| {
                write!(w, "mutex: {}", *x.mutex.lock())
            }),
            _atomic: dir.fmt_file(c_str!("atomic"), data.clone(), &|x, w| {
                write!(w, "atomic: {}", x.atomic.load(Ordering::Relaxed))
            }),
            // This cannot be written in file-per-field style because it involves combining
            // attributes available to two other debugfs files. We could still write it with
            // PinInit, but we'd have to produce the `Arc<DevData>` to support it.
            _combined: dir.fmt_file(c_str!("combined"), data.clone(), &|x, w| {
                write!(
                    w,
                    "combined: {}",
                    x.atomic.load(Ordering::Relaxed) + *x.mutex.lock()
                )
            }),
        }
    }
}

static EXAMPLE: AtomicU32 = AtomicU32::new(8);

impl kernel::Module for RustDebugFs {
    fn init(_this: &'static ThisModule) -> Result<Self> {
        // Create a debugfs directory in the root of the filesystem called "sample_debugfs".
        let debugfs = Dir::new(c_str!("sample_debugfs"));

        // Create a subdirectory, so "sample_debugfs/subdir" now exists.
        let sub = debugfs.subdir(c_str!("subdir"));

        // Create a single file in the subdirectory called "example" that will read from the
        // `EXAMPLE` atomic variable.
        // We `forget` the result to avoid deleting the file at the end of the scope.
        let file = sub.fmt_file(c_str!("example"), &EXAMPLE, &|example, f| {
            writeln!(f, "Reading atomic: {}", example.load(Ordering::Relaxed))
        });
        // Now, "sample_debugfs/subdir/example" will print "Reading atomic: 8\n" when read.

        // Change the value in the variable displayed by the file. This is intended to demonstrate
        // that the module can continue to change the value used by the file.
        EXAMPLE.store(10, Ordering::Relaxed);
        // Now, "sample_debugfs/subdir/example" will print "Reading atomic: 10\n" when read.

        // In addition to globals, we can also attach any kind of owned data. Most commonly, this
        // will look like an `Arc<MyObject>` as those can be shared with the rest of the module.
        let my_arc = Arc::pin_init(new_mutex!(10), GFP_KERNEL)?;
        // An `Arc<Mutex<usize>>` doesn't implement display, so let's give explicit instructions on
        // how to print it
        let file_2 = sub.fmt_file(c_str!("arc_backed"), my_arc.clone(), &|val, f| {
            writeln!(f, "locked value: {:#010x}", *val.lock())
        });

        // Since it's an `Arc` and we cloned it, we continue to have access to `my_arc`. If this
        // were real, we'd probably stash it in our module struct and do something with it when
        // handling real calls.
        *my_arc.lock() = 99;

        // Save the handles we want to preserve to our module object. They will be automatically
        // cleaned up when our module is unloaded.
        Ok(Self {
            _subdir: sub,
            _file: file,
            _file_2: file_2,
            _miscdev: KBox::pin_init(
                MiscDeviceRegistration::register(
                    MiscDeviceOptions {
                        name: c_str!("sample-debugfs-dynamic"),
                    },
                    DynDebugFs::new(&debugfs)?,
                ),
                GFP_KERNEL,
            )?,
            _debugfs: debugfs,
        })
    }
}
