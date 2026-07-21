// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Deterministic-simulation-testing (DST) hooks — the one place that talks to the Antithesis
//! Rust SDK.
//!
//! Everything here is gated on the crate's `enabled` feature. With `enabled` **off** (the
//! default) the crate compiles to nothing: the assertion macros expand to a dead
//! `if false { … }` that still type-checks their arguments, [`GhostState<T>`] is a zero-sized
//! type, `observe!` closures are type-checked but never run, and [`init`] is a no-op — so a
//! consumer that does not opt in never links the SDK. With `enabled` **on** the macros forward
//! to `antithesis_sdk` and dispatch for real.
//!
//! The assertion wrappers mirror the SDK's signatures (`$message` must be a string literal) and
//! are macros, not functions, so each assertion's location is captured at the real call site.
//!
//! `observe!` and `GhostState` are ported from `precept` (orbitinghail/precept): `observe!` runs
//! a read-only property block whose `Fn` bound makes the compiler reject any code that mutates
//! the observed system, and `GhostState<T>` is auxiliary property-only state erased from
//! non-instrumented builds. Here they sit on top of the official SDK rather than precept.

// Re-export the SDK so the `#[macro_export]` wrappers below can reach it from a consumer crate
// that does not depend on `antithesis_sdk` directly.
#[cfg(feature = "enabled")]
#[doc(hidden)]
pub use antithesis_sdk;

/// Register the Antithesis assertion catalog for this process. Required once per process that
/// emits assertions; without it a never-hit `assert_unreachable!` would pass vacuously instead
/// of being reported. `antithesis_init` is idempotent, so it is safe to call from every process
/// / forked worker. A no-op unless `enabled`.
#[cfg(feature = "enabled")]
pub fn init() {
    antithesis_sdk::antithesis_init();
}

/// See the [`enabled` definition](init).
#[cfg(not(feature = "enabled"))]
pub fn init() {}

// Two generator macros stamp out the wrappers (each forwards to the identically-named
// `antithesis_sdk` macro when enabled, or a type-checking-only `if false { … }` when not — see
// the crate docs). `$d` is bound to `$` at each call site so the generated macro can name its own
// metavariables — the standard escape for a macro that defines a macro.

/// Generate a condition-style wrapper: `name!(condition, "message" [, &details])`.
// rustfmt cannot format a `macro_rules!` that defines a `macro_rules!` idempotently — each pass
// re-indents the nested arms further — so pin this generator's formatting by hand.
#[rustfmt::skip]
macro_rules! define_condition_assert {
    ($d:tt $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = "enabled")]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal $d(, $d details:expr)?) => {
                $crate::antithesis_sdk::$name!($d condition, $d message $d(, $d details)?)
            };
        }

        #[doc = $doc]
        #[cfg(not(feature = "enabled"))]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal $d(, $d details:expr)?) => {
                if false {
                    let _: bool = $d condition;
                    let _: &str = $d message;
                    $d(let _ = &$d details;)?
                }
            };
        }
    };
}

/// Generate a message-only wrapper: `name!("message" [, &details])`.
#[rustfmt::skip]
macro_rules! define_message_assert {
    ($d:tt $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = "enabled")]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal $d(, $d details:expr)?) => {
                $crate::antithesis_sdk::$name!($d message $d(, $d details)?)
            };
        }

        #[doc = $doc]
        #[cfg(not(feature = "enabled"))]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal $d(, $d details:expr)?) => {
                if false {
                    let _: &str = $d message;
                    $d(let _ = &$d details;)?
                }
            };
        }
    };
}

define_condition_assert!($ assert_always,
    "Assert `condition` holds every time this site runs and that it is hit at least once. `message` must be a string literal; optional `details` is a `&serde_json::Value`.");
define_condition_assert!($ assert_always_or_unreachable,
    "Like `assert_always!`, but the property still passes if the site is never hit.");
define_condition_assert!($ assert_sometimes,
    "Assert `condition` holds at least once across the run.");
define_message_assert!($ assert_reachable,
    "Assert this site is reached at least once across the run.");
define_message_assert!($ assert_unreachable,
    "Assert this site is never reached; reaching it reports a violation.");

// Ghost state + read-only observation, ported from precept (orbitinghail/precept).

/// Auxiliary *ghost state* that exists only to express properties: its inner `T` can be read
/// only through [`observe!`] and mutated only through [`GhostState::mutate`]. When `enabled` is
/// off it is a zero-sized type and every access compiles out.
#[cfg(feature = "enabled")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GhostState<T>(T);

/// See the [`enabled` definition](GhostState).
#[cfg(not(feature = "enabled"))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GhostState<T>(::core::marker::PhantomData<T>);

