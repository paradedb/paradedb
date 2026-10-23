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

//! Per-process observations of recovery episodes, serialized by the caller.

#[derive(Default)]
pub(super) struct Schedule {
    pause: Option<u64>,
    ever_paused: bool,
    fault_since_pause: bool,
    fault_before_pause: bool,
    case_during_pause: bool,
    recovered_episode: bool,
}

#[derive(Default)]
pub(super) struct Observation {
    pub paused: bool,
    pub ended: bool,
    pub resumed_after_recovery: bool,
    pub case_before_any_pause: bool,
}

impl Schedule {
    pub fn observe(
        &mut self,
        pause: Option<u64>,
        case_completed: bool,
        fault: bool,
    ) -> Observation {
        let mut result = Observation::default();
        if pause != self.pause {
            if self.pause.is_some() {
                result.ended = pause.is_none();
                self.recovered_episode = self.fault_before_pause && self.case_during_pause;
            }
            if pause.is_some() {
                result.paused = true;
                self.ever_paused = true;
                self.fault_before_pause = self.fault_since_pause;
                self.fault_since_pause = false;
                self.case_during_pause = false;
                // A new pause interrupts the post-recovery chaos phase.
                self.recovered_episode = false;
            }
            self.pause = pause;
        }
        if case_completed && pause.is_some() {
            self.case_during_pause = true;
        }
        if fault && pause.is_none() {
            result.resumed_after_recovery = self.recovered_episode;
            self.fault_since_pause = true;
        }
        result.case_before_any_pause =
            case_completed && !self.ever_paused && self.fault_since_pause;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_recovery_fault_marks_completed_episode() {
        let mut s = Schedule::default();
        s.observe(None, false, true);
        assert!(s.observe(Some(1), false, false).paused);
        assert!(!s.observe(Some(1), true, false).ended);
        assert!(s.observe(None, false, false).ended);
        assert!(s.observe(None, false, true).resumed_after_recovery);
    }

    #[test]
    fn early_pause_does_not_disqualify_later_recovery() {
        let mut s = Schedule::default();
        s.observe(Some(1), true, false);
        s.observe(None, false, false);
        assert!(!s.observe(None, false, true).resumed_after_recovery);
        s.observe(Some(2), false, false);
        s.observe(Some(2), true, false);
        s.observe(None, false, false);
        assert!(s.observe(None, false, true).resumed_after_recovery);
    }

    #[test]
    fn pause_without_success_is_not_survival() {
        let mut s = Schedule::default();
        s.observe(None, false, true);
        s.observe(Some(1), false, true);
        s.observe(None, false, false);
        assert!(!s.observe(None, false, true).resumed_after_recovery);
    }

    #[test]
    fn success_outside_pause_does_not_count() {
        let mut s = Schedule::default();
        s.observe(None, false, true);
        s.observe(Some(1), false, false);
        s.observe(None, true, false);
        assert!(!s.observe(None, false, true).resumed_after_recovery);
    }

    #[test]
    fn replacement_pause_requires_its_own_success() {
        let mut s = Schedule::default();
        s.observe(None, false, true);
        s.observe(Some(1), true, false);
        s.observe(Some(2), false, false);
        s.observe(None, false, false);
        assert!(!s.observe(None, false, true).resumed_after_recovery);
    }

    #[test]
    fn successful_case_before_first_pause_is_observed_only_on_success() {
        let mut s = Schedule::default();
        assert!(!s.observe(None, false, true).case_before_any_pause);
        assert!(s.observe(None, true, false).case_before_any_pause);
        s.observe(Some(1), false, false);
        assert!(!s.observe(None, true, false).case_before_any_pause);
    }
}
