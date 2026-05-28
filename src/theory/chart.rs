use crate::theory::chords::{self, ChordQuality};

/// A single chord change in a progression.
#[derive(Clone, Debug, PartialEq)]
pub struct ChordChange {
    /// Root name as written in the chart, e.g. "Bb", "F#".
    pub root: String,
    /// Pitch class of the root (0=C, 1=Db, ..., 11=B).
    pub root_pc: u8,
    /// The chord quality.
    pub quality: &'static ChordQuality,
    /// Duration in beats.
    pub beats: f32,
}

/// A parsed chord chart (sequence of changes).
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    pub title: String,
    pub changes: Vec<ChordChange>,
}

impl Chart {
    /// Parse a chord chart from text.
    ///
    /// Format: bar lines `|` separate measures; spaces separate chords
    /// within a measure. Each chord is `RootQuality` where Root is a note
    /// name (C, Db, F#, etc.) and Quality matches a known `ChordQuality`.
    ///
    /// Beats are distributed evenly within each measure (assuming 4/4):
    /// - One chord in a bar gets 4 beats.
    /// - Two chords in a bar get 2 beats each.
    /// - Three chords get 4/3 beats each.
    /// - Four chords get 1 beat each.
    ///
    /// A `%` repeats the previous bar. Empty bars are skipped.
    ///
    /// # Example
    ///
    /// ```
    /// use chordz::theory::chart::Chart;
    /// let chart = Chart::parse("Autumn Leaves", "| Bbmaj7 | Eb7 | Am7b5 | D7b9 |");
    /// assert!(chart.is_ok());
    /// let chart = chart.unwrap();
    /// assert_eq!(chart.changes.len(), 4);
    /// assert_eq!(chart.changes[0].root, "Bb");
    /// ```
    pub fn parse(title: &str, input: &str) -> Result<Self, ParseError> {
        let mut changes = Vec::new();
        let mut last_bar: Vec<ChordChange> = Vec::new();

        for segment in input.split('|') {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed == "%" {
                if last_bar.is_empty() {
                    return Err(ParseError::RepeatWithoutPrevious);
                }
                changes.extend(last_bar.iter().cloned());
                continue;
            }

            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let beats_per_chord = 4.0 / tokens.len() as f32;
            let mut bar = Vec::new();

            for token in tokens {
                let (root, quality) = parse_chord_symbol(token)?;
                let root_pc = chords::root_to_pc(&root)
                    .ok_or_else(|| ParseError::UnknownRoot(root.clone()))?;
                bar.push(ChordChange {
                    root,
                    root_pc,
                    quality,
                    beats: beats_per_chord,
                });
            }

            changes.extend(bar.iter().cloned());
            last_bar = bar;
        }

        if changes.is_empty() {
            return Err(ParseError::Empty);
        }

        Ok(Chart {
            title: title.to_string(),
            changes,
        })
    }

    pub fn total_beats(&self) -> f32 {
        self.changes.iter().map(|c| c.beats).sum()
    }
}

