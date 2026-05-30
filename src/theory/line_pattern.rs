#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Direction {
    pub fn invert(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ascending => "↑",
            Self::Descending => "↓",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriadId {
    T1,
    T2,
}

impl TriadId {
    pub fn label(self) -> &'static str {
        match self {
            Self::T1 => "T1",
            Self::T2 => "T2",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RhythmicFigure {
    Eighth,
    Sixteenth,
    Triplet,
}

impl RhythmicFigure {
    pub const ALL: [Self; 3] = [Self::Eighth, Self::Sixteenth, Self::Triplet];

    pub fn beat_duration(self) -> f32 {
        match self {
            Self::Eighth => 0.5,
            Self::Sixteenth => 0.25,
            Self::Triplet => 1.0 / 3.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Eighth => "♪ Eighth",
            Self::Sixteenth => "♬ Sixteenth",
            Self::Triplet => "³ Triplet",
        }
    }
}

/// How a block orders the three voices of its triad.
///
/// A triad pair splits the 6 non-root scale tones into two 3-note groups; within a group
/// the voices have roles 0, 1, 2 in scale-index order (for a stacked-thirds pair that's
/// root, 3rd, 5th). `Order` lets a block play those roles in an explicit cyclic sequence
/// (e.g. `[0, 2, 1]` = 1-5-3), the way triad-pair études actually rotate a shape.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Shape {
    /// Walk the position by step in `direction` — the original behaviour.
    #[default]
    Monotonic,
    /// Play the triad voices in this cyclic role order, each voice-led to the previous note.
    Order(Vec<u8>),
}

/// Which triad voice a block lands on for its first (connecting) note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Anchor {
    /// Nearest distinct note of the new triad — the original block-boundary behaviour.
    #[default]
    Nearest,
    /// Triad voice role 0 (the root, for a stacked-thirds pair).
    Root,
    /// Triad voice role 1 (the 3rd).
    Third,
    /// Triad voice role 2 (the 5th).
    Fifth,
}

impl Anchor {
    /// The triad voice role this anchor targets, or `None` for `Nearest`.
    pub fn role(self) -> Option<usize> {
        match self {
            Self::Nearest => None,
            Self::Root => Some(0),
            Self::Third => Some(1),
            Self::Fifth => Some(2),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternBlock {
    pub count: u8,
    pub direction: Direction,
    pub triad: TriadId,
    /// Voice ordering within the block. Defaults to `Monotonic` (legacy walk).
    pub shape: Shape,
    /// The block's landing/first note. Defaults to `Nearest` (legacy connect).
    pub anchor: Anchor,
}

impl PatternBlock {
    /// A legacy block (monotonic walk, nearest-note connect) — the pre-shape behaviour.
    pub fn legacy(count: u8, direction: Direction, triad: TriadId) -> Self {
        Self {
            count,
            direction,
            triad,
            shape: Shape::Monotonic,
            anchor: Anchor::Nearest,
        }
    }

    pub fn label(&self) -> String {
        format!("{}{} {}", self.count, self.direction.label(), self.triad.label())
    }
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub name: &'static str,
    pub blocks: Vec<PatternBlock>,
}

impl Pattern {
    pub fn total_notes(&self) -> u32 {
        self.blocks.iter().map(|b| b.count as u32).sum()
    }

    pub fn iter(&self) -> PatternIter<'_> {
        PatternIter {
            blocks: &self.blocks,
            index: 0,
        }
    }

    pub fn preset_alternating() -> Self {
        Self {
            name: "Alternating 3+3",
            blocks: vec![
                PatternBlock::legacy(3, Direction::Ascending, TriadId::T1),
                PatternBlock::legacy(3, Direction::Descending, TriadId::T2),
            ],
        }
    }

    pub fn preset_continuous_up() -> Self {
        Self {
            name: "Continuous up",
            blocks: vec![
                PatternBlock::legacy(3, Direction::Ascending, TriadId::T1),
                PatternBlock::legacy(3, Direction::Ascending, TriadId::T2),
            ],
        }
    }

    pub fn preset_short_long() -> Self {
        Self {
            name: "Short-long",
            blocks: vec![
                PatternBlock::legacy(2, Direction::Ascending, TriadId::T1),
                PatternBlock::legacy(4, Direction::Descending, TriadId::T2),
            ],
        }
    }

    pub fn all_presets() -> Vec<Self> {
        vec![
            Self::preset_alternating(),
            Self::preset_continuous_up(),
            Self::preset_short_long(),
        ]
    }
}

pub struct PatternIter<'a> {
    blocks: &'a [PatternBlock],
    index: usize,
}

impl<'a> Iterator for PatternIter<'a> {
    type Item = &'a PatternBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if self.blocks.is_empty() {
            return None;
        }
        let block = &self.blocks[self.index % self.blocks.len()];
        self.index += 1;
        Some(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_alternating_has_two_blocks() {
        let p = Pattern::preset_alternating();
        assert_eq!(p.blocks.len(), 2);
        assert_eq!(p.blocks[0].count, 3);
        assert_eq!(p.blocks[0].direction, Direction::Ascending);
        assert_eq!(p.blocks[0].triad, TriadId::T1);
        assert_eq!(p.blocks[1].count, 3);
        assert_eq!(p.blocks[1].direction, Direction::Descending);
        assert_eq!(p.blocks[1].triad, TriadId::T2);
    }

    #[test]
    fn preset_continuous_up() {
        let p = Pattern::preset_continuous_up();
        assert_eq!(p.blocks.len(), 2);
        assert_eq!(p.blocks[0].direction, Direction::Ascending);
        assert_eq!(p.blocks[1].direction, Direction::Ascending);
    }

    #[test]
    fn preset_short_long() {
        let p = Pattern::preset_short_long();
        assert_eq!(p.blocks[0].count, 2);
        assert_eq!(p.blocks[1].count, 4);
    }

    #[test]
    fn pattern_total_notes() {
        let p = Pattern::preset_alternating();
        assert_eq!(p.total_notes(), 6);
    }

    #[test]
    fn pattern_iterator_cycles() {
        let p = Pattern::preset_alternating();
        let blocks: Vec<_> = p.iter().take(5).collect();
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].triad, TriadId::T1);
        assert_eq!(blocks[1].triad, TriadId::T2);
        assert_eq!(blocks[2].triad, TriadId::T1);
    }

    #[test]
    fn rhythmic_figure_beat_duration() {
        assert!((RhythmicFigure::Eighth.beat_duration() - 0.5).abs() < f32::EPSILON);
        assert!((RhythmicFigure::Sixteenth.beat_duration() - 0.25).abs() < f32::EPSILON);
        let triplet_dur = RhythmicFigure::Triplet.beat_duration();
        assert!((triplet_dur - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn all_presets_listed() {
        let presets = Pattern::all_presets();
        assert_eq!(presets.len(), 3);
    }
}
