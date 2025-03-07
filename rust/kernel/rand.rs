// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

//! Randomness Abstraction
//!
//! C header: [`include/linux/random.h`](srctree/include/linux/random.h)

/// Add data to the randomness input pool that is likely to differ between devices, and sometimes
/// between boots.
///
/// This does not increase the entropy available to the pool, as these values may be predictable or
/// static. Adding this data is primarily to get different randomness from different devices or
/// boots in low-entropy scenarios.
pub fn add_device_randomness(data: &[u8]) {
    // SAFETY: add_device_randomness only requires that the provided buffer is legal to read for
    // the provided length. We know this by slice invariants.
    unsafe { crate::bindings::add_device_randomness(data.as_ptr() as *const _, data.len()) }
}
