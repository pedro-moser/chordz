use crate::theory::chords::ChordQuality;

pub type StabilityTable = [u8; 12];

// Major (maj7): R=4 b9=0 9=3 b3=1 3=4 11=1 #11=2 5=4 b13=1 13=3 b7=0 7=4
const MAJOR: StabilityTable = [4, 0, 3, 1, 4, 1, 2, 4, 1, 3, 0, 4];

// Minor (m7): R=4 b9=1 9=3 b3=4 3=0 11=4 #11=2 5=4 b13=2 13=2 b7=4 maj7=2
const MINOR: StabilityTable = [4, 1, 3, 4, 0, 4, 2, 4, 2, 2, 4, 2];

// Dominant natural (→major): R=4 b9=2 9=3 #9=2 3=4 4/sus=3 #11=2 5=4 b13=2 13=3 b7=4 7=0
const DOM_NATURAL: StabilityTable = [4, 2, 3, 2, 4, 3, 2, 4, 2, 3, 4, 0];

// Dominant altered (→minor/tritone sub): R=4 b9=3 9=2 #9=3 3=4 4/sus=1 #11=3 5=4 b13=3 13=1 b7=4 7=0
const DOM_ALTERED: StabilityTable = [4, 3, 2, 3, 4, 1, 3, 4, 3, 1, 4, 0];

// Half-diminished (m7b5): R=4 b9=1 9=2 b3=4 3=0 11=3 b5=4 5=0 b13=2 13=1 b7=4 7=1
const HALF_DIM: StabilityTable = [4, 1, 2, 4, 0, 3, 4, 0, 2, 1, 4, 1];

// Diminished (dim7): R=4 b9=2 9=2 b3=4 3=1 11=2 b5=4 5=1 b13=2 dim7=4 13=2 b7=1
const DIM: StabilityTable = [4, 2, 2, 4, 1, 2, 4, 1, 2, 4, 2, 1];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HarmonicFamily {
    Major,
    Minor,
    Dominant,
    HalfDiminished,
    Diminished,
}

pub fn detect_family(quality: &ChordQuality) -> HarmonicFamily {
    let name = quality.name;
    if name.starts_with("maj") {
        HarmonicFamily::Major
    } else if name.starts_with("m7b5") || name.starts_with("m9b11") {
        HarmonicFamily::HalfDiminished
    } else if name.starts_with("dim") {
        HarmonicFamily::Diminished
    } else if name.starts_with('m') {
        HarmonicFamily::Minor
    } else {
        HarmonicFamily::Dominant
    }
}

pub fn resolves_to_minor(next_quality: Option<&ChordQuality>) -> bool {
    match next_quality {
        Some(q) => {
            let family = detect_family(q);
            matches!(family, HarmonicFamily::Minor | HarmonicFamily::HalfDiminished)
        }
        None => false,
    }
}

pub fn get_stability_table(
    quality: &ChordQuality,
    next_quality: Option<&ChordQuality>,
) -> StabilityTable {
    let mut table = match detect_family(quality) {
        HarmonicFamily::Major => MAJOR,
        HarmonicFamily::Minor => MINOR,
        HarmonicFamily::Dominant => {
            if resolves_to_minor(next_quality) {
                DOM_ALTERED
            } else {
                DOM_NATURAL
            }
        }
        HarmonicFamily::HalfDiminished => HALF_DIM,
        HarmonicFamily::Diminished => DIM,
    };

    let explicit: Vec<u8> = quality
        .intervals
        .iter()
        .map(|iv| iv.semitones % 12)
        .collect();

    for &s in &explicit {
        table[s as usize] = 4;
    }

    for &s in &explicit {
        let below = if s == 0 { 11 } else { s - 1 };
        if !explicit.contains(&below) && table[below as usize] > 1 {
            table[below as usize] = 1;
        }
    }

    table
}

/// Apply abstraction modifier to a stability table.
///
/// `abstraction` ranges from 0.0 (grounded) to 1.0 (abstract).
/// At 0.0, the table is unchanged.
/// At 1.0, core chord tones (R, 3/b3, 5/b5, 7/b7) are reduced,
/// and extensions (9, 11, 13, etc.) are boosted — making the engine
/// prefer voicings that imply the harmony without spelling it out.
/// Avoid notes (★0-1) are NOT changed — abstraction doesn't add dissonance.
pub fn apply_abstraction(table: &mut StabilityTable, quality: &ChordQuality, abstraction: f32) {
    if abstraction < 0.05 {
        return;
    }

    let explicit: Vec<u8> = quality
        .intervals
        .iter()
        .map(|iv| iv.semitones % 12)
        .collect();

    for (i, entry) in table.iter_mut().enumerate() {
        let s = *entry;
        if s <= 1 {
            continue;
        }
        let is_core = explicit.contains(&(i as u8));
        if is_core {
            // Grounded(0): no change. Abstract(1): 4→2, 3→2
            let new_val = s as f32 * (1.0 - abstraction * 0.6);
            *entry = (new_val.round() as u8).max(1);
        } else if s >= 2 {
            // Boost extensions: Balanced(0.3): +1, Abstract(1.0): +2
            let boost = (abstraction * 2.0).round() as u8;
            *entry = (s + boost).min(4);
        }
    }
}

/// Adjust min_total_stability threshold for abstraction level.
/// Higher abstraction lowers the effective threshold since all scores drop.
pub fn adjusted_threshold(base_threshold: u8, abstraction: f32) -> u8 {
    let scale = 1.0 - abstraction * 0.4;
    (base_threshold as f32 * scale).round() as u8
}

pub fn subset_stability(table: &StabilityTable, semitones: &[u8]) -> u16 {
    semitones
        .iter()
        .map(|&s| table[s as usize % 12] as u16)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_core_tones_max_stability() {
        assert_eq!(subset_stability(&MAJOR, &[0, 4, 7, 11]), 16);
    }

    #[test]
    fn major_with_9_lower() {
        assert_eq!(subset_stability(&MAJOR, &[0, 2, 4, 11]), 15);
    }

    #[test]
    fn major_avoid_note_low() {
        assert_eq!(MAJOR[5], 1);
    }

    #[test]
    fn major_b9_forbidden() {
        assert_eq!(MAJOR[1], 0);
    }

    #[test]
    fn minor_11_very_stable() {
        assert_eq!(MINOR[5], 4);
    }

    #[test]
    fn dom_natural_vs_altered() {
        assert_eq!(DOM_NATURAL[1], 2);
        assert_eq!(DOM_ALTERED[1], 3);
    }

    #[test]
    fn detect_family_covers_all_qualities() {
        for q in ChordQuality::ALL {
            let _ = detect_family(q);
        }
    }

    #[test]
    fn half_dim_b5_stable() {
        assert_eq!(HALF_DIM[6], 4);
    }

    #[test]
    fn half_dim_natural_5_forbidden() {
        assert_eq!(HALF_DIM[7], 0);
    }

    #[test]
    fn dom_resolving_to_minor_uses_altered() {
        let dom = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let minor = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let table = get_stability_table(dom, Some(minor));
        assert_eq!(table[1], 3);
    }

    #[test]
    fn dom_resolving_to_major_uses_natural() {
        let dom = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let maj = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let table = get_stability_table(dom, Some(maj));
        assert_eq!(table[1], 2);
    }
}
