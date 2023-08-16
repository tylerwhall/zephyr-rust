#include <version.h>
#if KERNEL_VERSION_MAJOR < 3
#include <zephyr.h>
#else
#include <zephyr/kernel.h>
#endif

#define MY_STACK_SIZE 1024
#define MY_PRIORITY 5

extern void rust_main(void);
extern void rust_second_thread(void *, void *, void *);

K_THREAD_DEFINE(my_tid, MY_STACK_SIZE,
				rust_second_thread, NULL, NULL, NULL,
				MY_PRIORITY, 0, 0);

void main(void)
{
	rust_main();
}
