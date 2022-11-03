// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2022 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use pyo3::ffi;
use std::rc::Rc;

use crate::enums::MessageCategory;
use nautilus_core::string::{pystr_to_string, string_to_pystr};
use nautilus_core::time::{Timedelta, Timestamp};
use nautilus_core::uuid::UUID4;

#[repr(C)]
#[derive(Clone, Debug)]
#[allow(clippy::redundant_allocation)] // C ABI compatibility
/// Represents a time event occurring at the event timestamp.
pub struct TimeEvent {
    /// The event name.
    pub name: Box<Rc<String>>,
    /// The event ID.
    pub category: MessageCategory, // Only applicable to generic messages in the future
    /// The UNIX timestamp (nanoseconds) when the time event occurred.
    pub event_id: UUID4,
    /// The message category
    pub ts_event: Timestamp,
    /// The UNIX timestamp (nanoseconds) when the object was initialized.
    pub ts_init: Timestamp,
}

impl PartialEq for TimeEvent {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.ts_event == other.ts_event
    }
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct Vec_TimeEvent {
    pub ptr: *const TimeEvent,
    pub len: usize,
}

////////////////////////////////////////////////////////////////////////////////
// C API
////////////////////////////////////////////////////////////////////////////////
/// # Safety
/// - Assumes `name` is borrowed from a valid Python UTF-8 `str`.
#[no_mangle]
pub unsafe extern "C" fn time_event_new(
    name: *mut ffi::PyObject,
    event_id: UUID4,
    ts_event: u64,
    ts_init: u64,
) -> TimeEvent {
    TimeEvent {
        name: Box::new(Rc::new(pystr_to_string(name))),
        category: MessageCategory::Event,
        event_id,
        ts_event,
        ts_init,
    }
}

#[no_mangle]
pub extern "C" fn time_event_copy(event: &TimeEvent) -> TimeEvent {
    event.clone()
}

#[no_mangle]
pub extern "C" fn time_event_free(event: TimeEvent) {
    drop(event); // Memory freed here
}

/// Returns a pointer to a valid Python UTF-8 string.
///
/// # Safety
/// - Assumes that since the data is originating from Rust, the GIL does not need
/// to be acquired.
/// - Assumes you are immediately returning this pointer to Python.
#[no_mangle]
pub unsafe extern "C" fn time_event_name(event: &TimeEvent) -> *mut ffi::PyObject {
    string_to_pystr(event.name.as_str())
}

/// Represents a bundled event and it's handler.
pub struct TimeEventHandler {
    /// A [TimeEvent] generated by a timer.
    pub event: TimeEvent,
    /// A callable handler for this time event.
    pub handler: &'static dyn Fn(TimeEvent),
}

// TODO(cs): Implement
// impl TimeEventHandler {
//     #[inline]
//     pub fn handle(&self) {
//         self.handler.call((self.event,));
//     }
// }

pub trait Timer {
    fn new(
        name: String,
        interval_ns: Timedelta,
        start_time_ns: Timestamp,
        stop_time_ns: Option<Timestamp>,
    ) -> Self;
    fn pop_event(&self, event_id: UUID4, ts_init: Timestamp) -> TimeEvent;
    fn iterate_next_time(&mut self, ts_now: Timestamp);
    fn cancel(&mut self);
}

#[derive(Clone)]
pub struct TestTimer {
    pub name: String,
    pub interval_ns: u64,
    pub start_time_ns: Timestamp,
    pub stop_time_ns: Option<Timestamp>,
    pub next_time_ns: Timestamp,
    pub is_expired: bool,
}

impl TestTimer {
    pub fn new(
        name: String,
        interval_ns: u64,
        start_time_ns: Timestamp,
        stop_time_ns: Option<Timestamp>,
    ) -> Self {
        TestTimer {
            name,
            interval_ns,
            start_time_ns,
            stop_time_ns,
            next_time_ns: start_time_ns + interval_ns as u64,
            is_expired: false,
        }
    }

