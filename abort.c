/*
 * Copyright (c) 2020 Linaro Limited
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <stdlib.h>
#include <zephyr.h>

#if KERNEL_VERSION_MAJOR <= 2 && KERNEL_VERSION_MINOR < 5
void __attribute__((weak, noreturn)) abort(void)
{
	printk("abort()\n");
	k_panic();
        __builtin_unreachable();
}
#endif
