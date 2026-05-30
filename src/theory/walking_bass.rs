//! Quarter-note walking bass generator (pure core).
//!
//! Given a sequence of chord segments it produces one bass note per beat following the
//! classic walking formula: root (or the written slash bass) on every downbeat, chord
//! tones (3rd/5th/7th) through the middle, and a chromatic approach note on the last beat
//! of each bar that leads by a half-step into the next chord's bass. Octaves are chosen so
//! the line moves stepwise (smooth voice-leading) and stays inside an upright-bass register.
//!
//! Ported from the original `web/src/lib/walkingBass.ts` so the musical logic lives in the
//! Rust core (cargo-tested) instead of the Svelte shell. The port fixes three bugs the TS
//! version had: it honours slash-chord bass notes, derives chord tones from the
//! authoritative `ChordQuality` intervals (so `7#5`/`m7b5`/`dim7` are correct instead of a
//! lossy regex), and guarantees every segment sounds at least one note (sub-beat chords
//! used to be silently dropped).

use crate::theory::chords::ChordQuality;

/// A single chord in the walking sequence.
#[derive(Clone, Copy, Debug)]
pub struct BassSegment {
    /// Chord root pitch class, 0=C … 11=B.
    pub root_pc: u8,
    /// Slash-chord bass pitch class (e.g. Fm7/Bb → Some(10)); `None` walks off the root.
    pub bass_pc: Option<u8>,
    /// Chord quality — its `intervals[1..=3]` are the authoritative 3rd/5th/7th.
    pub quality: &'static ChordQuality,
    /// Segment length in beats (may be fractional when several chords share a bar).
    pub beats: f32,
}

/// One walking bass note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BassNote {
    pub midi: i32,
    /// Beat offset from the start of the line (integer for the quarter-note pulse;
    /// fractional only for the recovery note injected for a squeezed sub-beat chord).
    pub beat: f32,
    /// Note length in beats.
    pub beats: f32,
    /// Index of the segment (chord) this note belongs to.
    pub chord: usize,
}

// Upright-bass walking range: roughly G1 to D3. Wide enough to walk, low enough to sit
// under the guitar without colliding with the melody register.
const LOW: i32 = 31;
const HIGH: i32 = 50;
const CENTER: i32 = 40; // E2-ish, where the first root is anchored

/// 3rd / 5th / 7th semitone offsets from the root. For every `ChordQuality` in the table
/// the intervals are ordered `[unison, 3rd, 5th, 7th, …extensions]`, so indices 1..=3 are
/// exactly the chord tones — no `% 12` classification that an extended chord could poison.
fn chord_tones(quality: &ChordQuality) -> (i32, i32, i32) {
    let nth = |i: usize, fallback: u8| {
        quality
            .intervals
            .get(i)
            .map(|iv| iv.semitones)
            .unwrap_or(fallback) as i32
    };
    (nth(1, 4), nth(2, 7), nth(3, 10))
}

/// Place pitch class `pc` in the octave nearest `reference`, clamped to the bass register.
fn place_near(pc: i32, reference: i32) -> i32 {
    let pc = ((pc % 12) + 12) % 12;
    let mut m = pc + 12 * (((reference - pc) as f32) / 12.0).round() as i32;
    while m < LOW {
        m += 12;
    }
    while m > HIGH {
        m -= 12;
    }
    m
}

struct Norm {
    root_pc: i32,
    bass_pc: i32,
    tones: (i32, i32, i32),
    start: f32,
    end: f32,
}

