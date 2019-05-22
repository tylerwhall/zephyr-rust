/*
 * Copyright (c) 2012-2014 Wind River Systems, Inc.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <zephyr.h>
#include <misc/printk.h>

extern uint8_t hello_rust(void);
extern void hello_rust_user(void);

void main(void)
{
        hello_rust();
}