/// Errors that can occur when parsing a chord chart.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    UnknownRoot(String),
    UnknownQuality(String),
    InvalidChordSymbol(String),
    RepeatWithoutPrevious,
    Empty,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRoot(r) => write!(f, "unknown root: {}", r),
            Self::UnknownQuality(q) => write!(f, "unknown quality: {}", q),
            Self::InvalidChordSymbol(s) => write!(f, "invalid chord symbol: {}", s),
            Self::RepeatWithoutPrevious => write!(f, "'%' without a previous bar"),
            Self::Empty => write!(f, "empty chart"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Quality aliases mapping common jazz notation to internal quality names.
const QUALITY_ALIASES: &[(&str, &str)] = &[
    // Exact matches (try longest first)
    ("maj7#11", "maj7#11"),
    ("maj13", "maj13"),
    ("maj9", "maj9"),
    ("maj7", "maj7"),
    ("Maj7#11", "maj7#11"),
    ("Maj13", "maj13"),
    ("Maj9", "maj9"),
    ("Maj7", "maj7"),
    ("M7#11", "maj7#11"),
    ("M13", "maj13"),
    ("M9", "maj9"),
    ("M7", "maj7"),
    ("Δ7#11", "maj7#11"),
    ("Δ13", "maj13"),
    ("Δ9", "maj9"),
    ("Δ7", "maj7"),
    ("Δ", "maj7"),
    // Minor
    ("m13", "m13"),
    ("m11", "m11"),
    ("m9b11", "m9b11"),
    ("m9", "m9"),
    ("m7b5", "m7b5"),
    ("m7", "m7"),
    ("mi13", "m13"),
    ("mi11", "m11"),
    ("mi9", "m9"),
    ("mi7b5", "m7b5"),
    ("mi7", "m7"),
    ("min13", "m13"),
    ("min11", "m11"),
    ("min9", "m9"),
    ("min7b5", "m7b5"),
    ("min7", "m7"),
    ("-13", "m13"),
    ("-11", "m11"),
    ("-9", "m9"),
    ("-7b5", "m7b5"),
    ("-7", "m7"),
    // Half-diminished
    ("ø9", "m9b11"),
    ("ø7", "m7b5"),
    ("ø", "m7b5"),
    // Dominant
    ("dom7b13", "dom7b13"),
    ("dom7#11", "dom7#11"),
    ("dom13", "dom13"),
    ("dom9", "dom9"),
    ("dom7#9", "dom7#9"),
    ("dom7b9", "dom7b9"),
    ("dom7#5", "dom7#5"),
    ("dom7", "dom7"),
    ("13", "dom13"),
    ("9", "dom9"),
    ("7b13", "dom7b13"),
    ("7#11", "dom7#11"),
    ("7#9", "dom7#9"),
    ("7b9", "dom7b9"),
    ("7#5", "dom7#5"),
    ("7alt", "dom7#9"),
    ("7", "dom7"),
    // Diminished
    ("dim7", "dim7"),
    ("o7", "dim7"),
    ("°7", "dim7"),
    ("°", "dim7"),
    // Augmented
    ("aug7", "dom7#5"),
    ("+7", "dom7#5"),
];

/// Normalize unicode accidentals to ASCII equivalents.
fn normalize_symbol(symbol: &str) -> String {
    symbol.replace('♭', "b").replace('♯', "#")
}

/// Parse a chord symbol into (root_name, quality).
fn parse_chord_symbol(symbol: &str) -> Result<(String, &'static ChordQuality), ParseError> {
    if symbol.is_empty() {
        return Err(ParseError::InvalidChordSymbol(symbol.to_string()));
    }

    let symbol = normalize_symbol(symbol);

    // Extract root: 1-2 characters (letter + optional accidental).
    let root_end = find_root_end(&symbol);
    if root_end == 0 {
        return Err(ParseError::InvalidChordSymbol(symbol.to_string()));
    }

    let root = &symbol[..root_end];
    if chords::root_to_pc(root).is_none() {
        return Err(ParseError::UnknownRoot(root.to_string()));
    }

    let quality_str = &symbol[root_end..];

    // Empty quality means dominant 7 is too ambiguous — require explicit quality.
    // But a bare root like "C" could mean... let's treat it as maj7 for jazz context.
    if quality_str.is_empty() {
        let quality = find_quality("maj7")?;
        return Ok((root.to_string(), quality));
    }

    // Try matching quality aliases (longest match first — aliases are pre-sorted).
    for &(alias, canonical) in QUALITY_ALIASES {
        if quality_str == alias {
            let quality = find_quality(canonical)?;
            return Ok((root.to_string(), quality));
        }
    }

    Err(ParseError::UnknownQuality(quality_str.to_string()))
}

fn find_root_end(symbol: &str) -> usize {
    let bytes = symbol.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_uppercase() {
        return 0;
    }
    // After normalization, accidentals are always '#' or 'b'.
    if bytes.len() > 1 && (bytes[1] == b'#' || bytes[1] == b'b') {
        2
    } else {
        1
    }
}

fn find_quality(name: &str) -> Result<&'static ChordQuality, ParseError> {
    ChordQuality::ALL
        .iter()
        .find(|q| q.name == name)
        .ok_or_else(|| ParseError::UnknownQuality(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_progression() {
        let chart = Chart::parse("Test", "| Cmaj7 | Dm7 | G7 | Cmaj7 |").unwrap();
        assert_eq!(chart.changes.len(), 4);
        assert_eq!(chart.changes[0].root, "C");
        assert_eq!(chart.changes[0].quality.name, "maj7");
        assert_eq!(chart.changes[0].beats, 4.0);
        assert_eq!(chart.changes[1].root, "D");
        assert_eq!(chart.changes[1].quality.name, "m7");
        assert_eq!(chart.changes[2].root, "G");
        assert_eq!(chart.changes[2].quality.name, "dom7");
    }

    #[test]
    fn parse_flat_roots() {
        let chart = Chart::parse("Test", "| Bbmaj7 | Ebm7 | Ab7 | Dbmaj7 |").unwrap();
        assert_eq!(chart.changes[0].root, "Bb");
        assert_eq!(chart.changes[0].root_pc, 10);
        assert_eq!(chart.changes[1].root, "Eb");
        assert_eq!(chart.changes[1].root_pc, 3);
        assert_eq!(chart.changes[2].root, "Ab");
        assert_eq!(chart.changes[2].root_pc, 8);
        assert_eq!(chart.changes[3].root, "Db");
        assert_eq!(chart.changes[3].root_pc, 1);
    }

    #[test]
    fn parse_sharp_roots() {
        let chart = Chart::parse("Test", "| F#m7 | C#m7 |").unwrap();
        assert_eq!(chart.changes[0].root, "F#");
        assert_eq!(chart.changes[0].root_pc, 6);
    }

    #[test]
    fn parse_two_chords_per_bar() {
        let chart = Chart::parse("Test", "| Dm7 G7 | Cmaj7 |").unwrap();
        assert_eq!(chart.changes.len(), 3);
        assert_eq!(chart.changes[0].beats, 2.0);
        assert_eq!(chart.changes[1].beats, 2.0);
        assert_eq!(chart.changes[2].beats, 4.0);
    }

    #[test]
    fn parse_three_chords_per_bar_sums_to_four_beats() {
        let chart = Chart::parse("Test", "| Dm7 G7 Cmaj7 |").unwrap();
        assert_eq!(chart.changes.len(), 3);
        assert!((chart.total_beats() - 4.0).abs() < f32::EPSILON);
        for change in &chart.changes {
            assert!((change.beats - 4.0 / 3.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn parse_repeat_bar() {
        let chart = Chart::parse("Test", "| Dm7 | % | G7 |").unwrap();
        assert_eq!(chart.changes.len(), 3);
        assert_eq!(chart.changes[0].quality.name, "m7");
        assert_eq!(chart.changes[1].quality.name, "m7");
        assert_eq!(chart.changes[2].quality.name, "dom7");
    }

    #[test]
    fn parse_quality_aliases() {
        let chart = Chart::parse("Test", "| CΔ7 | D-7 | G13 | Aø7 | Bo7 |").unwrap();
        assert_eq!(chart.changes[0].quality.name, "maj7");
        assert_eq!(chart.changes[1].quality.name, "m7");
        assert_eq!(chart.changes[2].quality.name, "dom13");
        assert_eq!(chart.changes[3].quality.name, "m7b5");
        assert_eq!(chart.changes[4].quality.name, "dim7");
    }

    #[test]
    fn parse_dominant_alterations() {
        let chart = Chart::parse("Test", "| G7b9 | G7#9 | G7#5 | G7#11 | G7b13 |").unwrap();
        assert_eq!(chart.changes[0].quality.name, "dom7b9");
        assert_eq!(chart.changes[1].quality.name, "dom7#9");
        assert_eq!(chart.changes[2].quality.name, "dom7#5");
        assert_eq!(chart.changes[3].quality.name, "dom7#11");
        assert_eq!(chart.changes[4].quality.name, "dom7b13");
    }

    #[test]
    fn parse_lydian_major() {
        let chart = Chart::parse("Test", "| Cmaj7#11 | FΔ7#11 |").unwrap();
        assert_eq!(chart.changes[0].quality.name, "maj7#11");
        assert_eq!(chart.changes[1].quality.name, "maj7#11");
    }

    #[test]
    fn parse_extended_chords() {
        let chart = Chart::parse("Test", "| Cmaj13 | Dm11 | G9 |").unwrap();
        assert_eq!(chart.changes[0].quality.name, "maj13");
        assert_eq!(chart.changes[1].quality.name, "m11");
        assert_eq!(chart.changes[2].quality.name, "dom9");
    }

    #[test]
    fn parse_stella_by_starlight_first_8_bars() {
        let input = "\
            | Em7b5 | A7b9  | Cm7   | F7    |\
            | Fm7   | Bb7   | Ebmaj7| Ab7   |";
        let chart = Chart::parse("Stella by Starlight", input).unwrap();
        assert_eq!(chart.changes.len(), 8);
        assert_eq!(chart.changes[0].root, "E");
        assert_eq!(chart.changes[0].quality.name, "m7b5");
        assert_eq!(chart.changes[1].root, "A");
        assert_eq!(chart.changes[1].quality.name, "dom7b9");
        assert_eq!(chart.changes[4].root, "F");
        assert_eq!(chart.changes[4].quality.name, "m7");
        assert_eq!(chart.changes[6].root, "Eb");
        assert_eq!(chart.changes[6].quality.name, "maj7");
        assert_eq!(chart.total_beats(), 32.0);
    }

    #[test]
    fn parse_error_unknown_quality() {
        let result = Chart::parse("Test", "| Csus4 |");
        assert!(matches!(result, Err(ParseError::UnknownQuality(_))));
    }

    #[test]
    fn parse_error_unknown_root() {
        let result = Chart::parse("Test", "| Hmaj7 |");
        assert!(matches!(result, Err(ParseError::UnknownRoot(_))));
    }

    #[test]
    fn parse_error_empty() {
        let result = Chart::parse("Test", "||");
        assert!(matches!(result, Err(ParseError::Empty)));
    }

    #[test]
    fn parse_error_repeat_without_previous() {
        let result = Chart::parse("Test", "| % |");
        assert!(matches!(result, Err(ParseError::RepeatWithoutPrevious)));
    }

    #[test]
    fn parse_bare_root_defaults_to_maj7() {
        let chart = Chart::parse("Test", "| C | D |").unwrap();
        assert_eq!(chart.changes[0].quality.name, "maj7");
    }

    #[test]
    fn parse_unicode_accidentals() {
        let chart = Chart::parse("Test", "| B♭7 | E♭maj7 |").unwrap();
        assert_eq!(chart.changes[0].root, "Bb");
        assert_eq!(chart.changes[0].root_pc, 10);
        assert_eq!(chart.changes[1].root, "Eb");
    }

    #[test]
    fn total_beats() {
        let chart = Chart::parse("Test", "| Cmaj7 | Dm7 G7 | Cmaj7 |").unwrap();
        assert_eq!(chart.total_beats(), 12.0);
    }
}
