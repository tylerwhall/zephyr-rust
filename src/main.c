/*
 * Copyright (c) 2012-2014 Wind River Systems, Inc.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <zephyr.h>
#include <misc/printk.h>

extern uint8_t hello_rust(void);
extern void hello_rust_user(void);

extern uint8_t hello_rust_second_thread(void *, void *, void *);

#define MY_STACK_SIZE 500
#define MY_PRIORITY 5

K_THREAD_DEFINE(my_tid, MY_STACK_SIZE,
		hello_rust_second_thread, NULL, NULL, NULL,
		MY_PRIORITY, 0, K_NO_WAIT);

void main(void)
{
        hello_rust();
}