pub fn walking_bass_line(segments: &[BassSegment]) -> Vec<BassNote> {
    if segments.is_empty() {
        return Vec::new();
    }

    // Lay the chords on the real (possibly fractional) beat timeline. The walking pulse is
    // one quarter note per WHOLE beat of this timeline, so chord changes stay locked to the
    // grid — no drift even when several chords share a bar (e.g. 4/3 beats each).
    let mut norm: Vec<Norm> = segments
        .iter()
        .map(|s| {
            let root_pc = ((s.root_pc as i32 % 12) + 12) % 12;
            let bass_pc = match s.bass_pc {
                Some(b) => ((b as i32 % 12) + 12) % 12,
                None => root_pc,
            };
            Norm {
                root_pc,
                bass_pc,
                tones: chord_tones(s.quality),
                start: 0.0,
                end: 0.0,
            }
        })
        .collect();
    let mut acc = 0.0_f32;
    for (i, s) in segments.iter().enumerate() {
        norm[i].start = acc;
        acc += s.beats.max(0.0);
        norm[i].end = acc;
    }
    let total_beats = acc.round().max(1.0) as i32;

    // Chord sounding at integer beat `g`: the segment whose [start, end) span contains it
    // (last segment also owns the final boundary). Walks forward, so it's O(total + chords).
    let mut cursor = 0usize;
    let chord_at = |g: f32, cursor: &mut usize| -> usize {
        while *cursor < norm.len() - 1 && g >= norm[*cursor].end - 1e-6 {
            *cursor += 1;
        }
        *cursor
    };

    let mut out: Vec<BassNote> = Vec::with_capacity(total_beats as usize);
    let mut prev = CENTER;
    for g in 0..total_beats {
        let gf = g as f32;
        let idx = chord_at(gf, &mut cursor);
        let (third, fifth, seventh) = norm[idx].tones;

        let is_downbeat = g == (norm[idx].start - 1e-6).ceil() as i32; // first grid beat of this chord
        let next_idx = if g + 1 < total_beats {
            chord_at((g + 1) as f32, &mut cursor)
        } else {
            idx
        };
        let is_last_beat_of_chord = next_idx != idx;

        let target = if is_downbeat {
            // Root (or the written slash bass) on the chord's first beat.
            place_near(norm[idx].bass_pc, prev)
        } else if is_last_beat_of_chord {
            // Chromatic approach: a half-step either side of the next chord's bass,
            // whichever steps smaller so the line keeps walking by step.
            let next_bass = norm[next_idx].bass_pc;
            let below = place_near(next_bass + 11, prev);
            let above = place_near(next_bass + 1, prev);
            if (below - prev).abs() <= (above - prev).abs() {
                below
            } else {
                above
            }
        } else if g == total_beats - 1 {
            // Final beat of the chart: settle on a stable chord tone nearest the previous
            // note but DIFFERENT from it (avoids repeating the 5th; honours #5/b5/bb7 via
            // the quality-derived tones).
            settle_tone(norm[idx].root_pc, third, fifth, seventh, prev)
        } else {
            // Middle of a chord: cycle through chord tones (3rd → 5th → 7th), nearest prev.
            let within = g - (norm[idx].start - 1e-6).ceil() as i32; // 1, 2, … inside this chord
            let pool = [third, fifth, seventh];
            let off = pool[((within - 1).rem_euclid(pool.len() as i32)) as usize];
            place_near(norm[idx].root_pc + off, prev)
        };
        out.push(BassNote {
            midi: target,
            beat: gf,
            beats: 1.0,
            chord: idx,
        });
        prev = target;
    }

    // Coverage guarantee: any segment squeezed out by the integer-quarter grid (more than
    // four chords in a bar) gets one recovery note at its real fractional start, so the bass
    // never goes silent on a chord. Placed off the integer grid, it can't collide with the
    // walking notes; both downstream callers map beat→time by multiplying by beat-seconds,
    // so a fractional beat schedules at the right moment.
    let mut covered = vec![false; norm.len()];
    for n in &out {
        covered[n.chord] = true;
    }
    for (i, seg) in norm.iter().enumerate() {
        if covered[i] {
            continue;
        }
        let reference = out.last().map(|n| n.midi).unwrap_or(CENTER);
        out.push(BassNote {
            midi: place_near(seg.bass_pc, reference),
            beat: seg.start,
            beats: segments[i].beats.max(0.05),
            chord: i,
        });
    }
    out.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap_or(std::cmp::Ordering::Equal));

    out
}

