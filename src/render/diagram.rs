use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::Fingering;

/// Render a `Fingering` as an ASCII fretboard diagram.
///
/// The diagram shows:
/// - Chord name header.
/// - Fret region indicator (starting fret).
/// - Six string rows (high E at top, low E at bottom).
/// - Muted strings marked with `x`, open strings with `O`.
/// - Fret positions shown in their corresponding column.
///
/// # Example
///
/// ```text
/// Cmaj7
/// frets 5-8
///      5  6  7  8
/// e| -- -- --  8
/// B| -- --  7 --
/// G|  5 -- -- --
/// D| xx xx xx xx
/// A| xx xx xx xx
/// E| xx xx xx xx
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
        .max(2);

    fn format_cell(value: &str, width: usize) -> String {
        format!("{value:>width$}")
    }

    let mut lines = Vec::new();

    // Header: chord name
    lines.push(chord_name.to_string());

    // Fret region indicator
    lines.push(format!("frets {}-{}", start_fret, end_fret));

    // Fret number row
    let fret_header = frets.iter().fold(String::from("   "), |mut row, fret| {
        row.push(' ');
        row.push_str(&format_cell(&fret.to_string(), cell_width));
        row
    });
    lines.push(fret_header);

    // String rows: index 0 = low E, index 5 = high E (render top-to-bottom = high-to-low)
    let string_labels = ['E', 'A', 'D', 'G', 'B', 'e'];

    for i in (0..6).rev() {
        let label = string_labels[i];
        let mut row = format!("{}|", label);

        for fret_num in &frets {
            row.push(' ');
            match fingering.positions[i] {
                None => {
                    row.push_str(&format_cell(&"x".repeat(cell_width), cell_width));
                }
                Some(0) => {
                    if *fret_num == 0 {
                        row.push_str(&format_cell("O", cell_width));
                    } else {
                        row.push_str(&format_cell(&"-".repeat(cell_width), cell_width));
                    }
                }
                Some(fret) => {
                    if fret == *fret_num {
                        row.push_str(&format_cell(&fret.to_string(), cell_width));
                    } else {
                        row.push_str(&format_cell(&"-".repeat(cell_width), cell_width));
                    }
                }
            }
        }

        lines.push(row);
    }

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
     0  1  2  3
e|  O -- -- --
B|  O -- -- --
G|  O -- -- --
D| -- --  2 --
A| -- -- --  3
E| xx xx xx xx";

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
     0  1  2  3
e| xx xx xx xx
B|  O -- -- --
G| -- --  2 --
D| -- -- --  3
A| xx xx xx xx
E| xx xx xx xx";

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
