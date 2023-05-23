/*
 * Copyright (c) 2020 Linaro Limited
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <stdlib.h>
#include <version.h>
#if KERNEL_VERSION_MAJOR < 3
#include <zephyr.h>
#else
#include <zephyr/kernel.h>
#endif

#if KERNEL_VERSION_MAJOR <= 2 && KERNEL_VERSION_MINOR < 5
void __attribute__((weak, noreturn)) abort(void)
{
	printk("abort()\n");
	k_panic();
        __builtin_unreachable();
}
#endif
