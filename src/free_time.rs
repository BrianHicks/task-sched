use chrono::{DateTime, TimeZone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreeTime<TZ: TimeZone> {
    Blocked,
    FromTo(DateTime<TZ>, DateTime<TZ>),
}

impl<TZ: TimeZone> FreeTime<TZ> {
    pub fn new(start: DateTime<TZ>, end: DateTime<TZ>) -> Self {
        Self::FromTo(start, end)
    }

    pub fn block(&self, block_start: DateTime<TZ>, block_end: DateTime<TZ>) -> Self {
        if block_end <= block_start {
            return self.clone();
        }

        match self {
            Self::Blocked => Self::Blocked,
            Self::FromTo(our_start, our_end) => {
                if *our_start >= block_end || *our_end < block_start {
                    self.clone()
                } else {
                    todo!()
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod block {
        use super::FreeTime::*;
        use super::*;
        use chrono::Utc;
        use proptest::prelude::*;
        use std::ops::Range;

        prop_compose! {
            fn time(range: Range<i64>)(timestamp in range) -> DateTime<Utc> {
                Utc.timestamp_opt(timestamp, 0).unwrap()
            }
        }

        fn from_to(range: Range<i64>) -> impl Strategy<Value = FreeTime<Utc>> {
            (time(range.clone()), time(range))
                .prop_map(|(start, end)| FromTo(start.min(end), start.max(end)))
        }

        fn free_time(range: Range<i64>) -> impl Strategy<Value = FreeTime<Utc>> {
            prop_oneof![
                1 => Just(Blocked),
                5 => from_to(range),
            ]
        }

        proptest! {
            #[test]
            fn blocked_remains_blocked(start in time(0..50), end in time(50..100)) {
                assert_eq!(Blocked.block(start, end), Blocked)
            }

            #[test]
            fn end_after_start_has_no_effect(start in time(5..10), end in time(0..5), ft in free_time(0..10)) {
                assert_eq!(ft.block(start, end), ft)
            }
        }
    }
}
