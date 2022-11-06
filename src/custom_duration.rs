use crate::sync_error::SyncError;
use chrono::Duration;
use serde::Deserialize;
use std::convert::TryFrom;

#[derive(Debug, Deserialize, Clone)]
pub struct CustomDuration {
    minutes: Option<i64>,
    hours: Option<i64>,
    days: Option<i64>,
    weeks: Option<i64>,
}

impl CustomDuration {
    pub fn minutes(minutes: i64) -> Self {
        CustomDuration {
            minutes: Some(minutes),
            hours: None,
            days: None,
            weeks: None,
        }
    }

    pub fn hours(hours: i64) -> Self {
        CustomDuration {
            minutes: None,
            hours: Some(hours),
            days: None,
            weeks: None,
        }
    }

    pub fn days(days: i64) -> Self {
        CustomDuration {
            minutes: None,
            hours: None,
            days: Some(days),
            weeks: None,
        }
    }

    pub fn weeks(weeks: i64) -> Self {
        CustomDuration {
            minutes: None,
            hours: None,
            days: None,
            weeks: Some(weeks),
        }
    }
}

impl TryFrom<&CustomDuration> for Duration {
    type Error = SyncError;

    fn try_from(cd: &CustomDuration) -> Result<Self, Self::Error> {
        let mut dur = Duration::nanoseconds(0);

        if cd.minutes.is_some() {
            dur = dur
                .checked_add(&Duration::minutes(cd.minutes.unwrap()))
                .ok_or(SyncError::DurationConversionError)?
        }

        if cd.hours.is_some() {
            dur = dur
                .checked_add(&Duration::hours(cd.hours.unwrap()))
                .ok_or(SyncError::DurationConversionError)?
        }

        if cd.days.is_some() {
            dur = dur
                .checked_add(&Duration::days(cd.days.unwrap()))
                .ok_or(SyncError::DurationConversionError)?
        }

        if cd.weeks.is_some() {
            dur = dur
                .checked_add(&Duration::weeks(cd.weeks.unwrap()))
                .ok_or(SyncError::DurationConversionError)?
        }

        Ok(dur)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::custom_duration::CustomDuration;
    use chrono::Duration;

    #[test]
    fn convert_duration_1() {
        let test: Duration = (&CustomDuration::minutes(2)).try_into().unwrap();
        assert_eq!(Duration::minutes(2), test);
    }

    #[test]
    fn convert_duration_2() {
        let test: Duration = (&CustomDuration::hours(1)).try_into().unwrap();
        assert_eq!(Duration::hours(1), test);
    }

    #[test]
    fn convert_duration_3() {
        let test: Duration = (&CustomDuration::days(4)).try_into().unwrap();
        assert_eq!(Duration::days(4), test);
    }

    #[test]
    fn convert_duration_4() {
        let test: Duration = (&CustomDuration::weeks(5)).try_into().unwrap();
        assert_eq!(Duration::weeks(5), test);
    }
}
