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

struct PgSharedThreshold<T, F, C> {
    parallel_state: *mut ParallelScanState,
    initial_value: T,
    is_more_restrictive: F,
    competitive_threshold: C,
    _phantom: PhantomData<T>,
}

unsafe impl<T, F, C> Send for PgSharedThreshold<T, F, C> {}
unsafe impl<T, F, C> Sync for PgSharedThreshold<T, F, C> {}

impl<T, F, C> PgSharedThreshold<T, F, C>
where
    T: Copy + Send + Sync,
    F: Fn(T, T) -> std::cmp::Ordering + Send + Sync,
    C: Fn(T, u32, u32) -> T + Send + Sync,
{
    pub fn new(
        parallel_state: *mut ParallelScanState,
        initial_value: T,
        is_more_restrictive: F,
        competitive_threshold: C,
    ) -> Self {
        assert!(
            std::mem::size_of::<T>() <= 16,
            "SharedThreshold type T must be <= 16 bytes"
        );
        Self {
            parallel_state,
            initial_value,
            is_more_restrictive,
            competitive_threshold,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, C> SharedThreshold<T> for PgSharedThreshold<T, F, C>
where
    T: Copy + Send + Sync,
    F: Fn(T, T) -> std::cmp::Ordering + Send + Sync,
    C: Fn(T, u32, u32) -> T + Send + Sync,
{
    fn load(&self) -> (T, u32) {
        let state = unsafe { &mut *self.parallel_state };
        let mut _lock = state.shared_threshold.lock.acquire();

        if state.shared_threshold.initialized {
            let mut value: T = self.initial_value;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    state.shared_threshold.value.as_ptr(),
                    &mut value as *mut T as *mut u8,
                    std::mem::size_of::<T>(),
                );
            }
            (value, state.shared_threshold.segment_ord)
        } else {
            (self.initial_value, u32::MAX)
        }
    }

    fn update(&self, new_threshold: T, segment_ord: u32) -> (T, u32) {
        let state = unsafe { &mut *self.parallel_state };
        let mut _lock = state.shared_threshold.lock.acquire();

        if !state.shared_threshold.initialized {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &new_threshold as *const T as *const u8,
                    state.shared_threshold.value.as_mut_ptr(),
                    std::mem::size_of::<T>(),
                );
            }
            state.shared_threshold.segment_ord = segment_ord;
            state.shared_threshold.initialized = true;
            (new_threshold, segment_ord)
        } else {
            let mut current_threshold: T = self.initial_value;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    state.shared_threshold.value.as_ptr(),
                    &mut current_threshold as *mut T as *mut u8,
                    std::mem::size_of::<T>(),
                );
            }

            let cmp = (self.is_more_restrictive)(new_threshold, current_threshold);
            let is_better = match cmp {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => segment_ord < state.shared_threshold.segment_ord,
            };

            if is_better {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        &new_threshold as *const T as *const u8,
                        state.shared_threshold.value.as_mut_ptr(),
                        std::mem::size_of::<T>(),
                    );
                }
                state.shared_threshold.segment_ord = segment_ord;
                (new_threshold, segment_ord)
            } else {
                (current_threshold, state.shared_threshold.segment_ord)
            }
        }
    }

    fn competitive_threshold(&self, value: T, threshold_ord: u32, segment_ord: u32) -> T {
        (self.competitive_threshold)(value, threshold_ord, segment_ord)
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
    // f32_to_ordered_u32(Score::MIN) == 0 (because MIN is negative infinity, which maps to 0 in this monotonic conversion)
    // Wait, let's just use a constant. Since f32::MIN.to_bits() is 0xff800000.
    // 0xff800000 & 0x8000_0000 != 0, so !0xff800000 = 0x007fffff.
    // So top is 0x007fffff. bottom is !u32::MAX = 0.
    // So INITIAL_PACKED_VALUE is (0x007fffff << 32) | 0.
    pub const INITIAL_PACKED_VALUE: u64 = 0x007fffff_00000000;

    /// Creates a SharedThreshold configured specifically for Tantivy BM25 Scores,
    /// backed by an `AtomicU64` in shared memory for lock-free updates.
    pub fn new(parallel_state: *mut ParallelScanState) -> Self {
        Self { parallel_state }
    }
}

impl SharedThreshold<tantivy::Score> for PgAtomicSharedScoreThreshold {
    fn load(&self) -> (tantivy::Score, u32) {
        let state = unsafe { &*self.parallel_state };
        unpack_score_and_ord(
            state
                .shared_threshold
                .atomic_score_threshold
                .load(Ordering::Relaxed),
        )
    }

    fn update(&self, new_score: tantivy::Score, segment_ord: u32) -> (tantivy::Score, u32) {
        let state = unsafe { &*self.parallel_state };
        let new_packed = pack_score_and_ord(new_score, segment_ord);
        let mut current = state
            .shared_threshold
            .atomic_score_threshold
            .load(Ordering::Relaxed);
        loop {
            if new_packed <= current {
                return unpack_score_and_ord(current);
            }
            match state
                .shared_threshold
                .atomic_score_threshold
                .compare_exchange_weak(current, new_packed, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => return (new_score, segment_ord),
                Err(actual) => current = actual,
            }
        }
    }

    /// # Tie-breaking determinism
    /// Tantivy's fast-path WAND scorer uses a strict inequality (`score > threshold`) to prune documents.
    /// When our segment is supposed to WIN a global tie-breaker (`segment_ord < threshold_ord`), we must
    /// accept documents with a score *equal* to the threshold.
    /// To do this mathematically against a strict `>` operator, we artificially lower the threshold by
    /// one float epsilon (`next_down()`), ensuring `score > threshold_minus_epsilon` evaluates to `true`.
    fn competitive_threshold(
        &self,
        value: tantivy::Score,
        threshold_ord: u32,
        segment_ord: u32,
    ) -> tantivy::Score {
        if segment_ord < threshold_ord {
            value.next_down()
        } else {
            value
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
    order: tantivy::collector::sort_key::ComparatorEnum,
) -> impl SharedThreshold<Option<u64>> {
    use tantivy::collector::sort_key::Comparator;
    PgSharedThreshold::new(
        parallel_state,
        None,
        move |new: Option<u64>, current: Option<u64>| order.compare(&new, &current),
        |score: Option<u64>, _threshold_ord: u32, _segment_ord: u32| score,
    )
}
