/// A musical interval defined by its semitone distance and a display name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Interval {
    pub semitones: u8,
    pub name: &'static str,
}

impl Interval {
    pub const UNISON: Self = Self { semitones: 0, name: "1" };
    pub const m2: Self = Self { semitones: 1, name: "b2" };
    pub const M2: Self = Self { semitones: 2, name: "2" };
    pub const m3: Self = Self { semitones: 3, name: "b3" };
    pub const M3: Self = Self { semitones: 4, name: "3" };
    pub const P4: Self = Self { semitones: 5, name: "4" };
    pub const tritone: Self = Self { semitones: 6, name: "#4" };
    pub const P5: Self = Self { semitones: 7, name: "5" };
    pub const m6: Self = Self { semitones: 8, name: "b6" };
    pub const M6: Self = Self { semitones: 9, name: "6" };
    pub const m7: Self = Self { semitones: 10, name: "b7" };
    pub const M7: Self = Self { semitones: 11, name: "maj7" };
    pub const M9: Self = Self { semitones: 14, name: "9" };
    pub const m9: Self = Self { semitones: 13, name: "b9" };
    pub const M11: Self = Self { semitones: 17, name: "11" };
    pub const m11: Self = Self { semitones: 16, name: "b11" };
    pub const M13: Self = Self { semitones: 21, name: "13" };
    pub const m13: Self = Self { semitones: 20, name: "b13" };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_semitones() {
        assert_eq!(Interval::M3.semitones, 4);
        assert_eq!(Interval::P5.semitones, 7);
        assert_eq!(Interval::m7.semitones, 10);
        assert_eq!(Interval::M7.semitones, 11);
        assert_eq!(Interval::M9.semitones, 14);
    }
}
