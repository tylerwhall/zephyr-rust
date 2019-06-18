use core::alloc::{GlobalAlloc, Layout};

pub use zephyr_sys::raw::sys_mem_pool;

pub struct MempoolAlloc(pub &'static sys_mem_pool);

unsafe impl Send for MempoolAlloc {}
unsafe impl Sync for MempoolAlloc {}

unsafe impl GlobalAlloc for MempoolAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        zephyr_sys::raw::sys_mem_pool_alloc(self.0 as *const _ as *mut _, layout.size()) as *mut _
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        zephyr_sys::raw::sys_mem_pool_free(ptr as *mut _)
    }
}

/// Assign a Zephyr sys mem pool as #[global_allocator]
///
/// This should be defined with SYS_MEM_POOL_DEFINE and granted permission to any
/// Rust threads that need to use libstd or alloc.
#[macro_export]
macro_rules! global_sys_mem_pool {
    ($pool:ident) => {
        extern "C" {
            #[no_mangle]
            static $pool: $crate::mempool::sys_mem_pool;
        }

        #[global_allocator]
        static GLOBAL: $crate::mempool::MempoolAlloc =
            $crate::mempool::MempoolAlloc(unsafe { &$pool });
    };
}
