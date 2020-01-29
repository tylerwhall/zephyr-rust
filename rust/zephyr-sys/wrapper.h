// Work around compile error on x86 with clang < 3.9.0 where __float128 is not
// defined
#define _GCC_MAX_ALIGN_T

#include <kernel.h>
#include <all_syscalls.h>
#include <misc/mempool.h>
#include <device.h>
#include <uart.h>
#include <uart_buffered.h>

// Create a constant we can use from Rust in all cases
#ifdef CONFIG_USERSPACE
const bool RUST_CONFIG_USERSPACE = true;
#else
const bool RUST_CONFIG_USERSPACE = false;
#endif
