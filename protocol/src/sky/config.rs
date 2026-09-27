/// Real seconds in one in-game day
pub const DAY_LENGTH_SECS: f64 = 20.0 * 60.0;

/// In-game days in a year: one full round of the seasons. 365 is Earth's; fewer
/// makes the seasons turn faster (at 20 minute days, 365 days is 5 real days)
pub const DAYS_PER_YEAR: f64 = 365.0;

/// Degrees north (positive) or south of the equator. Sets how high the sun
/// climbs and how much day length changes with the seasons: 0 is the equator
/// (12 hour days all year), 45 is like southern France, 60+ has very long summer
/// days and very short winter ones
pub const LATITUDE_DEG: f64 = 45.0;

/// When a new world starts: the day of the year (0 = 1 January, 80 = about the
/// spring equinox) and the time of day (0 midnight, 0.25 six in the morning, 0.5
/// noon)
pub const START_DAY_OF_YEAR: f64 = 80.0;
pub const START_TIME_OF_DAY: f64 = 0.3;

/// Days from one new moon to the next
pub const LUNAR_MONTH_DAYS: f64 = 29.530588;
