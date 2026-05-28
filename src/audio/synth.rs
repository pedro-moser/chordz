use crate::theory::notes::Note;

const SAMPLE_RATE: u32 = 44100;

/// Generate a plucked-string-like sample for a single note.
///
/// Uses a sine wave with harmonics and an exponential decay envelope.
/// This is a placeholder — swap with real guitar .wav samples later.
pub fn generate_pluck(note: Note, duration_secs: f32) -> Vec<f32> {
    // Guard against non-finite / non-positive / absurd durations: a float->int cast
    // saturates to usize::MAX, which would make Vec::with_capacity panic and abort the
    // WASM module. Clamp so no caller (including #[wasm_bindgen] entry points) can crash us.
    let duration_secs = sanitize_duration(duration_secs);
    if duration_secs == 0.0 {
        return Vec::new();
    }
    let freq = midi_to_freq(note.midi());
    let num_samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Exponential decay envelope.
        let envelope = (-3.0 * t / duration_secs).exp();

        // Fundamental + harmonics for a richer tone.
        let fundamental = (2.0 * std::f32::consts::PI * freq * t).sin();
        let h2 = 0.5 * (2.0 * std::f32::consts::PI * freq * 2.0 * t).sin();
        let h3 = 0.25 * (2.0 * std::f32::consts::PI * freq * 3.0 * t).sin();
        let h4 = 0.12 * (2.0 * std::f32::consts::PI * freq * 4.0 * t).sin();

        let sample = envelope * (fundamental + h2 + h3 + h4) * 0.3;
        samples.push(sample);
    }

    samples
}

/// Clamp a requested duration to a finite, positive, bounded range (seconds).
/// Returns 0.0 for non-finite or non-positive input so callers emit silence.
fn sanitize_duration(duration_secs: f32) -> f32 {
    if duration_secs.is_finite() && duration_secs > 0.0 {
        duration_secs.min(30.0)
    } else {
        0.0
    }
}

/// Generate a chord sample by mixing multiple notes.
pub fn generate_chord(notes: &[Note], duration_secs: f32) -> StereoSamples {
    let duration_secs = sanitize_duration(duration_secs);
    let num_samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
    let mut left = vec![0.0f32; num_samples];
    let mut right = vec![0.0f32; num_samples];

    if notes.is_empty() {
        return StereoSamples {
            left,
            right,
            sample_rate: SAMPLE_RATE,
        };
    }

    let gain = 1.0 / notes.len() as f32;

    for (idx, &note) in notes.iter().enumerate() {
        let mono = generate_pluck(note, duration_secs);
        // Slight stereo spread: low strings left, high strings right.
        let pan = idx as f32 / (notes.len() - 1).max(1) as f32;
        let left_gain = gain * (1.0 - pan * 0.3);
        let right_gain = gain * (0.7 + pan * 0.3);

        for (i, &s) in mono.iter().enumerate() {
            if i < num_samples {
                left[i] += s * left_gain;
                right[i] += s * right_gain;
            }
        }
    }

    StereoSamples {
        left,
        right,
        sample_rate: SAMPLE_RATE,
    }
}

pub struct StereoSamples {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub sample_rate: u32,
}

fn midi_to_freq(midi: i32) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::notes::Note;

    #[test]
    fn pluck_has_correct_length() {
        let note = Note::new(0, 4); // C4
        let samples = generate_pluck(note, 1.0);
        assert_eq!(samples.len(), 44100);
    }

    #[test]
    fn pluck_starts_loud_and_decays() {
        let note = Note::new(9, 4); // A4 = 440Hz
        let samples = generate_pluck(note, 1.0);
        let start_energy: f32 = samples[..1000].iter().map(|s| s * s).sum::<f32>() / 1000.0;
        let end_energy: f32 = samples[40000..].iter().map(|s| s * s).sum::<f32>() / 4100.0;
        assert!(
            start_energy > end_energy * 5.0,
            "start energy ({}) should be much higher than end ({})",
            start_energy,
            end_energy
        );
    }

    #[test]
    fn chord_mixes_multiple_notes() {
        let notes = vec![Note::new(0, 3), Note::new(4, 3), Note::new(7, 3)]; // C E G
        let chord = generate_chord(&notes, 0.5);
        assert_eq!(chord.left.len(), chord.right.len());
        assert_eq!(chord.left.len(), 22050);
        // Should have non-zero energy.
        let energy: f32 = chord.left.iter().map(|s| s * s).sum();
        assert!(energy > 0.0);
    }

    #[test]
    fn empty_chord_is_silence() {
        let chord = generate_chord(&[], 0.5);
        assert!(chord.left.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn midi_to_freq_a4_is_440() {
        let freq = midi_to_freq(69);
        assert!((freq - 440.0).abs() < 0.01);
    }
}
