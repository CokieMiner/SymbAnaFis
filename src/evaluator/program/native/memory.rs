//! Operating-system executable virtual memory management with strict W^X protection.

#![expect(
    unsafe_code,
    reason = "allocating and protecting executable virtual memory pages requires OS FFI calls"
)]

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ptr::{NonNull, copy_nonoverlapping, null_mut},
    slice::from_raw_parts,
};

/// An immutable, page-aligned buffer of executable machine code.
///
/// Pages are allocated writable, populated with machine-code bytes, switched to
/// read-execute (`W^X`), and flushed from the instruction cache on architectures
/// without coherent instruction caches (`aarch64`, `riscv64`, Windows). The
/// mapping is released when the buffer is dropped.
pub struct ExecutableBuffer {
    ptr: NonNull<u8>,
    len: usize,
}

// SAFETY: `ExecutableBuffer` owns an immutable read-execute virtual memory
// mapping that is never mutated after construction and is unmapped exclusively
// on `Drop`.
unsafe impl Send for ExecutableBuffer {}

// SAFETY: Concurrent threads may read and execute the immutable code pages
// simultaneously without synchronization.
unsafe impl Sync for ExecutableBuffer {}

impl Debug for ExecutableBuffer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("ExecutableBuffer")
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}

impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        self.release();
    }
}

impl ExecutableBuffer {
    /// Allocate executable virtual memory pages and copy `code` into them.
    ///
    /// Returns `None` if `code` is empty, the host OS is unsupported, or the
    /// virtual memory allocation / protection transition fails.
    #[must_use]
    pub fn new(code: &[u8]) -> Option<Self> {
        if code.is_empty() {
            return None;
        }
        let ptr = Self::allocate(code)?;
        Some(Self {
            ptr,
            len: code.len(),
        })
    }

