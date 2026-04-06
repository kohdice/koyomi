use chrono::{DateTime, FixedOffset, Local, NaiveDate, TimeDelta, Utc};

use crate::{CalendarError, Result};

/// Timezone for time range calculation.
///
/// Determines how "midnight" (00:00:00) is interpreted when converting
/// to UTC for Google Calendar API queries (`timeMin`/`timeMax`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeZone {
    /// System timezone (chrono::Local)
    #[default]
    Local,
    /// Japan Standard Time (UTC+9:00)
    Jst,
    /// Coordinated Universal Time (UTC+0:00)
    Utc,
}

impl TimeZone {
    /// Return the fixed UTC offset for this timezone.
    ///
    /// **Caution:** For `Local`, this captures the offset at the *current moment*.
    /// If the system timezone observes DST, the returned offset may not match
    /// the offset at an arbitrary event timestamp. Prefer [`convert_datetime`]
    /// for converting event times.
    ///
    /// [`convert_datetime`]: TimeZone::convert_datetime
    #[must_use]
    pub fn fixed_offset(self) -> FixedOffset {
        match self {
            Self::Local => *Local::now().offset(),
            Self::Jst => FixedOffset::east_opt(9 * 3600).expect("JST offset is always valid"),
            Self::Utc => FixedOffset::east_opt(0).expect("UTC offset is always valid"),
        }
    }

    /// Convert a `DateTime<FixedOffset>` to the local representation in this timezone.
    ///
    /// For `Local`, uses `chrono::Local` which resolves DST at the target timestamp.
    /// For `Jst`/`Utc`, delegates to the fixed offset (no DST).
    #[must_use]
    pub fn convert_datetime(self, dt: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
        match self {
            Self::Local => dt.with_timezone(&Local).fixed_offset(),
            _ => dt.with_timezone(&self.fixed_offset()),
        }
    }

    /// Return today's date in this timezone.
    #[must_use]
    pub fn today(self) -> NaiveDate {
        match self {
            Self::Local => chrono::Local::now().date_naive(),
            _ => chrono::Utc::now().with_timezone(&self.fixed_offset()).date_naive(),
        }
    }
}

/// Convert midnight (00:00:00) of the given date in the specified timezone to UTC.
fn midnight_to_utc(base: NaiveDate, tz: TimeZone) -> Result<DateTime<Utc>> {
    let midnight = base.and_hms_opt(0, 0, 0).expect("midnight is always valid");
    let err = || CalendarError::InvalidTime("timezone conversion failed".into());
    let utc = match tz {
        TimeZone::Local => {
            midnight.and_local_timezone(Local).single().ok_or_else(err)?.with_timezone(&Utc)
        }
        _ => midnight
            .and_local_timezone(tz.fixed_offset())
            .single()
            .ok_or_else(err)?
            .with_timezone(&Utc),
    };
    Ok(utc)
}

/// Return a 1-day time range starting from `base`.
///
/// # Errors
///
/// Returns an error if the timezone conversion fails.
pub fn for_day(base: NaiveDate, tz: TimeZone) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start = midnight_to_utc(base, tz)?;
    let end = start + TimeDelta::days(1);
    Ok((start, end))
}

/// Return a 7-day time range starting from `base`.
///
/// # Errors
///
/// Returns an error if the timezone conversion fails.
pub fn for_week(base: NaiveDate, tz: TimeZone) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start = midnight_to_utc(base, tz)?;
    let end = start + TimeDelta::days(7);
    Ok((start, end))
}

/// Return a 1-calendar-month time range starting from `base`.
///
/// # Errors
///
/// Returns an error if the timezone conversion or month addition fails.
pub fn for_month(base: NaiveDate, tz: TimeZone) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start = midnight_to_utc(base, tz)?;
    let end = start
        .checked_add_months(chrono::Months::new(1))
        .ok_or_else(|| CalendarError::InvalidTime("failed to add 1 calendar month".into()))?;
    Ok((start, end))
}