#[cfg(feature = "enabled")]
impl<T> GhostState<T> {
    /// Create ghost state, initializing the inner `T` with `init`. `init` is read-only (an `Fn`);
    /// when the crate is compiled out it is never called and no `T` is constructed.
    pub fn new<F: Fn() -> T>(init: F) -> Self {
        GhostState(init())
    }

    /// The sole mutator. `f` gets `&mut T`; everything it captures is read-only (`Fn`). When the
    /// crate is compiled out, `f` is type-checked but never called.
    pub fn mutate<F: Fn(&mut T)>(&mut self, f: F) {
        f(&mut self.0)
    }

    // Private — the only readers are the crate-internal `__observeN` helpers, so there is no
    // public way to obtain a `&T`: ghost state can only be read from inside an `observe!` block.
    fn inner(&self) -> &T {
        &self.0
    }
}

#[cfg(not(feature = "enabled"))]
impl<T> GhostState<T> {
    /// See the enabled definition.
    #[allow(unused_variables)]
    pub fn new<F: Fn() -> T>(init: F) -> Self {
        GhostState(::core::marker::PhantomData)
    }

    /// See the enabled definition.
    #[allow(unused_variables)]
    pub fn mutate<F: Fn(&mut T)>(&mut self, f: F) {}
}

// The per-arity `__observeN` helpers that back `observe!`. Each carries the `Fn(&T0, ..)` bound
// that enforces read-only access. When `enabled` is off the body is empty, so the closure is
// type-checked but never executed.
macro_rules! define_observe_helpers {
    ($( $name:ident ( $($ty:ident : $arg:ident),* ) ),* $(,)?) => {$(
        #[cfg(feature = "enabled")]
        #[doc(hidden)]
        #[allow(clippy::too_many_arguments)]
        pub fn $name<$($ty,)* F: Fn($(&$ty),*)>($($arg: &$crate::GhostState<$ty>,)* f: F) {
            f($($arg.inner()),*)
        }

        #[cfg(not(feature = "enabled"))]
        #[doc(hidden)]
        #[allow(unused_variables, clippy::too_many_arguments)]
        pub fn $name<$($ty,)* F: Fn($(&$ty),*)>($($arg: &$crate::GhostState<$ty>,)* f: F) {}
    )*};
}

define_observe_helpers! {
    __observe0(),
    __observe1(T0: m0),
    __observe2(T0: m0, T1: m1),
    __observe3(T0: m0, T1: m1, T2: m2),
    __observe4(T0: m0, T1: m1, T2: m2, T3: m3),
}

/// Run a read-only observation block, optionally borrowing one or more [`GhostState`]s. The
/// block is an `Fn` closure — the compiler rejects any attempt to mutate what it captures — and
/// may call the assertion macros in this crate. When `enabled` is off it is type-checked but
/// never executed. Up to 4 ghost states may be observed at once.
#[macro_export]
macro_rules! observe {
    ($closure:expr $(,)?) => {
        $crate::__observe0($closure)
    };
    ($m0:expr, $closure:expr $(,)?) => {
        $crate::__observe1(&$m0, $closure)
    };
    ($m0:expr, $m1:expr, $closure:expr $(,)?) => {
        $crate::__observe2(&$m0, &$m1, $closure)
    };
    ($m0:expr, $m1:expr, $m2:expr, $closure:expr $(,)?) => {
        $crate::__observe3(&$m0, &$m1, &$m2, $closure)
    };
    ($m0:expr, $m1:expr, $m2:expr, $m3:expr, $closure:expr $(,)?) => {
        $crate::__observe4(&$m0, &$m1, &$m2, &$m3, $closure)
    };
}

#[cfg(test)]
mod tests {
    use crate::GhostState;

    // Compiles and runs in both feature configurations. With `enabled` off the closures are
    // type-checked but never executed and `GhostState` is zero-sized; either way this must not
    // panic and the assertion macros must type-check.
    #[test]
    fn compiles_and_runs_in_all_configs() {
        crate::init();

        let mut seen = GhostState::new(|| 0i64);
        seen.mutate(|n| *n += 1);

        // NB: the assert wrappers are macro-generated `#[macro_export]` macros, which cannot be
        // referred to by an absolute path (`crate::assert_always!`) from inside this crate — so
        // call them unqualified (they are in textual scope). Consumer crates reference them
        // cross-crate as `dst::assert_*!`, which is unaffected.
        crate::observe!(seen, |n: &i64| {
            assert_always!(
                *n >= 0,
                "seen count is never negative",
                &::serde_json::json!({ "n": *n })
            );
        });

        crate::observe!(|| {
            assert_reachable!("observation ran");
            assert_unreachable!("should not construct impossible state");
            assert_sometimes!(true, "sometimes true");
            assert_always_or_unreachable!(1 + 1 == 2, "arithmetic holds");
        });
    }
}
