// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

//! Sample DebugFS exporting module

use core::sync::atomic::{AtomicU32, Ordering};
use kernel::c_str;
use kernel::debugfs::{Dir, File};
use kernel::prelude::*;
use kernel::sync::{new_mutex, Arc};

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
            _debugfs: debugfs,
            _subdir: sub,
            _file: file,
            _file_2: file_2,
        })
    }
}