    pub fn pop_event(&self, event_id: UUID4, ts_init: Timestamp) -> TimeEvent {
        TimeEvent {
            name: Box::new(Rc::new(self.name.clone())),
            category: MessageCategory::Event,
            event_id,
            ts_event: self.next_time_ns,
            ts_init,
        }
    }

    /// Advance the test timer forward to the given time, generating a sequence
    /// of events. A [TimeEvent] is appended for each time a next event is
    /// <= the given `to_time_ns`.
    pub fn advance(&mut self, to_time_ns: Timestamp) -> impl Iterator<Item = TimeEvent> + '_ {
        let advances =
            to_time_ns.saturating_sub(self.next_time_ns - self.interval_ns) / self.interval_ns;
        self.take(advances as usize).map(|(event, _)| event)
    }

    /// Cancels the timer (the timer will not generate an event).
    pub fn cancel(&mut self) {
        self.is_expired = true;
    }
}

impl Iterator for TestTimer {
    type Item = (TimeEvent, Timestamp);

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_expired {
            None
        } else {
            let item = (
                TimeEvent {
                    name: Box::new(Rc::new(self.name.clone())),
                    category: MessageCategory::Event,
                    event_id: UUID4::new(),
                    ts_event: self.next_time_ns,
                    ts_init: self.next_time_ns,
                },
                self.next_time_ns,
            );

            // If current next event time has exceeded stop time, then expire timer
            if let Some(stop_time_ns) = self.stop_time_ns {
                if self.next_time_ns >= stop_time_ns {
                    self.is_expired = true;
                }
            }

            self.next_time_ns += self.interval_ns as u64;

            Some(item)
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::{TestTimer, TimeEvent};

    #[test]
    fn test_pop_event() {
        let name = String::from("test_timer");
        let mut timer = TestTimer::new(name, 0, 1, None);

        assert!(timer.next().is_some());
        assert!(timer.next().is_some());
        timer.is_expired = true;
        assert!(timer.next().is_none());
    }

    #[test]
    fn test_advance_within_next_time_ns() {
        let name = String::from("test_timer");
        let mut timer = TestTimer::new(name, 5, 0, None);

        let _: Vec<TimeEvent> = timer.advance(1).collect();
        let _: Vec<TimeEvent> = timer.advance(2).collect();
        let _: Vec<TimeEvent> = timer.advance(3).collect();
        let events: Vec<TimeEvent> = timer.advance(4).collect();

        assert_eq!(events.len(), 0);
        assert_eq!(timer.next_time_ns, 5);
        assert_eq!(timer.is_expired, false)
    }

    #[test]
    fn test_advance_up_to_next_time_ns() {
        let name = String::from("test_timer");
        let mut timer = TestTimer::new(name, 1, 0, None);
        let events: Vec<TimeEvent> = timer.advance(1).collect();

        assert_eq!(events.len(), 1);
        assert_eq!(timer.is_expired, false);
    }

    #[test]
    fn test_advance_up_to_next_time_ns_with_stop_time() {
        let name = String::from("test_timer");
        let mut timer = TestTimer::new(name, 1, 0, Some(2));
        let events: Vec<TimeEvent> = timer.advance(2).collect();

        assert_eq!(events.len(), 2);
        assert_eq!(timer.is_expired, true);
    }

    #[test]
    fn test_advance_beyond_next_time_ns() {
        let name = String::from("test_timer");
        let mut timer = TestTimer::new(name, 1, 0, Some(5));
        let events: Vec<TimeEvent> = timer.advance(5).collect();

        assert_eq!(events.len(), 5);
        assert_eq!(timer.is_expired, true);
    }

    #[test]
    fn test_advance_beyond_stop_time() {
        let name = String::from("test_timer");
        let mut timer = TestTimer::new(name, 1, 0, Some(5));
        let events: Vec<TimeEvent> = timer.advance(10).collect();

        assert_eq!(events.len(), 5);
        assert_eq!(timer.is_expired, true);
    }
}