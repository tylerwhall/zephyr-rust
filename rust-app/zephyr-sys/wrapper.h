// Work around compile error on x86 with clang < 3.9.0 where __float128 is not
// defined
#define _GCC_MAX_ALIGN_T

#include <rust_syscall_macros.h>
#include <kernel.h>

// Create a constant we can use from Rust in all cases
#ifdef CONFIG_USERSPACE
const bool RUST_CONFIG_USERSPACE = true;
#else
const bool RUST_CONFIG_USERSPACE = false;
#endif
