use crate::postgres::locks::Spinlock;
use crate::postgres::ParallelScanState;
use std::marker::PhantomData;
use tantivy::collector::SharedThreshold;

#[repr(C)]
pub struct ParallelScanThresholdState {
    // NOTE: Only one of the following two threshold approaches is used per-query depending on the Sort Key type.

    // 1. Lock-free Atomic Threshold. Used exclusively for f32 (BM25 Scores) for maximum parallel throughput.
    atomic_score_threshold: std::sync::atomic::AtomicU64,

    // 2. Lock-based Threshold. Used for arbitrary threshold types <= 16 bytes (like Option<u64> fast fields).
    lock: Spinlock,
    value: [u8; 16],
    segment_ord: u32,
    initialized: bool,
}

impl Default for ParallelScanThresholdState {
    fn default() -> Self {
        Self {
            atomic_score_threshold: std::sync::atomic::AtomicU64::new(
                PgAtomicSharedScoreThreshold::INITIAL_PACKED_VALUE,
            ),
            lock: Spinlock::default(),
            value: [0; 16],
            segment_ord: u32::MAX,
            initialized: false,
        }
    }
}

impl ParallelScanThresholdState {
    pub fn init(&mut self) {
        self.lock.init();
    }

    pub fn reset(&mut self) {
        self.atomic_score_threshold.store(
            PgAtomicSharedScoreThreshold::INITIAL_PACKED_VALUE,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.initialized = false;
        self.value = [0; 16];
        self.segment_ord = u32::MAX;
    }
}

struct PgSharedThreshold<T> {
    parallel_state: *mut ParallelScanState,
    _phantom: PhantomData<T>,
}

unsafe impl<T> Send for PgSharedThreshold<T> {}
unsafe impl<T> Sync for PgSharedThreshold<T> {}

impl<T> PgSharedThreshold<T>
where
    T: Copy + Send + Sync,
{
    pub fn new(parallel_state: *mut ParallelScanState) -> Self {
        assert!(
            std::mem::size_of::<T>() <= 16,
            "SharedThreshold type T must be <= 16 bytes"
        );
        Self {
            parallel_state,
            _phantom: PhantomData,
        }
    }
}

impl<T> SharedThreshold<T> for PgSharedThreshold<T>
where
    T: Copy + Send + Sync + PartialEq,
{
    fn load(&self) -> Option<(T, u32)> {
        let state = unsafe { &mut *self.parallel_state };
        let mut _lock = state.shared_threshold.lock.acquire();

        if state.shared_threshold.initialized {
            let mut value: std::mem::MaybeUninit<T> = std::mem::MaybeUninit::uninit();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    state.shared_threshold.value.as_ptr(),
                    value.as_mut_ptr() as *mut u8,
                    std::mem::size_of::<T>(),
                );
                Some((value.assume_init(), state.shared_threshold.segment_ord))
            }
        } else {
            None
        }
    }

    fn try_update(
        &self,
        expected_threshold: &Option<(T, u32)>,
        new_threshold: (T, u32),
    ) -> Result<(), Option<(T, u32)>> {
        let state = unsafe { &mut *self.parallel_state };
        let mut _lock = state.shared_threshold.lock.acquire();

        let current = if state.shared_threshold.initialized {
            let mut value: std::mem::MaybeUninit<T> = std::mem::MaybeUninit::uninit();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    state.shared_threshold.value.as_ptr(),
                    value.as_mut_ptr() as *mut u8,
                    std::mem::size_of::<T>(),
                );
                Some((value.assume_init(), state.shared_threshold.segment_ord))
            }
        } else {
            None
        };

        if current == *expected_threshold {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &new_threshold.0 as *const T as *const u8,
                    state.shared_threshold.value.as_mut_ptr(),
                    std::mem::size_of::<T>(),
                );
            }
            state.shared_threshold.segment_ord = new_threshold.1;
            state.shared_threshold.initialized = true;
            Ok(())
        } else {
            Err(current)
        }
    }
}

use std::sync::atomic::Ordering;

#[inline]
fn f32_to_ordered_u32(f: f32) -> u32 {
    let bits = f.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits ^ 0x8000_0000
    }
}

#[inline]
fn ordered_u32_to_f32(u: u32) -> f32 {
    let bits = if u & 0x8000_0000 != 0 {
        u ^ 0x8000_0000
    } else {
        !u
    };
    f32::from_bits(bits)
}

#[inline(always)]
fn pack_score_and_ord(score: tantivy::Score, segment_ord: u32) -> u64 {
    let top = f32_to_ordered_u32(score) as u64;
    let bottom = (!segment_ord) as u64;
    (top << 32) | bottom
}

#[inline(always)]
fn unpack_score_and_ord(val: u64) -> (tantivy::Score, u32) {
    let score = ordered_u32_to_f32((val >> 32) as u32);
    let segment_ord = !(val as u32);
    (score, segment_ord)
}

struct PgAtomicSharedScoreThreshold {
    parallel_state: *mut ParallelScanState,
}

unsafe impl Send for PgAtomicSharedScoreThreshold {}
unsafe impl Sync for PgAtomicSharedScoreThreshold {}

impl PgAtomicSharedScoreThreshold {
    // Initial packed value is Score::MIN with u32::MAX (worst possible ordinal)
    pub const INITIAL_PACKED_VALUE: u64 = 0x007fffff_00000000;

    /// Creates a SharedThreshold configured specifically for Tantivy BM25 Scores,
    /// backed by an `AtomicU64` in shared memory for lock-free updates.
    pub fn new(parallel_state: *mut ParallelScanState) -> Self {
        Self { parallel_state }
    }
}

impl SharedThreshold<tantivy::Score> for PgAtomicSharedScoreThreshold {
    fn load(&self) -> Option<(tantivy::Score, u32)> {
        let state = unsafe { &*self.parallel_state };
        let packed = state
            .shared_threshold
            .atomic_score_threshold
            .load(Ordering::Relaxed);

        if packed == Self::INITIAL_PACKED_VALUE {
            None
        } else {
            Some(unpack_score_and_ord(packed))
        }
    }

    fn try_update(
        &self,
        expected_threshold: &Option<(tantivy::Score, u32)>,
        new_threshold: (tantivy::Score, u32),
    ) -> Result<(), Option<(tantivy::Score, u32)>> {
        let state = unsafe { &*self.parallel_state };

        let expected_packed = match expected_threshold {
            Some((score, ord)) => pack_score_and_ord(*score, *ord),
            None => Self::INITIAL_PACKED_VALUE,
        };
        let new_packed = pack_score_and_ord(new_threshold.0, new_threshold.1);

        match state
            .shared_threshold
            .atomic_score_threshold
            .compare_exchange_weak(
                expected_packed,
                new_packed,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
            Ok(_) => Ok(()),
            Err(actual_packed) => {
                if actual_packed == Self::INITIAL_PACKED_VALUE {
                    Err(None)
                } else {
                    Err(Some(unpack_score_and_ord(actual_packed)))
                }
            }
        }
    }
}

pub fn new_score_threshold(
    parallel_state: *mut ParallelScanState,
) -> impl SharedThreshold<tantivy::Score> {
    PgAtomicSharedScoreThreshold::new(parallel_state)
}

pub fn new_fast_value_threshold(
    parallel_state: *mut ParallelScanState,
    _order: tantivy::collector::sort_key::ComparatorEnum,
) -> impl SharedThreshold<Option<u64>> {
    PgSharedThreshold::new(parallel_state)
}
