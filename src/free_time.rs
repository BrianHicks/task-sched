use chrono::{DateTime, TimeZone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreeTime<TZ: TimeZone> {
    Blocked,
    Single(DateTimeRange<TZ>),
}

impl<TZ: TimeZone> FreeTime<TZ> {
    pub fn new(start: DateTime<TZ>, end: DateTime<TZ>) -> Self {
        Self::Single(DateTimeRange { start, end })
    }

    pub fn block(&self, range: &DateTimeRange<TZ>) -> Self {
        match self {
            Self::Blocked => Self::Blocked,
            Self::Single(single) => single.block(&range),
        }
    }
}

/// a "half-open" date range. That is to say: a `DateTimeRange` includes `start`
/// and all moments right up to but not including `end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTimeRange<TZ: TimeZone> {
    start: DateTime<TZ>,
    end: DateTime<TZ>,
}

impl<TZ: TimeZone> DateTimeRange<TZ> {
    fn new(start: DateTime<TZ>, end: DateTime<TZ>) -> Self {
        Self { start, end }
    }

    fn block(&self, other: &Self) -> FreeTime<TZ> {
        // easy case: the ranges don't overlap at all
        if !self.overlaps(other) {
            FreeTime::Single(self.clone())
        } else {
            FreeTime::Blocked
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::{Duration, Utc};
    use proptest::prelude::*;
    use std::ops::Range;

    prop_compose! {
        fn time(range: Range<i64>)(timestamp in range) -> DateTime<Utc> {
            Utc.timestamp_opt(timestamp, 0).unwrap()
        }
    }

    prop_compose! {
        fn date_time_range(range: Range<i64>)(a in time(range.clone()), b in time(range)) -> DateTimeRange<Utc> {
            let start = a.min(b);
            let mut end = a.max(b);

            if start == end {
                end += Duration::seconds(1);
            }

            DateTimeRange::new(start, end)
        }
    }

    fn free_time(range: Range<i64>) -> impl Strategy<Value = FreeTime<Utc>> {
        prop_oneof![
            1 => Just(FreeTime::Blocked),
            5 => date_time_range(range).prop_map(FreeTime::Single)
        ]
    }

    mod date_time_range {
        use super::*;

        mod block {
            use chrono::Duration;

            use super::*;

            proptest! {
                #[test]
                fn completely_before_is_single(before in date_time_range(0..5), after in date_time_range(5..10)) {
                    assert_eq!(after.block(&before), FreeTime::Single(after))
                }
            }

            proptest! {
                #[test]
                fn completely_after_is_single(before in date_time_range(0..5), after in date_time_range(5..10)) {
                    assert_eq!(before.block(&after), FreeTime::Single(before))
                }
            }

            proptest! {
                #[test]
                fn totally_blocks(range in date_time_range(0..1), margin in 0..1i64) {
                    assert_eq!(
                        range.block(&DateTimeRange::new(
                            range.start - Duration::microseconds(margin),
                            range.end + Duration::microseconds(margin),
                        )),
                        FreeTime::Blocked
                    )
                }
            }
        }
    }

    // mod block {
    //     use super::FreeTime::*;
    //     use super::*;

    //     proptest! {
    //         #[test]
    //         fn blocked_remains_blocked(start in time(0..50), end in time(50..100)) {
    //             assert_eq!(Blocked.block(start, end), Blocked)
    //         }

    //         #[test]
    //         fn end_after_start_has_no_effect(start in time(5..10), end in time(0..5), ft in free_time(0..10)) {
    //             assert_eq!(ft.block(start, end), ft)
    //         }

    //         #[test]
    //         fn wider_range_totally_blocks(ft in free_time(0..10)) {
    //             assert_eq!()
    //         }
    //     }
    // }
}
