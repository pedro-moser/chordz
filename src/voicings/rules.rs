/// Constraints that determine which chord voicings are considered valid.
#[derive(Clone, Copy, Debug)]
pub struct VoicingRules {
    /// Maximum fret span (max fret - min fret) for a voicing.
    pub max_fret_span: u8,
    /// Maximum fret position allowed on any string.
    pub max_fret: u8,
    /// Minimum number of strings that must be played.
    pub min_strings: u8,
    /// Maximum number of strings that can be played.
    pub max_strings: u8,
    /// Whether the root note must be present in the voicing.
    pub require_root: bool,
}

impl VoicingRules {
    pub const fn default_rules() -> Self {
        Self {
            max_fret_span: 5,
            max_fret: 15,
            min_strings: 4,
            max_strings: 6,
            require_root: true,
        }
    }

    /// Check if a set of fret positions satisfies all rules.
    /// `positions` is indexed by string; `None` means muted.
    pub fn validate(&self, positions: &[Option<u8>; 6]) -> bool {
        let played: Vec<u8> = positions.iter().filter_map(|f| *f).collect();
        if played.len() < self.min_strings as usize || played.len() > self.max_strings as usize {
            return false;
        }
        let min_fret = *played.iter().min().unwrap();
        let max_fret = *played.iter().max().unwrap();
        if max_fret > self.max_fret {
            return false;
        }
        if max_fret - min_fret > self.max_fret_span {
            return false;
        }
        true
    }
}

impl Default for VoicingRules {
    fn default() -> Self {
        Self::default_rules()
    }
}
