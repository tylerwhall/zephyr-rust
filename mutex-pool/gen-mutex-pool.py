#!/usr/bin/env python3

import sys

count = int(sys.argv[1])

initializer = "\n".join([f'\tZ_MUTEX_INITIALIZER(rust_mutex_pool[{i}]),' for i in range(count)])

sys.stdout.write(f'''
#include <kernel.h>

Z_STRUCT_SECTION_ITERABLE(k_mutex, rust_mutex_pool[{count}]) = {{
{initializer}
}};
''')
