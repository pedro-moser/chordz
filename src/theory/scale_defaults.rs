use crate::theory::chords::ChordQuality;
use crate::theory::scales::Scale;

fn find_scale(name: &str) -> &'static Scale {
    Scale::ALL.iter().find(|s| s.name == name).unwrap()
}

pub fn default_scale(quality: &ChordQuality) -> &'static Scale {
    match quality.name {
        "maj7" | "maj9" | "maj13" => find_scale("Ionian"),
        "maj7#11" => find_scale("Lydian"),
        "m7" | "m9" | "m11" | "m13" => find_scale("Dorian"),
        "m7b5" | "m9b11" => find_scale("Locrian"),
        "dom7" | "dom9" | "dom13" => find_scale("Mixolydian"),
        "dom7#11" => find_scale("Lydian Dominant"),
        "dom7b9" | "dom7#9" | "dom7#5" => find_scale("Altered"),
        "dom7b13" => find_scale("Mixolydian \u{266D}6"),
        "dim7" => find_scale("Locrian"),
        _ => find_scale("Ionian"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chords::ChordQuality;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    #[test]
    fn major_family_defaults() {
        assert_eq!(default_scale(quality("maj7")).name, "Ionian");
        assert_eq!(default_scale(quality("maj9")).name, "Ionian");
        assert_eq!(default_scale(quality("maj13")).name, "Ionian");
        assert_eq!(default_scale(quality("maj7#11")).name, "Lydian");
    }

    #[test]
    fn minor_family_defaults() {
        assert_eq!(default_scale(quality("m7")).name, "Dorian");
        assert_eq!(default_scale(quality("m9")).name, "Dorian");
        assert_eq!(default_scale(quality("m11")).name, "Dorian");
        assert_eq!(default_scale(quality("m13")).name, "Dorian");
    }

    #[test]
    fn dominant_family_defaults() {
        assert_eq!(default_scale(quality("dom7")).name, "Mixolydian");
        assert_eq!(default_scale(quality("dom9")).name, "Mixolydian");
        assert_eq!(default_scale(quality("dom13")).name, "Mixolydian");
        assert_eq!(default_scale(quality("dom7#11")).name, "Lydian Dominant");
        assert_eq!(default_scale(quality("dom7b13")).name, "Mixolydian \u{266D}6");
    }

    #[test]
    fn altered_dominant_defaults() {
        assert_eq!(default_scale(quality("dom7b9")).name, "Altered");
        assert_eq!(default_scale(quality("dom7#9")).name, "Altered");
    }

    #[test]
    fn half_dim_defaults() {
        assert_eq!(default_scale(quality("m7b5")).name, "Locrian");
    }

    #[test]
    fn dim_defaults() {
        let scale = default_scale(quality("dim7"));
        assert_eq!(scale.name, "Locrian");
    }

    #[test]
    fn all_qualities_have_a_default() {
        for q in ChordQuality::ALL {
            let scale = default_scale(q);
            assert!(!scale.name.is_empty(), "no default for {}", q.name);
        }
    }
}
