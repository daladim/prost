#![doc(html_root_url = "https://docs.rs/prost-types/0.6.1")]

//! Protocol Buffers well-known types.
//!
//! Note that the documentation for the types defined in this crate are generated from the Protobuf
//! definitions, so code examples are not in Rust.
//!
//! See the [Protobuf reference][1] for more information about well-known types.
//!
//! [1]: https://developers.google.com/protocol-buffers/docs/reference/google.protobuf

use std::convert::TryFrom;
use std::i32;
use std::i64;
use std::time;

include!("protobuf.rs");
pub mod compiler {
    include!("compiler.rs");
}

// The Protobuf `Duration` and `Timestamp` types can't delegate to the standard library equivalents
// because the Protobuf versions are signed. To make them easier to work with, `From` conversions
// are defined in both directions.

const NANOS_PER_SECOND: i32 = 1_000_000_000;

impl Duration {
    /// Normalizes the duration to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L79-L100
    fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            self.seconds += (self.nanos / NANOS_PER_SECOND) as i64;
            self.nanos %= NANOS_PER_SECOND;
        }

        // nanos should have the same sign as seconds.
        if self.seconds < 0 && self.nanos > 0 {
            self.seconds += 1;
            self.nanos -= NANOS_PER_SECOND;
        } else if self.seconds > 0 && self.nanos < 0 {
            self.seconds -= 1;
            self.nanos += NANOS_PER_SECOND;
        }
        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -315_576_000_000 && self.seconds <= 315_576_000_000,
        //               "invalid duration: {:?}", self);
    }
}

/// Converts a `std::time::Duration` to a `Duration`.
impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Duration {
        let seconds = duration.as_secs();
        let seconds = if seconds > i64::MAX as u64 {
            i64::MAX
        } else {
            seconds as i64
        };
        let nanos = duration.subsec_nanos();
        let nanos = if nanos > i32::MAX as u32 {
            i32::MAX
        } else {
            nanos as i32
        };
        let mut duration = Duration { seconds, nanos };
        duration.normalize();
        duration
    }
}

impl TryFrom<Duration> for time::Duration {
    type Error = time::Duration;

    /// Converts a `Duration` to a result containing a positive (`Ok`) or negative (`Err`)
    /// `std::time::Duration`.
    fn try_from(mut duration: Duration) -> Result<time::Duration, time::Duration> {
        duration.normalize();
        if duration.seconds >= 0 {
            Ok(time::Duration::new(
                duration.seconds as u64,
                duration.nanos as u32,
            ))
        } else {
            Err(time::Duration::new(
                (-duration.seconds) as u64,
                (-duration.nanos) as u32,
            ))
        }
    }
}

impl Timestamp {
    /// Normalizes the timestamp to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L59-L77
    fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            self.seconds += (self.nanos / NANOS_PER_SECOND) as i64;
            self.nanos %= NANOS_PER_SECOND;
        }

        // For Timestamp nanos should be in the range [0, 999999999].
        if self.nanos < 0 {
            self.seconds -= 1;
            self.nanos += NANOS_PER_SECOND;
        }

        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -62_135_596_800 && self.seconds <= 253_402_300_799,
        //               "invalid timestamp: {:?}", self);
    }
}

/// Converts a `chrono::DateTime` to a `Timestamp`.
#[cfg(feature = "chrono-conversions")]
impl<Tz: chrono::TimeZone> From<chrono::DateTime<Tz>> for Timestamp {
    fn from(dt: chrono::DateTime<Tz>) -> Self {
        Self{
            seconds: dt.timestamp() as _,
            nanos: dt.timestamp_subsec_nanos() as _,
        }
    }
}

/// Converts a `Timestamp` to a `chrono::DateTime`.
#[cfg(feature = "chrono-conversions")]
impl Into<chrono::DateTime<chrono::Utc>> for Timestamp {
    fn into(self) -> chrono::DateTime<chrono::Utc> {
        use chrono::TimeZone;
        chrono::Utc.timestamp(self.seconds, self.nanos as _)
    }
}

/// Converts a `std::time::SystemTime` to a `Timestamp`.
impl From<time::SystemTime> for Timestamp {
    fn from(time: time::SystemTime) -> Timestamp {
        let duration = Duration::from(time.duration_since(time::UNIX_EPOCH).unwrap());
        Timestamp {
            seconds: duration.seconds,
            nanos: duration.nanos,
        }
    }
}

impl TryFrom<Timestamp> for time::SystemTime {
    type Error = time::Duration;

    /// Converts a `Timestamp` to a `SystemTime`, or if the timestamp falls before the Unix epoch,
    /// a duration containing the difference.
    fn try_from(mut timestamp: Timestamp) -> Result<time::SystemTime, time::Duration> {
        timestamp.normalize();
        if timestamp.seconds >= 0 {
            Ok(time::UNIX_EPOCH
                + time::Duration::new(timestamp.seconds as u64, timestamp.nanos as u32))
        } else {
            let mut duration = Duration {
                seconds: -timestamp.seconds,
                nanos: timestamp.nanos,
            };
            duration.normalize();
            Err(time::Duration::new(
                duration.seconds as u64,
                duration.nanos as u32,
            ))
        }
    }
}



mod test {
    #[test]
    #[cfg(feature = "chrono-conversions")]
    fn test_datetime_to_wkt_timestamp() {
        use super::*;
        use chrono::{Utc, DateTime, TimeZone};

        let date_utc = Utc.ymd(2014, 7, 8).and_hms_nano(9, 10, 11, 12);
        let ts0: Timestamp = date_utc.into();
        let expected0 = Timestamp{seconds: 1404810611, nanos: 12};
        assert_eq!(ts0, expected0);

        let date_6 = DateTime::parse_from_rfc2822("8 Jul 2014 09:10:11 +0600").unwrap();
        let ts6: Timestamp = date_6.into();
        let expected6 = Timestamp{seconds: 1404810611 - 6*3600, nanos: 0};
        assert_eq!(ts6, expected6);
    }

    #[test]
    #[cfg(feature = "chrono-conversions")]
    fn test_wkt_timestamp_to_datetime() {
        use super::*;
        use chrono::{Utc, DateTime, TimeZone};

        let ts = Timestamp{seconds: 1404810611, nanos: 12};
        let expected_date = Utc.ymd(2014, 7, 8).and_hms_nano(9, 10, 11, 12);
        let date: DateTime<Utc> = ts.into();
        assert_eq!(date, expected_date);
    }
}
