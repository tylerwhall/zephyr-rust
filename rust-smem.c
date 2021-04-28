#include <zephyr.h>
#include <init.h>
#include <version.h>
#include <app_memory/app_memdomain.h>

#ifdef CONFIG_USERSPACE
struct k_mem_domain rust_std_domain;
K_APPMEM_PARTITION_DEFINE(rust_std_partition);
#define RUST_STD_SECTION K_APP_DMEM_SECTION(rust_std_partition)
#else
#define RUST_STD_SECTION .data
#endif

#if defined(CONFIG_RUST_ALLOC_POOL)

#define RUST_STD_MEM_POOL_SIZE (WB_UP(CONFIG_RUST_HEAP_MEM_POOL_MAX_SIZE) * \
		CONFIG_RUST_HEAP_MEM_POOL_NMAX)

char __aligned(sizeof(void *)) Z_GENERIC_SECTION(RUST_STD_SECTION)
	kheap_rust_std_mem_pool[RUST_STD_MEM_POOL_SIZE];

struct k_heap Z_GENERIC_SECTION(RUST_STD_SECTION) rust_std_mem_pool;

#elif CONFIG_HEAP_MEM_POOL_SIZE == 0

#error CONFIG_HEAP_MEM_POOL_SIZE (k_malloc) \
	must be non-zero if not using a Rust sys mem pool.

#endif /* defined(CONFIG_RUST_ALLOC_POOL) */

#if defined(CONFIG_USERSPACE) || defined(CONFIG_RUST_ALLOC_POOL)

/* Harmless API difference that generates a warning */
#if ZEPHYR_VERSION_CODE >= ZEPHYR_VERSION(2, 4, 0)
static int rust_std_init(const struct device *arg)
#else
static int rust_std_init(struct device *arg)
#endif /* ZEPHYR_VERSION_CODE >= ZEPHYR_VERSION(2, 4, 0) */
{
	ARG_UNUSED(arg);

#ifdef CONFIG_USERSPACE
	struct k_mem_partition *rust_std_parts[] = { &rust_std_partition };

	k_mem_domain_init(&rust_std_domain,
			ARRAY_SIZE(rust_std_parts), rust_std_parts);
#endif /* CONFIG_USERSPACE */

#ifdef CONFIG_RUST_ALLOC_POOL

	k_heap_init(&rust_std_mem_pool, kheap_rust_std_mem_pool,
			RUST_STD_MEM_POOL_SIZE);

#endif /* CONFIG_RUST_ALLOC_POOL */

#if defined(CONFIG_USERSPACE) && defined(CONFIG_RUST_MUTEX_POOL)
	extern struct k_mutex rust_mutex_pool[CONFIG_RUST_MUTEX_POOL_SIZE];

	for (size_t i = 0; i < ARRAY_SIZE(rust_mutex_pool); i++) {
		k_object_access_all_grant(&rust_mutex_pool[i]);
	}
#endif /* defined(CONFIG_USERSPACE) && defined(CONFIG_RUST_MUTEX_POOL) */

	return 0;
}

SYS_INIT(rust_std_init, PRE_KERNEL_2, CONFIG_APPLICATION_INIT_PRIORITY);

#endif /* defined(CONFIG_USERSPACE) || defined(CONFIG_RUST_ALLOC_POOL) */
