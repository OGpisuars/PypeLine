//! Time of day, driven by the factory's own tick counter (never the wall
//! clock), so it is deterministic and replayable (roadmap: DAY / NIGHT).

/// Ticks in one full day: four minutes of game time.
pub const DAY_TICKS: u64 = 20 * 60 * 4;
/// A new factory starts at this hour, so new players see the day first.
const START_HOUR: u64 = 6;

/// Part of the day, for the sky and the forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Dawn,
    Day,
    Dusk,
    Night,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dawn => "dawn",
            Self::Day => "day",
            Self::Dusk => "dusk",
            Self::Night => "night",
        }
    }
}

/// Minutes since midnight (0-1439) after `ticks` factory ticks.
pub fn minute_of_day(ticks: u64) -> u64 {
    let start = START_HOUR * DAY_TICKS / 24;
    (ticks + start) % DAY_TICKS * 1440 / DAY_TICKS
}

/// The hour (0-23) after `ticks` factory ticks.
pub fn hour(ticks: u64) -> u64 {
    minute_of_day(ticks) / 60
}

/// Day is 6:00 to 18:00.
pub fn is_day(ticks: u64) -> bool {
    (6..18).contains(&hour(ticks))
}

pub fn phase(ticks: u64) -> Phase {
    match hour(ticks) {
        5 => Phase::Dawn,
        6..=16 => Phase::Day,
        17..=19 => Phase::Dusk,
        _ => Phase::Night,
    }
}

/// Air temperature in degrees: 15 at night, rising to 30 at noon.
pub fn air_temperature(ticks: u64) -> u32 {
    let h = hour(ticks) as i64;
    if (6..18).contains(&h) {
        15 + ((6 - (h - 12).abs()) * 15 / 6) as u32
    } else {
        15
    }
}

/// Game seconds until the next day or night starts.
pub fn seconds_until_change(ticks: u64) -> u64 {
    let minute = minute_of_day(ticks);
    let next = if minute < 6 * 60 {
        6 * 60
    } else if minute < 18 * 60 {
        18 * 60
    } else {
        30 * 60
    };
    // One game minute of clock time is DAY_TICKS / 1440 ticks.
    (next - minute) * DAY_TICKS / 1440 / 20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_goes_round() {
        assert_eq!(hour(0), 6);
        assert!(is_day(0));
        assert_eq!(hour(DAY_TICKS / 4), 12);
        assert_eq!(air_temperature(DAY_TICKS / 4), 30);
        assert_eq!(hour(DAY_TICKS / 2), 18);
        assert!(!is_day(DAY_TICKS / 2));
        assert_eq!(air_temperature(DAY_TICKS / 2), 15);
        assert_eq!(hour(DAY_TICKS), 6);
        assert_eq!(phase(DAY_TICKS * 3 / 4), Phase::Night);
        assert_eq!(seconds_until_change(0), 120);
    }
}