    /// Raw pointer to the entry point at byte offset `0`.
    #[must_use]
    pub const fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Borrow the position-independent machine-code bytes stored in the buffer.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        // SAFETY: `self.ptr` points to a readable mapping of at least `self.len`
        // initialized bytes for the lifetime of `self`.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    #[cfg(any(
        target_os = "linux",
        all(target_os = "macos", not(target_arch = "aarch64"))
    ))]
    fn allocate(code: &[u8]) -> Option<NonNull<u8>> {
        const PROT_READ: i32 = 0x1;
        const PROT_WRITE: i32 = 0x2;
        const PROT_EXEC: i32 = 0x4;
        const MAP_PRIVATE: i32 = 0x0002;
        #[cfg(target_os = "linux")]
        const MAP_ANONYMOUS: i32 = 0x0020;
        #[cfg(target_os = "macos")]
        const MAP_ANONYMOUS: i32 = 0x1000;

        unsafe extern "C" {
            fn mmap(
                addr: *mut u8,
                len: usize,
                prot: i32,
                flags: i32,
                fd: i32,
                offset: isize,
            ) -> *mut u8;
            fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
            fn munmap(addr: *mut u8, len: usize) -> i32;
        }

        let len = code.len();
        // SAFETY: `mmap` with `MAP_PRIVATE | MAP_ANONYMOUS` and null address
        // requests a fresh private writable page mapping from the OS kernel.
        let raw = unsafe {
            mmap(
                null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if raw.is_null() || raw.addr() == usize::MAX {
            return None;
        }
        // SAFETY: `raw` is a writable mapping of `len` bytes and `code` is disjoint.
        unsafe {
            copy_nonoverlapping(code.as_ptr(), raw, len);
        }
        // SAFETY: transitions the page-aligned mapping to strict read-execute (`W^X`).
        let protected = unsafe { mprotect(raw, len, PROT_READ | PROT_EXEC) };
        if protected != 0 {
            // SAFETY: `raw` was allocated by `mmap` with length `len`.
            unsafe {
                let _ = munmap(raw, len);
            }
            return None;
        }
        Self::flush_icache(raw, len);
        NonNull::new(raw)
    }

    #[cfg(all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "riscv64")
    ))]
    fn flush_icache(ptr: *mut u8, len: usize) {
        unsafe extern "C" {
            fn __clear_cache(start: *mut u8, end: *mut u8);
        }
        // SAFETY: `ptr..ptr.wrapping_add(len)` is the valid mapped buffer range.
        unsafe {
            __clear_cache(ptr, ptr.wrapping_add(len));
        }
    }

    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    fn flush_icache(ptr: *mut u8, len: usize) {
        unsafe extern "C" {
            fn sys_icache_invalidate(start: *mut u8, len: usize);
        }
        // SAFETY: `ptr..ptr.wrapping_add(len)` is the valid mapped buffer range.
        unsafe {
            sys_icache_invalidate(ptr, len);
        }
    }

    #[cfg(all(
        target_os = "linux",
        not(any(target_arch = "aarch64", target_arch = "riscv64"))
    ))]
    const fn flush_icache(_ptr: *mut u8, _len: usize) {}

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn allocate(code: &[u8]) -> Option<NonNull<u8>> {
        const PROT_READ: i32 = 0x1;
        const PROT_WRITE: i32 = 0x2;
        const PROT_EXEC: i32 = 0x4;
        const MAP_PRIVATE: i32 = 0x0002;
        const MAP_ANONYMOUS: i32 = 0x1000;
        const MAP_JIT: i32 = 0x0800;

        unsafe extern "C" {
            fn mmap(
                addr: *mut u8,
                len: usize,
                prot: i32,
                flags: i32,
                fd: i32,
                offset: isize,
            ) -> *mut u8;
            fn pthread_jit_write_protect_np(enabled: i32);
            fn sys_icache_invalidate(start: *mut u8, len: usize);
        }

        let len = code.len();
        // SAFETY: `mmap` with `MAP_JIT` allocates a W^X JIT region on Apple Silicon.
        let raw = unsafe {
            mmap(
                null_mut(),
                len,
                PROT_READ | PROT_WRITE | PROT_EXEC,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_JIT,
                -1,
                0,
            )
        };
        if raw.is_null() || raw.addr() == usize::MAX {
            return None;
        }
        // SAFETY: disables write protection for the current thread on `MAP_JIT`
        // pages, copies `len` bytes, restores execute protection, and flushes I-cache.
        unsafe {
            pthread_jit_write_protect_np(0);
            copy_nonoverlapping(code.as_ptr(), raw, len);
            pthread_jit_write_protect_np(1);
            sys_icache_invalidate(raw, len);
        }
        NonNull::new(raw)
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn release(&mut self) {
        unsafe extern "C" {
            fn munmap(addr: *mut u8, len: usize) -> i32;
        }
        // SAFETY: `self.ptr` and `self.len` came from a successful `mmap` in `Self::allocate`.
        unsafe {
            let _ = munmap(self.ptr.as_ptr(), self.len);
        }
    }

    #[cfg(target_os = "windows")]
    fn allocate(code: &[u8]) -> Option<NonNull<u8>> {
        const MEM_COMMIT: u32 = 0x1000;
        const MEM_RESERVE: u32 = 0x2000;
        const MEM_RELEASE: u32 = 0x8000;
        const PAGE_READWRITE: u32 = 0x04;
        const PAGE_EXECUTE_READ: u32 = 0x20;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn VirtualAlloc(addr: *mut u8, size: usize, alloc_type: u32, protect: u32) -> *mut u8;
            fn VirtualProtect(
                addr: *mut u8,
                size: usize,
                new_protect: u32,
                old_protect: *mut u32,
            ) -> i32;
            fn VirtualFree(addr: *mut u8, size: usize, free_type: u32) -> i32;
            fn GetCurrentProcess() -> *mut u8;
            fn FlushInstructionCache(process: *mut u8, base: *const u8, size: usize) -> i32;
        }

        let len = code.len();
        // SAFETY: `VirtualAlloc` reserves and commits `len` read-write bytes.
        let raw =
            unsafe { VirtualAlloc(null_mut(), len, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: `raw` points to `len` writable bytes and `code` is disjoint.
        unsafe {
            copy_nonoverlapping(code.as_ptr(), raw, len);
        }
        let mut old_protect = 0_u32;
        // SAFETY: transitions the allocated region from `PAGE_READWRITE` to `PAGE_EXECUTE_READ`.
        let ok = unsafe { VirtualProtect(raw, len, PAGE_EXECUTE_READ, &raw mut old_protect) };
        if ok == 0 {
            // SAFETY: `raw` was returned by `VirtualAlloc`; `MEM_RELEASE` requires `size == 0`.
            unsafe {
                let _ = VirtualFree(raw, 0, MEM_RELEASE);
            }
            return None;
        }
        // SAFETY: flushes the processor instruction cache for the newly protected pages.
        unsafe {
            let _ = FlushInstructionCache(GetCurrentProcess(), raw, len);
        }
        NonNull::new(raw)
    }

    #[cfg(target_os = "windows")]
    fn release(&mut self) {
        const MEM_RELEASE: u32 = 0x8000;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn VirtualFree(addr: *mut u8, size: usize, free_type: u32) -> i32;
        }
        // SAFETY: `self.ptr` was allocated by `VirtualAlloc`; `MEM_RELEASE` requires `size == 0`.
        unsafe {
            let _ = VirtualFree(self.ptr.as_ptr(), 0, MEM_RELEASE);
        }
    }
}
