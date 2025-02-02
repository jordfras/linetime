#[cfg(test)]
use std::collections::VecDeque;
#[cfg(not(test))]
use std::time::SystemTime;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Timestamp {
    previous_time: Option<Duration>,
    #[cfg(not(test))]
    start_time: SystemTime,
    #[cfg(test)]
    // Fake timestamps
    expected_stamps: VecDeque<Duration>,
}

#[cfg(not(test))]
impl Timestamp {
    pub fn new() -> Self {
        Self {
            previous_time: None,
            start_time: SystemTime::now(),
        }
    }

    pub fn get(&mut self) -> Duration {
        let t = SystemTime::now()
            .duration_since(self.start_time)
            .expect("Start time should be earlier than get");
        self.previous_time = Some(t);
        t
    }
}

impl Timestamp {
    pub fn previous(&self) -> Option<Duration> {
        self.previous_time
    }
}

#[cfg(test)]
// Fake implementation for unit testing
impl Timestamp {
    pub fn new() -> Self {
        Self {
            previous_time: None,
            expected_stamps: VecDeque::new(),
        }
    }

    pub fn expect_get(&mut self, stamp: Duration) {
        self.expected_stamps.push_back(stamp);
    }

    pub fn get(&mut self) -> Duration {
        let t = self
            .expected_stamps
            .pop_front()
            .expect("Unexpected request for timestamp");
        self.previous_time = Some(t);
        t
    }

    pub fn assert_all_used(&self) {
        assert!(
            self.expected_stamps.is_empty(),
            "All expected timestamps where not requested: {:?}",
            self.expected_stamps
        );
    }
}

/// Gets a timestamp and creates a string suitable for prefixing output lines
pub fn create_prefix(
    timestamp: &Arc<Mutex<Timestamp>>,
    with_delta: bool,
    microseconds: bool,
) -> String {
    let Ok(mut guard) = timestamp.lock() else {
        // If other thread has panicked, we return a string of correct length with spaces instead
        return " ".repeat(stamp_length(with_delta, microseconds));
    };
    let previous_time = guard.previous();
    let time = guard.get();
    drop(guard);

    let mut result = format(time, microseconds);
    if with_delta {
        result += if let Some(previous_time) = previous_time {
            format!(" ({})", format(time - previous_time, microseconds))
        } else {
            " ".repeat(duration_length(microseconds) + 3)
        }
        .as_str();
    }
    result
}

fn subsec_length(microseconds: bool) -> usize {
    if microseconds {
        6
    } else {
        3
    }
}

/// The string length of a single duration string
fn duration_length(microseconds: bool) -> usize {
    2 + 1 + 2 + 1 + subsec_length(microseconds)
}

/// The string length of a complete timestamp string
fn stamp_length(with_delta: bool, microseconds: bool) -> usize {
    if with_delta {
        duration_length(microseconds) * 2 + 3
    } else {
        duration_length(microseconds)
    }
}

fn format(duration: Duration, microseconds: bool) -> String {
    if microseconds {
        format!(
            "{:0>2}:{:0>2}.{:0>6}",
            duration.as_secs() / 60,
            duration.as_secs() % 60,
            duration.subsec_micros()
        )
    } else {
        format!(
            "{:0>2}:{:0>2}.{:0>3}",
            duration.as_secs() / 60,
            duration.as_secs() % 60,
            duration.subsec_millis()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_expected_stamps_in_order() {
        let mut t = Timestamp::new();
        t.expect_get(Duration::from_millis(1234));
        t.expect_get(Duration::from_millis(2345));

        assert_eq!(Duration::from_millis(1234), t.get());
        assert_eq!(Duration::from_millis(2345), t.get());
    }

    #[test]
    fn previous_returns_previously_gotten_stamp() {
        let mut t = Timestamp::new();
        t.expect_get(Duration::from_millis(1234));
        t.expect_get(Duration::from_millis(2345));

        assert_eq!(None, t.previous());
        assert_eq!(None, t.previous());

        assert_eq!(Duration::from_millis(1234), t.get());
        assert_eq!(Some(Duration::from_millis(1234)), t.previous());
        assert_eq!(Some(Duration::from_millis(1234)), t.previous());

        assert_eq!(Duration::from_millis(2345), t.get());
        assert_eq!(Some(Duration::from_millis(2345)), t.previous());
        assert_eq!(Some(Duration::from_millis(2345)), t.previous());
    }

    #[test]
    #[should_panic(expected = "Unexpected request for timestamp")]
    fn get_more_than_available_panics() {
        let mut t = Timestamp::new();
        t.get();
    }

    #[test]
    #[should_panic(expected = "All expected timestamps where not requested: [1.234s]")]
    fn not_getting_all_timestamps_panics_when_checked() {
        let mut t = Timestamp::new();
        t.expect_get(Duration::from_millis(1234));
        t.assert_all_used();
    }
}
