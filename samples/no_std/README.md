# A core/alloc-only example for zephyr-rust

`CONFIG_RUST_STD=n` selects Cargo `-Zbuild-std=core,alloc`. Neither the image
root nor its application dependencies link target std. The sample supplies a
panic handler; `CONFIG_RUST_ALLOC_POOL` registers the allocator at the generated
image root. Kernel allocation, static mutexes, semaphores, and userspace syscalls
are exercised before the intentional user-mode fault. Std examples live in
`samples/rust-app`.
