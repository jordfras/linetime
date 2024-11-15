#[cfg(test)]
use std::collections::VecDeque;
use std::time::Duration;
#[cfg(not(test))]
use std::time::SystemTime;

#[derive(Clone)]
pub struct Timestamp {
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
            start_time: SystemTime::now(),
        }
    }

    pub fn get(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.start_time)
            .expect("Start time should be earlier than get")
    }
}

#[cfg(test)]
// Fake implementation for unit testing
impl Timestamp {
    pub fn new() -> Self {
        Self {
            expected_stamps: VecDeque::new(),
        }
    }

    pub fn expect_get(&mut self, stamp: Duration) {
        self.expected_stamps.push_back(stamp);
    }

    pub fn get(&mut self) -> Duration {
        self.expected_stamps
            .pop_front()
            .expect("Unexpected request for timestamp")
    }

    pub fn expect_empty(&self) {
        assert!(
            self.expected_stamps.is_empty(),
            "All expected timestamps where not requested: {:?}",
            self.expected_stamps
        );
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
        t.expect_empty();
    }
}
