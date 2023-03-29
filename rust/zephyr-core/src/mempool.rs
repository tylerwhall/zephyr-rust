use core::alloc::{GlobalAlloc, Layout};

pub use zephyr_sys::raw::k_heap;
use zephyr_sys::raw::K_NO_WAIT;

pub struct MempoolAlloc(pub &'static k_heap);

unsafe impl Send for MempoolAlloc {}
unsafe impl Sync for MempoolAlloc {}

unsafe impl GlobalAlloc for MempoolAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let kheap = self.0 as *const _ as *mut _;
        #[cfg(zephyr250)]
        {
            zephyr_sys::raw::k_heap_aligned_alloc(kheap, layout.align(), layout.size(), K_NO_WAIT) as *mut _
        }
        #[cfg(not(zephyr250))]
        {
            let ret = zephyr_sys::raw::k_heap_alloc(kheap, layout.size(), K_NO_WAIT) as *mut _;
            if ret as usize & (layout.align() - 1) != 0 {
                zephyr_sys::raw::printk(
                    "Rust unsatisfied alloc alignment\n\0".as_ptr() as *const libc::c_char
                );
                core::ptr::null_mut()
            } else {
                ret
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let kheap = self.0 as *const _ as *mut _;
        zephyr_sys::raw::k_heap_free(kheap, ptr as *mut _)
    }
}

/// Assign a Zephyr k_heap as #[global_allocator]
///
/// This should be defined with K_HEAP_DEFINE and granted permission to any
/// Rust threads that need to use libstd or alloc.
#[macro_export]
macro_rules! global_sys_mem_pool {
    ($pool:ident) => {
        extern "C" {
            static $pool: $crate::mempool::k_heap;
        }

        #[global_allocator]
        static GLOBAL: $crate::mempool::MempoolAlloc =
            $crate::mempool::MempoolAlloc(unsafe { &$pool });
    };
}
