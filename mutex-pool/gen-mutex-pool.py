#!/usr/bin/env python3

import sys

count = int(sys.argv[1])

initializer = "\n".join([f'\tZ_MUTEX_INITIALIZER(rust_mutex_pool[{i}]),' for i in range(count)])

sys.stdout.write(f'''
#include <version.h>
#if KERNEL_VERSION_MAJOR < 3
#include <kernel.h>
#else
#include <zephyr/kernel.h>
#endif

#if KERNEL_VERSION_MAJOR < 3
Z_STRUCT_SECTION_ITERABLE(k_mutex, rust_mutex_pool[{count}]) = {{
#else
STRUCT_SECTION_ITERABLE_ARRAY(k_mutex, rust_mutex_pool, {count}) = {{
#endif
{initializer}
}};
''')
