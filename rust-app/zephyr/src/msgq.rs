// Assuming kernel-only for now. No special handling of the kobject allocation.
// TODO: make size a parameter

pub struct MessageQueue<T> {
    storage: [T; 8];
}

impl<T> MessageQueue<T> {
    unsafe fn new() {
    }

    fn put() {
        crate::kernel::
    }

    fn get() {
    }

    fn peek() {
    }
}
