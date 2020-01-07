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

#if defined(CONFIG_RUST_ALLOC_POOL)
SYS_MEM_POOL_DEFINE(rust_std_mem_pool, NULL, CONFIG_RUST_HEAP_MEM_POOL_MIN_SIZE,
                    CONFIG_RUST_HEAP_MEM_POOL_SIZE, 1, 4, RUST_STD_SECTION);
#elif CONFIG_HEAP_MEM_POOL_SIZE == 0
#error CONFIG_HEAP_MEM_POOL_SIZE (k_malloc) must be non-zero if not using a Rust sys mem pool.
#endif

#if defined(CONFIG_USERSPACE) || defined(CONFIG_RUST_ALLOC_POOL)
static int rust_std_init(struct device *arg)
{
    ARG_UNUSED(arg);

#ifdef CONFIG_USERSPACE
    struct k_mem_partition *rust_std_parts[] = { &rust_std_partition };

    k_mem_domain_init(&rust_std_domain, ARRAY_SIZE(rust_std_parts), rust_std_parts);
#endif
#ifdef CONFIG_RUST_ALLOC_POOL
    sys_mem_pool_init(&rust_std_mem_pool);
#endif

    return 0;
}

SYS_INIT(rust_std_init, PRE_KERNEL_2, CONFIG_APPLICATION_INIT_PRIORITY);
#endif
