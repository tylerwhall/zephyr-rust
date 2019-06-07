#define __ZEPHYR_DEFINE_SYSCALLS__
#include <rust_syscall_macros.h>
#include <kernel.h>
#include <device.h>
#include <uart.h>

// Create a constant we can use from Rust in all cases
#ifdef CONFIG_USERSPACE
const bool RUST_CONFIG_USERSPACE = true;
#else
const bool RUST_CONFIG_USERSPACE = false;
#endif