/// Return a time range spanning `months` calendar months starting from
/// the 1st day of the given `year`/`month`.
///
/// # Errors
///
/// Returns an error if the year/month is invalid, timezone conversion fails,
/// or the month addition overflows.
pub fn for_month_range(
    year: i32,
    month: u32,
    months: u32,
    tz: TimeZone,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start_date = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| CalendarError::InvalidTime(format!("invalid year/month: {year}/{month}")))?;
    let start = midnight_to_utc(start_date, tz)?;
    let end = start
        .checked_add_months(chrono::Months::new(months))
        .ok_or_else(|| CalendarError::InvalidTime(format!("failed to add {months} months")))?;
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;

    #[test]
    fn timezone_default_is_local() {
        assert_eq!(TimeZone::default(), TimeZone::Local);
    }

    #[test]
    fn for_day_returns_24_hours() {
        let base = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let (start, end) = for_day(base, TimeZone::Jst).unwrap();
        assert_eq!((end - start).num_hours(), 24);
    }

    #[test]
    fn for_week_returns_7_days() {
        let base = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let (start, end) = for_week(base, TimeZone::Jst).unwrap();
        assert_eq!((end - start).num_days(), 7);
    }

    #[test]
    fn for_month_february_non_leap() {
        let base = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let (start, end) = for_month(base, TimeZone::Jst).unwrap();
        assert_eq!((end - start).num_days(), 28);
    }

    #[test]
    fn for_month_february_leap() {
        let base = NaiveDate::from_ymd_opt(2028, 2, 1).unwrap();
        let (start, end) = for_month(base, TimeZone::Jst).unwrap();
        assert_eq!((end - start).num_days(), 29);
    }

    #[test]
    fn for_month_january_31_days() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let (start, end) = for_month(base, TimeZone::Jst).unwrap();
        assert_eq!((end - start).num_days(), 31);
    }

    #[test]
    fn for_month_range_single_month() {
        let (start, end) = for_month_range(2026, 2, 1, TimeZone::Jst).unwrap();
        assert_eq!((end - start).num_days(), 28);
    }

    #[test]
    fn for_month_range_13_months_from_august() {
        let (start, end) = for_month_range(2025, 8, 13, TimeZone::Jst).unwrap();
        let days = (end - start).num_days();
        // 2025/8 ~ 2026/8: 31+30+31+30+31+31+28+31+30+31+30+31+31 = 396 days
        assert_eq!(days, 396);
    }

    #[test]
    fn for_month_range_year_boundary() {
        let (start, end) = for_month_range(2025, 11, 13, TimeZone::Jst).unwrap();
        let days = (end - start).num_days();
        // 2025/11 ~ 2026/11: 30+31+31+28+31+30+31+30+31+31+30+31+30 = 395 days
        assert_eq!(days, 395);
    }

    #[test]
    fn for_month_range_invalid_month() {
        let result = for_month_range(2026, 13, 1, TimeZone::Jst);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("invalid year/month"), "Expected 'invalid year/month' in: {error}");
    }

    #[test]
    fn midnight_to_utc_produces_utc() {
        let base = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let dt = midnight_to_utc(base, TimeZone::Jst).unwrap();
        assert_eq!(dt.timezone(), Utc);
    }

    #[test]
    fn midnight_to_utc_jst_is_15_hours_earlier() {
        // 2026-02-16 00:00 JST = 2026-02-15 15:00 UTC
        let base = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let dt = midnight_to_utc(base, TimeZone::Jst).unwrap();
        assert_eq!(dt.date_naive(), NaiveDate::from_ymd_opt(2026, 2, 15).unwrap());
        assert_eq!(dt.time(), chrono::NaiveTime::from_hms_opt(15, 0, 0).unwrap());
    }

    #[test]
    fn midnight_to_utc_utc_is_midnight() {
        // 2026-02-16 00:00 UTC = 2026-02-16 00:00 UTC
        let base = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let dt = midnight_to_utc(base, TimeZone::Utc).unwrap();
        assert_eq!(dt.date_naive(), NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
        assert_eq!(dt.time(), chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn for_day_jst_start_differs_from_utc_by_9_hours() {
        let base = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let (jst_start, _) = for_day(base, TimeZone::Jst).unwrap();
        let (utc_start, _) = for_day(base, TimeZone::Utc).unwrap();
        assert_eq!((utc_start - jst_start).num_hours(), 9);
    }

    #[test]
    fn convert_datetime_jst_preserves_offset() {
        let dt = DateTime::parse_from_rfc3339("2026-07-15T12:00:00+00:00").unwrap();
        let converted = TimeZone::Jst.convert_datetime(&dt);
        assert_eq!(converted.offset().local_minus_utc(), 9 * 3600);
        assert_eq!(converted.hour(), 21);
    }

    #[test]
    fn convert_datetime_utc_returns_zero_offset() {
        let dt = DateTime::parse_from_rfc3339("2026-07-15T12:00:00+09:00").unwrap();
        let converted = TimeZone::Utc.convert_datetime(&dt);
        assert_eq!(converted.offset().local_minus_utc(), 0);
        assert_eq!(converted.hour(), 3);
    }

    #[test]
    fn convert_datetime_preserves_instant() {
        let dt = DateTime::parse_from_rfc3339("2026-03-08T07:30:00+00:00").unwrap();

        let jst = TimeZone::Jst.convert_datetime(&dt);
        let utc = TimeZone::Utc.convert_datetime(&dt);
        let local = TimeZone::Local.convert_datetime(&dt);

        assert_eq!(jst.to_utc(), dt.to_utc());
        assert_eq!(utc.to_utc(), dt.to_utc());
        assert_eq!(local.to_utc(), dt.to_utc());
    }

    #[test]
    fn convert_datetime_local_preserves_instant_across_seasons() {
        let winter = DateTime::parse_from_rfc3339("2026-01-15T12:00:00+00:00").unwrap();
        let summer = DateTime::parse_from_rfc3339("2026-07-15T12:00:00+00:00").unwrap();

        let winter_local = TimeZone::Local.convert_datetime(&winter);
        let summer_local = TimeZone::Local.convert_datetime(&summer);

        assert_eq!(winter_local.to_utc(), winter.to_utc());
        assert_eq!(summer_local.to_utc(), summer.to_utc());
    }

    #[test]
    fn for_month_range_utc_single_month() {
        let (start, end) = for_month_range(2026, 3, 1, TimeZone::Utc).unwrap();
        assert_eq!((end - start).num_days(), 31);
        // UTC midnight should be exactly midnight
        assert_eq!(start.time(), chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }
}
