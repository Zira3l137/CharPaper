use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering;

static LIVE_BYTES: AtomicIsize = AtomicIsize::new(0);
static LIVE_BLOCKS: AtomicIsize = AtomicIsize::new(0);

// Wraps the app's allocator to count what is allocated and not yet freed: everything Rust code
// holds on the heap, the app's, Bevy's and wgpu's alike, but not what the graphics driver or
// the system allocate on their own. Two uncontended atomic adds per allocation, always on, so
// the count is right from the first allocation.
pub struct CountingAllocator<A>(pub A);

// SAFETY: every call is passed on to the wrapped allocator unchanged; only counts are kept.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's guarantees hold for the wrapped allocator too.
        let ptr = unsafe { self.0.alloc(layout) };
        if !ptr.is_null() {
            counted(layout.size() as isize, 1);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as above.
        let ptr = unsafe { self.0.alloc_zeroed(layout) };
        if !ptr.is_null() {
            counted(layout.size() as isize, 1);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: as above.
        unsafe { self.0.dealloc(ptr, layout) };
        counted(-(layout.size() as isize), -1);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: as above.
        let new = unsafe { self.0.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            counted(new_size as isize - layout.size() as isize, 0);
        }
        new
    }
}

fn counted(bytes: isize, blocks: isize) {
    LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed);
    LIVE_BLOCKS.fetch_add(blocks, Ordering::Relaxed);
}

// None when the app's allocator isn't wrapped, since nothing was ever counted then.
pub(crate) fn live() -> Option<(usize, usize)> {
    let blocks = LIVE_BLOCKS.load(Ordering::Relaxed);
    (blocks > 0).then(|| (LIVE_BYTES.load(Ordering::Relaxed).max(0) as usize, blocks as usize))
}
