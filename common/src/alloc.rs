use std::alloc::{GlobalAlloc, System};
use std::cell::Cell;
use std::fmt::Display;

// OPEN QUESTION: How to handle memory allocated before budget context, but dropped inside of?

/// Thread-local memory budget. All members are `Cell`s, and this makes heavy use of interior
/// mutability, as we need the base struct to be a thread local `pub static`
pub struct Budget {
    soft_budget: Cell<usize>,
    hard_budget: Cell<usize>,
    allocated: Cell<usize>,
    peak: Cell<usize>,
}
impl Budget {
    pub const fn new() -> Self {
        Self {
            soft_budget: Cell::new(usize::MAX),
            hard_budget: Cell::new(usize::MAX),
            allocated: Cell::new(0),
            peak: Cell::new(0),
        }
    }

    pub fn exceeded(&self) -> bool {
        if self.peak > self.hard_budget {
            panic!(
                "hard budget of {} exceeded with peak of {}",
                self.hard_budget.get(),
                self.peak.get()
            );
        }
        self.peak > self.soft_budget
    }

    pub fn set_budget(&self, soft: usize, hard: Option<usize>) {
        let hard = hard.unwrap_or(soft * 3 / 2); // default to 1.5x
        self.soft_budget.set(soft);
        self.hard_budget.set(hard);
        self.clear();
    }

    pub fn clear(&self) {
        self.allocated.set(0);
        self.peak.set(0);
    }

    pub fn reset_to_no_budget(&self) {
        let no_budget = Self::new();
        self.soft_budget.set(no_budget.soft_budget.get());
        self.hard_budget.set(no_budget.hard_budget.get());
        self.allocated.set(no_budget.allocated.get());
        self.peak.set(no_budget.peak.get());
    }

    #[inline(always)]
    unsafe fn charge(&self, size: usize) {
        self.allocated.set(self.allocated.get() + size);
        self.peak.set(self.peak.get().max(self.allocated.get())); // maybe update only on check or decrease
    }

    #[inline(always)]
    unsafe fn release(&self, size: usize) {
        self.allocated.set(self.allocated.get() - size);
    }
}
impl Default for Budget {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    pub static THREAD_MEMORY_BUDGET: Budget = const { Budget::new() };
}

#[derive(Debug)]
pub struct BudgetExceededError {
    budget: usize,
    peak: usize,
}
impl Display for BudgetExceededError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Memory budget of {} was exceeded with peak of {}",
            self.budget, self.peak,
        )
    }
}
impl std::error::Error for BudgetExceededError {}

pub fn thread_memory_budget_has_been_exceeded() -> bool {
    THREAD_MEMORY_BUDGET.with(|b| b.exceeded())
}

pub fn check_if_thread_memory_budget_has_been_exceeded_with_error()
-> Result<(), BudgetExceededError> {
    THREAD_MEMORY_BUDGET.with(|b| {
        if b.exceeded() {
            Err(BudgetExceededError {
                budget: b.soft_budget.get(),
                peak: b.peak.get(),
            })
        } else {
            Ok(())
        }
    })
}

pub struct BudgetAlloc;
unsafe impl GlobalAlloc for BudgetAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe {
            let ptr = System.alloc(layout);
            THREAD_MEMORY_BUDGET.with(|b| b.charge(allocated_size(ptr, layout)));
            ptr
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        unsafe {
            let current_size = allocated_size(ptr, layout);
            let reallocated = System.realloc(ptr, layout, new_size);
            let new_size = allocated_size(reallocated, layout);
            if new_size > current_size {
                THREAD_MEMORY_BUDGET.with(|b| b.charge(new_size - current_size));
            } else {
                THREAD_MEMORY_BUDGET.with(|b| b.release(current_size - new_size));
            }
            reallocated
        }
    }

    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe {
            let ptr = System.alloc_zeroed(layout);
            THREAD_MEMORY_BUDGET.with(|b| b.charge(allocated_size(ptr, layout)));
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe {
            let size = allocated_size(ptr, layout);
            System.dealloc(ptr, layout);
            THREAD_MEMORY_BUDGET.with(|b| b.release(size));
        }
    }
}

/// Bytes the system allocator actually reserved for `ptr`.
///
/// # Safety
/// `ptr` must be a live, non-null pointer obtained from the *system* malloc family
/// (`std::alloc::System` / `allocator_api2::alloc::Global` on unix, which use
/// `malloc`/`posix_memalign`/`realloc`) and not yet freed. Passing a pointer from
/// jemalloc, mimalloc, a bump arena, etc. is undefined behavior.
#[inline]
unsafe fn allocated_size(ptr: *mut u8, layout: std::alloc::Layout) -> usize {
    #[cfg(any(
        all(target_os = "linux", any(target_env = "gnu", target_env = "musl")),
        target_os = "android",
        target_os = "freebsd",
    ))]
    {
        let _ = layout;
        unsafe { libc::malloc_usable_size(ptr.cast::<c_void>()) }
    }

    #[cfg(target_vendor = "apple")]
    {
        let _ = layout;
        unsafe {
            use std::ffi::c_void;
            libc::malloc_size(ptr.cast::<c_void>())
        }
    }

    #[cfg(not(any(
        all(target_os = "linux", any(target_env = "gnu", target_env = "musl")),
        target_os = "android",
        target_os = "freebsd",
        target_vendor = "apple",
    )))]
    {
        // No reliable query (e.g. Windows `System` uses HeapAlloc and over-allocates
        // with a hidden header for over-aligned layouts, so `_msize`/`HeapSize` would
        // be wrong). Fall back to the requested size; still symmetric.
        let _ = ptr;
        layout.size()
    }
}
