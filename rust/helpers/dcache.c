// SPDX-License-Identifier: GPL-2.0

/*
 * Copyright (C) 2025 Google LLC.
 */

#include <linux/dcache.h>

struct dentry *rust_helper_dget(struct dentry *d)
{
	return dget(d);
}
