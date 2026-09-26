//! Game calendar. Day 0 is 1 November 1855, the start of the MVP winter (§25).

use std::fmt;

const START_YEAR: i32 = 1855;
const START_MONTH: u32 = 11;

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Autumn,
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Lunar phase on day 0 (1 Nov 1855), from the new moon of 6 Jan 2000
/// 18:14 UTC and the mean synodic month. 0 = new, 0.5 = full.
const MOON_AT_DAY0: f32 = 0.7047;
const SYNODIC_MONTH: f32 = 29.530_589;

impl Day {
    /// 0 new .. 0.5 full .. 1 new again.
    pub fn moon_phase(self) -> f32 {
        (MOON_AT_DAY0 + self.0 as f32 / SYNODIC_MONTH).fract()
    }

    /// 0 dark .. 1 full: how much a night-rider can be seen by.
    pub fn moonlight(self) -> f32 {
        (1.0 - (std::f32::consts::TAU * self.moon_phase()).cos()) / 2.0
    }

    /// Waxing: "the light of the moon", when the almanac says to plant.
    pub fn waxing(self) -> bool {
        self.moon_phase() < 0.5
    }

    pub fn moon_name(self) -> &'static str {
        match self.moon_phase() {
            p if !(0.035..0.965).contains(&p) => "new moon",
            p if p < 0.215 => "waxing crescent",
            p if p < 0.285 => "first quarter",
            p if p < 0.465 => "waxing gibbous",
            p if p < 0.535 => "full moon",
            p if p < 0.715 => "waning gibbous",
            p if p < 0.785 => "last quarter",
            _ => "waning crescent",
        }
    }

    /// (year, month 1-12, day 1-31)
    pub fn date(self) -> (i32, u32, u32) {
        let (mut year, mut month, mut left) = (START_YEAR, START_MONTH, self.0);
        loop {
            let len = days_in_month(year, month);
            if left < len {
                return (year, month, left + 1);
            }
            left -= len;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }

    pub fn month(self) -> u32 {
        self.date().1
    }

    pub fn season(self) -> Season {
        match self.month() {
            12 | 1 | 2 => Season::Winter,
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            _ => Season::Autumn,
        }
    }

    pub fn is_first_of_month(self) -> bool {
        self.date().2 == 1
    }

    pub fn month_label(self) -> String {
        let (y, m, _) = self.date();
        format!("{} {}", MONTH_NAMES[m as usize - 1], y)
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.date();
        write!(f, "{:>2} {} {}", d, MONTH_NAMES[m as usize - 1], y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_moon_was_full_the_night_before_lawrence_burned() {
        // Full moon 19-20 May 1856; the Sack was the 21st.
        let d = (0..400)
            .map(Day)
            .find(|d| d.date() == (1856, 5, 20))
            .unwrap();
        assert!(d.moonlight() > 0.97, "{}", d.moonlight());
        assert_eq!(d.moon_name(), "full moon");
    }

    #[test]
    fn moon_cycles_in_a_synodic_month() {
        let a = Day(100).moon_phase();
        let b = Day(100 + 59).moon_phase();
        assert!((a - b).abs() < 0.03);
    }

    #[test]
    fn calendar_rolls_over_years_and_leap_day() {
        assert_eq!(Day(0).date(), (1855, 11, 1));
        assert_eq!(Day(61).date(), (1856, 1, 1));
        // 1856 is a leap year: 31 Jan + 29 Feb.
        assert_eq!(Day(61 + 31 + 28).date(), (1856, 2, 29));
        assert_eq!(Day(61 + 366).date(), (1857, 1, 1));
    }
}
