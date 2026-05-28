use crate::theory::chords::ChordQuality;

pub type StabilityTable = [u8; 12];

// Major (maj7): R=40 b9=0 9=30 b3=10 3=40 11=0 #11=20 5=40 b13=10 13=30 b7=0 7=40
const MAJOR: StabilityTable = [40, 0, 30, 10, 40, 0, 20, 40, 10, 30, 0, 40];

// Minor (m7): R=40 b9=5 9=30 b3=40 3=0 11=40 #11=20 5=40 b13=20 13=20 b7=40 maj7=20
const MINOR: StabilityTable = [40, 5, 30, 40, 0, 40, 20, 40, 20, 20, 40, 20];

// Dominant natural (→major): R=40 b9=20 9=30 #9=20 3=40 4/sus=20 #11=20 5=40 b13=20 13=30 b7=40 7=0
const DOM_NATURAL: StabilityTable = [40, 20, 30, 20, 40, 20, 20, 40, 20, 30, 40, 0];

// Dominant altered (→minor/tritone sub): R=40 b9=30 9=20 #9=30 3=40 4/sus=10 #11=30 5=40 b13=20 13=10 b7=40 7=0
const DOM_ALTERED: StabilityTable = [40, 30, 20, 30, 40, 10, 30, 40, 20, 10, 40, 0];

// Half-diminished (m7b5): R=40 b9=10 9=20 b3=40 3=0 11=30 b5=40 5=0 b13=20 13=10 b7=40 7=2
const HALF_DIM: StabilityTable = [40, 10, 20, 40, 0, 30, 40, 0, 20, 10, 40, 2];

// Diminished (dim7): R=40 b9=20 9=20 b3=40 3=10 11=20 b5=40 5=10 b13=20 dim7=40 13=20 b7=10
const DIM: StabilityTable = [40, 20, 20, 40, 10, 20, 40, 10, 20, 40, 20, 10];

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

    for iv in quality.intervals {
        table[iv.semitones as usize % 12] = 40;
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

    let third_semitones: &[u8] = match () {
        _ if explicit.contains(&4) => &[4],    // M3
        _ if explicit.contains(&3) => &[3],    // m3
        _ => &[],
    };

    for (i, entry) in table.iter_mut().enumerate() {
        let s = *entry;
        if s <= 1 {
            continue;
        }
        let sem = i as u8;
        let is_core = explicit.contains(&sem);
        if !is_core {
            if s >= 20 {
                let boost = (abstraction * 20.0).round() as u8;
                *entry = (s + boost).min(40);
            }
            continue;
        }
        let is_root = sem == 0;
        let is_third = third_semitones.contains(&sem);
        if is_root {
            let new_val = s as f32 * (1.0 - abstraction);
            *entry = new_val.round() as u8;
        } else if is_third {
            let new_val = s as f32 * (1.0 - abstraction);
            *entry = new_val.round() as u8;
        } else {
            let new_val = s as f32 * (1.0 - abstraction * 0.5);
            *entry = (new_val.round() as u8).max(20);
        }
    }
}

/// Adjust min_total_stability threshold for abstraction level.
/// Higher abstraction lowers the effective threshold since all scores drop.
pub fn adjusted_threshold(base_threshold: u8, abstraction: f32) -> u8 {
    let scale = 1.0 - abstraction * 0.4;
    (base_threshold as f32 * scale).round() as u8
}

/// Returns the scale degree class (1-7) for a semitone, given the chord context.
///
/// Semitone 6 is ambiguous: #4 (4th class) vs b5 (5th class).
/// Semitone 9 is ambiguous: 6/13 (6th class) vs dim7 (7th class).
/// The quality's explicit intervals resolve the ambiguity.
pub fn degree_class(semitone: u8, quality: &ChordQuality) -> u8 {
    let s = semitone % 12;
    match s {
        0 => 1,
        1 | 2 => 2,
        3 => {
            // #9 (2nd class) if the chord has M3; b3 (3rd class) otherwise
            let has_major_third = quality.intervals.iter().any(|iv| iv.semitones % 12 == 4);
            if has_major_third { 2 } else { 3 }
        }
        4 => 3,
        5 => 4,
        6 => {
            // b5 (5th class) if quality uses it as a 5th; #4 (4th class) otherwise
            let has_b5_as_chord_tone = quality.intervals.iter().any(|iv| {
                iv.semitones % 12 == 6
            });
            let has_natural_5 = quality.intervals.iter().any(|iv| {
                iv.semitones % 12 == 7
            });
            if has_b5_as_chord_tone && !has_natural_5 {
                5 // b5 = 5th class
            } else {
                4 // #4 = 4th class
            }
        }
        7 => 5,
        8 => 6,
        9 => {
            // dim7 (7th class) if quality has dim7; otherwise 6/13 (6th class)
            let has_dim7 = quality.intervals.iter().any(|iv| {
                iv.semitones % 12 == 9 && iv.name == "dim7"
            });
            if has_dim7 {
                7
            } else {
                6
            }
        }
        10 | 11 => 7,
        _ => 0,
    }
}

/// Check if a subset has two semitones from the same degree class.
pub fn has_duplicate_degree(semitones: &[u8], quality: &ChordQuality) -> bool {
    let classes: Vec<u8> = semitones.iter().map(|&s| degree_class(s, quality)).collect();
    for i in 0..classes.len() {
        for j in (i + 1)..classes.len() {
            if classes[i] == classes[j] {
                return true;
            }
        }
    }
    false
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
        assert_eq!(subset_stability(&MAJOR, &[0, 4, 7, 11]), 160);
    }

    #[test]
    fn major_with_9_lower() {
        assert_eq!(subset_stability(&MAJOR, &[0, 2, 4, 11]), 150);
    }

    #[test]
    fn major_avoid_note_excluded() {
        assert_eq!(MAJOR[5], 0);
    }

    #[test]
    fn major_b9_forbidden() {
        assert_eq!(MAJOR[1], 0);
    }

    #[test]
    fn minor_11_very_stable() {
        assert_eq!(MINOR[5], 40);
    }

    #[test]
    fn dom_natural_vs_altered() {
        assert_eq!(DOM_NATURAL[1], 20);
        assert_eq!(DOM_ALTERED[1], 30);
    }

    #[test]
    fn detect_family_covers_all_qualities() {
        for q in ChordQuality::ALL {
            let _ = detect_family(q);
        }
    }

    #[test]
    fn half_dim_b5_stable() {
        assert_eq!(HALF_DIM[6], 40);
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
        assert_eq!(table[1], 30);
    }

    #[test]
    fn dom_resolving_to_major_uses_natural() {
        let dom = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let maj = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let table = get_stability_table(dom, Some(maj));
        assert_eq!(table[1], 20);
    }
}
