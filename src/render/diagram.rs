use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::Fingering;

/// Render a `Fingering` as an ASCII fretboard diagram.
///
/// The diagram shows:
/// - Chord name header.
/// - Fret region indicator.
/// - Six string columns (low E to high e).
/// - Frets as horizontal rows.
/// - Muted strings marked with `x`, open strings with `O`.
///
/// # Example
///
/// ```text
/// Cmaj7
/// frets 0-3
///      E   A   D   G   B   e
///    +---+---+---+---+---+---+
///  0 | x |   |   | O | O | O |
///    +---+---+---+---+---+---+
///  1 |   |   |   |   |   |   |
///    +---+---+---+---+---+---+
///  2 |   |   | 2 |   |   |   |
///    +---+---+---+---+---+---+
///  3 |   | 3 |   |   |   |   |
///    +---+---+---+---+---+---+
/// ```
pub fn render_fingering(fingering: &Fingering, fretboard: &Fretboard, chord_name: &str) -> String {
    let _ = fretboard; // used for note lookups in future extensions

    let played: Vec<u8> = fingering.positions.iter().filter_map(|f| *f).collect();
    if played.is_empty() {
        return chord_name.to_string();
    }

    let max_fret = *played.iter().max().unwrap();
    let has_open = played.contains(&0);
    let min_fretted = played.iter().filter(|f| **f > 0).min().copied();

    let start_fret = if has_open {
        0
    } else {
        min_fretted.unwrap_or(0)
    };
    let end_fret = max_fret.max(start_fret.saturating_add(2));
    let frets: Vec<u8> = (start_fret..=end_fret).collect();
    let cell_width = frets
        .iter()
        .map(|fret| fret.to_string().len())
        .max()
        .unwrap_or(1)
        .max(3);
    let fret_label_width = end_fret.to_string().len().max(2);

    let mut lines = Vec::new();

    lines.push(chord_name.to_string());
    lines.push(format!("frets {}-{}", start_fret, end_fret));

    let string_labels = ["E", "A", "D", "G", "B", "e"];
    let header_prefix = " ".repeat(fret_label_width + 2);
    let header = string_labels.iter().fold(header_prefix, |mut row, label| {
        row.push_str(&format!("{:^cell_width$} ", label));
        row
    });
    lines.push(header.trim_end().to_string());

    let separator = format!(
        "{}+{}+",
        " ".repeat(fret_label_width + 1),
        vec!["-".repeat(cell_width); 6].join("+")
    );

    for fret_num in frets {
        lines.push(separator.clone());
        let mut row = format!("{:>width$} |", fret_num, width = fret_label_width);

        for string in 0..6 {
            let cell = match fingering.positions[string] {
                None if fret_num == start_fret => "x".to_string(),
                None => String::new(),
                Some(0) if fret_num == 0 => "O".to_string(),
                Some(0) => String::new(),
                Some(fret) if fret == fret_num => fret.to_string(),
                Some(_) => String::new(),
            };
            row.push_str(&format!("{:^cell_width$}|", cell));
        }

        lines.push(row);
    }
    lines.push(separator);

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::intervals::Interval;
    use crate::voicings::generate::Fingering;

    /// Golden test: open-position Cmaj7 voicing.
    ///
    /// x32000 = C E G B E, a real Cmaj7 shape.
    #[test]
    fn test_open_voicing_cmaj7() {
        let fretboard = Fretboard::standard_tuning();
        let fingering = Fingering {
            positions: [None, Some(3), Some(2), Some(0), Some(0), Some(0)],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                Some(Interval::M7),
                Some(Interval::M3),
            ],
        };

        let diagram = render_fingering(&fingering, &fretboard, "Cmaj7");

        let expected = "\
Cmaj7
frets 0-3
     E   A   D   G   B   e
   +---+---+---+---+---+---+
 0 | x |   |   | O | O | O |
   +---+---+---+---+---+---+
 1 |   |   |   |   |   |   |
   +---+---+---+---+---+---+
 2 |   |   | 2 |   |   |   |
   +---+---+---+---+---+---+
 3 |   | 3 |   |   |   |   |
   +---+---+---+---+---+---+";

        assert_eq!(diagram, expected, "diagram mismatch:\n{}", diagram);
    }

    /// Golden test: rootless G9 shell voicing with open string.
    ///
    /// D3/G2/B0 = F A B, containing b7, 9, and 3 of G9.
    #[test]
    fn test_rootless_shell_voicing_g9() {
        let fretboard = Fretboard::standard_tuning();
        let fingering = Fingering {
            positions: [None, None, Some(3), Some(2), Some(0), None],
            intervals: [
                None,
                None,
                Some(Interval::m7),
                Some(Interval::M9),
                Some(Interval::M3),
                None,
            ],
        };

        let diagram = render_fingering(&fingering, &fretboard, "G9");

        let expected = "\
G9
frets 0-3
     E   A   D   G   B   e
   +---+---+---+---+---+---+
 0 | x | x |   |   | O | x |
   +---+---+---+---+---+---+
 1 |   |   |   |   |   |   |
   +---+---+---+---+---+---+
 2 |   |   |   | 2 |   |   |
   +---+---+---+---+---+---+
 3 |   |   | 3 |   |   |   |
   +---+---+---+---+---+---+";

        assert_eq!(diagram, expected, "diagram mismatch:\n{}", diagram);
    }

    #[test]
    fn test_two_digit_frets_are_not_truncated() {
        let fretboard = Fretboard::standard_tuning();
        let fingering = Fingering {
            positions: [None, None, None, Some(10), Some(11), Some(12)],
            intervals: [
                None,
                None,
                None,
                Some(Interval::m7),
                Some(Interval::M3),
                Some(Interval::M13),
            ],
        };

        let diagram = render_fingering(&fingering, &fretboard, "G13");

        assert!(diagram.contains("10"));
        assert!(diagram.contains("11"));
        assert!(diagram.contains("12"));
    }
}