/// Pick a chord tone (root/3rd/5th/7th) nearest `prev` whose pitch class differs from
/// `prev` — a stable, non-repeating note to end a phrase on.
fn settle_tone(root_pc: i32, third: i32, fifth: i32, seventh: i32, prev: i32) -> i32 {
    let prev_pc = ((prev % 12) + 12) % 12;
    let mut best: Option<i32> = None;
    for off in [0, third, fifth, seventh] {
        let placed = place_near(root_pc + off, prev);
        if ((placed % 12) + 12) % 12 == prev_pc {
            continue;
        }
        match best {
            Some(b) if (b - prev).abs() <= (placed - prev).abs() => {}
            _ => best = Some(placed),
        }
    }
    // If every chord tone equals prev's pitch class (degenerate), just keep the root.
    best.unwrap_or_else(|| place_near(root_pc, prev))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    /// Build a segment from a root pitch class + internal quality name.
    fn seg(root_pc: u8, quality_name: &str, beats: f32) -> BassSegment {
        BassSegment {
            root_pc,
            bass_pc: None,
            quality: quality(quality_name),
            beats,
        }
    }

    fn slash(root_pc: u8, quality_name: &str, bass_pc: u8, beats: f32) -> BassSegment {
        BassSegment {
            root_pc,
            bass_pc: Some(bass_pc),
            quality: quality(quality_name),
            beats,
        }
    }

    fn pc(midi: i32) -> i32 {
        ((midi % 12) + 12) % 12
    }

    #[test]
    fn emits_one_quarter_note_per_beat() {
        let line = walking_bass_line(&[seg(2, "m7", 4.0), seg(7, "dom7", 4.0)]);
        assert_eq!(line.len(), 8);
        assert_eq!(
            line.iter().map(|n| n.beat).collect::<Vec<_>>(),
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
        );
        assert!(line.iter().all(|n| n.beats == 1.0));
    }

    #[test]
    fn lands_the_chord_root_on_every_downbeat() {
        let line = walking_bass_line(&[seg(2, "m7", 4.0), seg(7, "dom7", 4.0), seg(0, "maj7", 4.0)]);
        assert_eq!(pc(line[0].midi), 2); // Dm7 -> D
        assert_eq!(pc(line[4].midi), 7); // G7  -> G
        assert_eq!(pc(line[8].midi), 0); // Cmaj7 -> C
    }

    #[test]
    fn approaches_the_next_root_chromatically_on_the_last_beat() {
        let line = walking_bass_line(&[seg(2, "m7", 4.0), seg(7, "dom7", 4.0)]);
        // beat 3 is the last beat of the Dm7 bar; next root is G (7).
        let approach = pc(line[3].midi);
        assert!(approach == 6 || approach == 8, "approach {} not a half-step from G", approach);
    }

    #[test]
    fn uses_minor_third_for_minor_and_major_third_for_dominant() {
        let minor = walking_bass_line(&[seg(2, "m7", 4.0), seg(2, "m7", 4.0)]);
        assert!(minor[..4].iter().any(|n| pc(n.midi) == (2 + 3) % 12)); // F natural (m3 of D)
        let dom = walking_bass_line(&[seg(7, "dom7", 4.0), seg(7, "dom7", 4.0)]);
        assert!(dom[..4].iter().any(|n| pc(n.midi) == (7 + 4) % 12)); // B natural (M3 of G)
    }

    #[test]
    fn keeps_motion_stepwise_no_leap_wider_than_an_octave() {
        let line = walking_bass_line(&[seg(2, "m7", 4.0), seg(7, "dom7", 4.0), seg(0, "maj7", 4.0)]);
        for i in 1..line.len() {
            assert!((line[i].midi - line[i - 1].midi).abs() <= 12);
        }
    }

    #[test]
    fn stays_within_a_sensible_bass_register() {
        let line = walking_bass_line(&[seg(2, "m7", 4.0), seg(7, "dom7", 4.0)]);
        for n in &line {
            assert!(n.midi >= LOW && n.midi <= HIGH, "midi {} out of register", n.midi);
        }
    }

    #[test]
    fn quantizes_short_bars_to_at_least_one_beat() {
        let line = walking_bass_line(&[seg(2, "m7", 2.0), seg(7, "dom7", 2.0)]);
        assert_eq!(line.len(), 4);
        assert_eq!(pc(line[0].midi), 2);
        assert_eq!(pc(line[2].midi), 7);
    }

    #[test]
    fn locks_the_pulse_to_the_global_beat_grid_with_three_chords_per_bar() {
        let t = 4.0 / 3.0;
        let line = walking_bass_line(&[
            seg(2, "m7", t),
            seg(7, "dom7", t),
            seg(0, "maj7", t),
            seg(5, "maj7", 4.0),
        ]);
        // Total real beats = 4 + 4 = 8 -> 8 quarter notes on the grid, beats 0..7.
        assert_eq!(
            line.iter().map(|n| n.beat).collect::<Vec<_>>(),
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
        );
        assert_eq!(pc(line[4].midi), 5); // downbeat of bar 2 (Fmaj7) stays on grid beat 4
        assert_eq!(pc(line[0].midi), 2); // first bar still opens on D
    }

    #[test]
    fn tags_each_note_with_the_index_of_its_chord() {
        let line = walking_bass_line(&[seg(2, "m7", 4.0), seg(7, "dom7", 4.0)]);
        assert_eq!(line[..4].iter().map(|n| n.chord).collect::<Vec<_>>(), vec![0, 0, 0, 0]);
        assert_eq!(line[4..8].iter().map(|n| n.chord).collect::<Vec<_>>(), vec![1, 1, 1, 1]);
    }

    #[test]
    fn ends_on_a_chord_tone_for_the_final_bar() {
        let line = walking_bass_line(&[seg(0, "maj7", 4.0)]);
        let last = pc(line.last().unwrap().midi);
        assert!([0, 4, 7, 11].contains(&last)); // C maj7 chord tones
    }

    // --- Bug-fix regressions (new behaviour the TS version lacked) ---

    #[test]
    fn honours_the_slash_chord_bass_on_the_downbeat() {
        // Fm7/Bb: the written bass is Bb (10), not the chord root F (5).
        let line = walking_bass_line(&[slash(5, "m7", 10, 4.0), seg(0, "maj7", 4.0)]);
        assert_eq!(pc(line[0].midi), 10, "downbeat should land on the slash bass Bb");
    }

    #[test]
    fn approaches_the_slash_bass_of_the_next_chord() {
        // Walking into Fm7/Bb should chromatically approach Bb (10), not F (5).
        let line = walking_bass_line(&[seg(7, "dom7", 4.0), slash(5, "m7", 10, 4.0)]);
        let approach = pc(line[3].midi);
        assert!(approach == 9 || approach == 11, "approach {} not a half-step from Bb", approach);
    }

    #[test]
    fn final_note_is_a_real_chord_tone_for_an_augmented_dominant() {
        // G7#5 chord tones: G(7) B(11) D#(3, the #5) F(5). A plain perfect-5th settle
        // (D=2) would be wrong; the #5 comes from the quality intervals.
        let line = walking_bass_line(&[seg(7, "dom7#5", 4.0)]);
        let last = pc(line.last().unwrap().midi);
        assert!([7, 11, 3, 5].contains(&last), "final pc {} not a 7#5 chord tone", last);
    }

    #[test]
    fn final_note_does_not_repeat_the_previous_note() {
        // On a lone 4-beat chord the old code played the 5th on beat 2 and again on the
        // final beat 3. The settle must differ from the previous note.
        let line = walking_bass_line(&[seg(0, "maj7", 4.0)]);
        let n = line.len();
        assert_ne!(pc(line[n - 1].midi), pc(line[n - 2].midi));
    }

    #[test]
    fn every_segment_sounds_at_least_one_note_even_when_squeezed() {
        // Six chords in one 4/4 bar = 0.667 beats each; the integer grid only has 4 beats,
        // so some chords used to be dropped entirely. Each must now appear at least once.
        let b = 4.0 / 6.0;
        let segs = [
            seg(0, "maj7", b),
            seg(2, "m7", b),
            seg(4, "m7", b),
            seg(5, "maj7", b),
            seg(7, "dom7", b),
            seg(9, "m7", b),
        ];
        let line = walking_bass_line(&segs);
        for i in 0..segs.len() {
            assert!(line.iter().any(|n| n.chord == i), "segment {} got no bass note", i);
        }
    }

    #[test]
    fn empty_input_yields_no_notes() {
        assert!(walking_bass_line(&[]).is_empty());
    }
}
