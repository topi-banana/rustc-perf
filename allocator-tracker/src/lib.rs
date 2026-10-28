use libc::{c_void, size_t};
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_ALLOCATED: AtomicU64 = AtomicU64::new(0);

// Resolve the real libc implementations under names that do not collide with
// the interposed functions exported by this library. Calling these bypasses
// LD_PRELOAD interposition, avoiding reentrancy during early allocation.
extern "C" {
    #[link_name = "__libc_malloc"]
    fn libc_malloc(size: size_t) -> *mut c_void;
    #[link_name = "__libc_calloc"]
    fn libc_calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    #[link_name = "__libc_realloc"]
    fn libc_realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    #[link_name = "__libc_free"]
    fn libc_free(ptr: *mut c_void);
}

#[no_mangle]
pub unsafe extern "C" fn malloc(size: size_t) -> *mut c_void {
    let ptr = libc_malloc(size);
    if !ptr.is_null() && size != 0 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_ALLOCATED.fetch_add(size as u64, Ordering::Relaxed);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: size_t, size: size_t) -> *mut c_void {
    let ptr = libc_calloc(nmemb, size);
    let bytes = nmemb.saturating_mul(size);
    if !ptr.is_null() && bytes != 0 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_ALLOCATED.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }
    if size == 0 {
        free(ptr);
        return std::ptr::null_mut();
    }
    let new_ptr = libc_realloc(ptr, size);
    if !new_ptr.is_null() {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_ALLOCATED.fetch_add(size as u64, Ordering::Relaxed);
    }
    new_ptr
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    libc_free(ptr);
}

extern "C" fn report() {
    println!("!allocations:{}", ALLOCATION_COUNT.load(Ordering::Relaxed));
    println!(
        "!allocated-bytes:{}",
        TOTAL_ALLOCATED.load(Ordering::Relaxed)
    );
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

extern "C" fn init() {
    unsafe {
        libc::atexit(report);
    }
}

#[used]
#[link_section = ".init_array"]
static INIT_ARRAY: [extern "C" fn(); 1] = [init];
