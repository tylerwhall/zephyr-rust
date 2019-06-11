/*
 * Copyright (c) 2012-2014 Wind River Systems, Inc.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <zephyr.h>
#include <init.h>
#include <misc/mempool.h>
#include <app_memory/app_memdomain.h>

struct k_mem_domain rust_std_domain;

#ifdef CONFIG_USERSPACE
K_APPMEM_PARTITION_DEFINE(rust_std_partition);
#define RUST_STD_SECTION K_APP_DMEM_SECTION(rust_std_partition)
#else
#define RUST_STD_SECTION .data
#endif

#define RUST_STD_MEM_POOL_SIZE 1024
SYS_MEM_POOL_DEFINE(rust_std_mem_pool, NULL, 16,
                    RUST_STD_MEM_POOL_SIZE, 1, 4, RUST_STD_SECTION);

static int rust_std_init(struct device *arg)
{
    ARG_UNUSED(arg);

#ifdef CONFIG_USERSPACE
    k_mem_domain_init(&rust_std_domain, 0, NULL);
    k_mem_domain_add_partition(&rust_std_domain, &rust_std_partition);
#endif
    sys_mem_pool_init(&rust_std_mem_pool);

    return 0;
}

SYS_INIT(rust_std_init, PRE_KERNEL_2, CONFIG_APPLICATION_INIT_PRIORITY);
