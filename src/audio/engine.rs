use std::io::Cursor;

use kira::manager::backend::DefaultBackend;
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::PlaybackState;
use kira::tween::Tween;

use crate::theory::notes::Note;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::Fingering;

use super::synth;

/// Audio playback engine backed by kira.
pub struct AudioEngine {
    manager: AudioManager,
    active_sounds: Vec<StaticSoundHandle>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        let settings = AudioManagerSettings::default();
        let manager = AudioManager::<DefaultBackend>::new(settings)
            .map_err(|e| format!("failed to initialize audio: {}", e))?;
        Ok(Self {
            manager,
            active_sounds: Vec::new(),
        })
    }

    /// Stop all currently playing sounds.
    pub fn stop_all(&mut self) {
        for handle in &mut self.active_sounds {
            handle.stop(Tween::default());
        }
        self.active_sounds.clear();
    }

    fn cleanup_finished(&mut self) {
        self.active_sounds
            .retain(|h| h.state() != PlaybackState::Stopped);
    }

    fn play_wav(&mut self, wav: Vec<u8>) -> Result<(), String> {
        self.cleanup_finished();
        let sound_data = StaticSoundData::from_cursor(Cursor::new(wav))
            .map_err(|e| format!("decode failed: {}", e))?;
        let handle = self
            .manager
            .play(sound_data)
            .map_err(|e| format!("play failed: {}", e))?;
        self.active_sounds.push(handle);
        Ok(())
    }

    /// Play a chord voicing as a strum (all notes at once).
    pub fn play_voicing(
        &mut self,
        fingering: &Fingering,
        fretboard: &Fretboard,
        duration_secs: f32,
    ) -> Result<(), String> {
        let notes: Vec<Note> = fingering.notes(fretboard).into_iter().flatten().collect();
        if notes.is_empty() {
            return Ok(());
        }

        let stereo = synth::generate_chord(&notes, duration_secs);
        let wav = encode_wav(&stereo.left, &stereo.right, stereo.sample_rate);
        self.play_wav(wav)
    }

    /// Play a full chord progression (each chord strummed in sequence).
    pub fn play_progression(
        &mut self,
        chords: &[(Fingering, f32)],
        fretboard: &Fretboard,
        bpm: f32,
    ) -> Result<(), String> {
        self.stop_all();

        let beat_duration = 60.0 / bpm;
        let sample_rate = 44100u32;
        let total_beats: f32 = chords.iter().map(|(_, b)| *b).sum();
        let total_duration = total_beats * beat_duration + 1.0;
        let total_samples = (sample_rate as f32 * total_duration) as usize;
        let mut left = vec![0.0f32; total_samples];
        let mut right = vec![0.0f32; total_samples];

        let mut beat_offset: f32 = 0.0;
        for (fingering, beats) in chords {
            let notes: Vec<Note> = fingering.notes(fretboard).into_iter().flatten().collect();
            if notes.is_empty() {
                beat_offset += *beats;
                continue;
            }
            let chord_duration = *beats * beat_duration;
            let stereo = synth::generate_chord(&notes, chord_duration);
            let sample_offset = (beat_offset * beat_duration * sample_rate as f32) as usize;

            for (j, (&l, &r)) in stereo.left.iter().zip(stereo.right.iter()).enumerate() {
                let idx = sample_offset + j;
                if idx < total_samples {
                    left[idx] += l;
                    right[idx] += r;
                }
            }
            beat_offset += *beats;
        }

        let wav = encode_wav(&left, &right, sample_rate);
        self.play_wav(wav)
    }

    /// Play notes one at a time (arpeggio).
    pub fn play_arpeggio(
        &mut self,
        fingering: &Fingering,
        fretboard: &Fretboard,
        note_duration: f32,
    ) -> Result<(), String> {
        let notes: Vec<Note> = fingering.notes(fretboard).into_iter().flatten().collect();
        if notes.is_empty() {
            return Ok(());
        }

        let total_duration = note_duration * notes.len() as f32 + 0.5;
        let sample_rate = 44100u32;
        let total_samples = (sample_rate as f32 * total_duration) as usize;
        let mut left = vec![0.0f32; total_samples];
        let mut right = vec![0.0f32; total_samples];

        let gain = 1.0 / notes.len() as f32;
        for (i, &note) in notes.iter().enumerate() {
            let offset = (sample_rate as f32 * note_duration * i as f32) as usize;
            let pluck = synth::generate_pluck(note, note_duration + 0.3);
            let pan = i as f32 / (notes.len() - 1).max(1) as f32;
            let lg = gain * (1.0 - pan * 0.3);
            let rg = gain * (0.7 + pan * 0.3);

            for (j, &s) in pluck.iter().enumerate() {
                let idx = offset + j;
                if idx < total_samples {
                    left[idx] += s * lg;
                    right[idx] += s * rg;
                }
            }
        }

        let wav = encode_wav(&left, &right, sample_rate);
        self.play_wav(wav)
    }
}

/// Encode stereo f32 samples as a WAV file in memory.
fn encode_wav(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = left.len();
    let num_channels = 2u16;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = (num_samples * num_channels as usize * bits_per_sample as usize / 8) as u32;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size as usize + 8);

    // RIFF header.
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk.
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk.
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    for i in 0..num_samples {
        let l = (left[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        let r = (right[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&l.to_le_bytes());
        buf.extend_from_slice(&r.to_le_bytes());
    }

    buf
}
