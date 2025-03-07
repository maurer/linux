// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

use kernel::error::{from_err_ptr, Result};

pub(crate) fn qcom_smem_get(host: i32, item: u32) -> Result<&'static [u8]> {
    let mut size = 0;
    // SAFETY: qcom_smem_get only requires that the size pointer be a writable size_t,
    // host and item are error checked in the qcom_smem module.
    let ptr =
        from_err_ptr(unsafe { kernel::bindings::qcom_smem_get(host as _, item as _, &mut size) })?;
    // SAFETY: If qcom_smem_get does not return an error, the returned pointer points to a readable
    // byte buffer with its size written into size. Because these buffers are derived from the
    // static ranges in the DT, this buffer remains accessible even if the qcom_smem module is
    // unloaded, so 'static is appropriate. The underlying buffer cannot mutate, so upgrading it
    // to a reference is allowed.
    Ok(unsafe { core::slice::from_raw_parts(ptr as *const _, size) })
}
