use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::render::diagram::render_fingering;
use crate::theory::chords::{self, ChordQuality};
use crate::theory::intervals::Interval;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::{map_voice_set, Fingering};
use crate::voicings::ranking::rank_fingerings;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;
use crate::voicings::voice_set::VoiceSet;

const NOTE_COUNTS: [usize; 5] = [2, 3, 4, 5, 6];
const VOICINGS_PER_VOICE_SET: usize = 3;
const MAX_VOICINGS: usize = 64;

/// Focusable panes in the browser screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserPane {
    Root,
    Family,
    Notes,
    Voicings,
}

/// Harmonic identity used before selecting a concrete chord color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChordFamily {
    Major,
    Dominant,
    Minor,
    HalfDiminished,
    Diminished,
}

impl ChordFamily {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Major,
            Self::Dominant,
            Self::Minor,
            Self::HalfDiminished,
            Self::Diminished,
        ]
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Dominant => "dominant",
            Self::Minor => "minor",
            Self::HalfDiminished => "half-diminished",
            Self::Diminished => "diminished",
        }
    }

    pub const fn quality_names(self) -> &'static [&'static str] {
        match self {
            Self::Major => &["maj7", "maj9", "maj13"],
            Self::Dominant => &["dom7", "dom9", "dom13", "dom7b9", "dom7#9", "dom7#5"],
            Self::Minor => &["m7", "m9", "m11", "m13"],
            Self::HalfDiminished => &["m7b5", "m9b11"],
            Self::Diminished => &["dim7"],
        }
    }
}

/// A generated fingering plus the recipe and ranking metadata used by the UI.
#[derive(Clone, Debug)]
pub struct VoicingData {
    pub quality: &'static ChordQuality,
    pub recipe: VoicingRecipe,
    pub tension: &'static str,
    pub fingering: Fingering,
}

/// The browser screen state.
pub struct BrowserScreen {
    pub root_index: usize,
    pub family_index: usize,
    pub note_count_index: usize,
    pub selected_voicing: usize,
    pub focused_pane: BrowserPane,
    pub voicings: Vec<VoicingData>,
}

impl BrowserScreen {
    pub fn new() -> Self {
        let mut screen = Self {
            root_index: 0,
            family_index: 0,
            note_count_index: 2,
            selected_voicing: 0,
            focused_pane: BrowserPane::Root,
            voicings: Vec::new(),
        };
        screen.refresh_voicings();
        screen
    }

    pub fn move_up(&mut self) {
        self.adjust_focused(-1);
    }

    pub fn move_down(&mut self) {
        self.adjust_focused(1);
    }

    pub fn focus_previous(&mut self) {
        self.focused_pane = match self.focused_pane {
            BrowserPane::Root => BrowserPane::Voicings,
            BrowserPane::Family => BrowserPane::Root,
            BrowserPane::Notes => BrowserPane::Family,
            BrowserPane::Voicings => BrowserPane::Notes,
        };
    }

    pub fn focus_next(&mut self) {
        self.focused_pane = match self.focused_pane {
            BrowserPane::Root => BrowserPane::Family,
            BrowserPane::Family => BrowserPane::Notes,
            BrowserPane::Notes => BrowserPane::Voicings,
            BrowserPane::Voicings => BrowserPane::Root,
        };
    }

    pub fn select_first(&mut self) {
        match self.focused_pane {
            BrowserPane::Root => self.set_root(0),
            BrowserPane::Family => self.set_family(0),
            BrowserPane::Notes => self.set_note_count(0),
            BrowserPane::Voicings => self.selected_voicing = 0,
        }
    }

    pub fn select_last(&mut self) {
        match self.focused_pane {
            BrowserPane::Root => self.set_root(chords::ROOTS.len() - 1),
            BrowserPane::Family => self.set_family(ChordFamily::all().len() - 1),
            BrowserPane::Notes => self.set_note_count(NOTE_COUNTS.len() - 1),
            BrowserPane::Voicings => {
                self.selected_voicing = self.voicings.len().saturating_sub(1);
            }
        }
    }

    pub fn current_chord_name(&self) -> String {
        self.chord_name_for(self.selected_quality())
    }

    pub fn current_note_count(&self) -> usize {
        NOTE_COUNTS[self.note_count_index]
    }

    /// Render the browser screen onto the given frame.
    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(5),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(frame.area());

        self.render_selectors(frame, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(44), Constraint::Percentage(56)].as_ref())
            .split(chunks[1]);

        self.render_voicings(frame, main_chunks[0]);
        self.render_diagram(frame, main_chunks[1]);
        self.render_status(frame, chunks[2]);
    }

    fn adjust_focused(&mut self, delta: isize) {
        match self.focused_pane {
            BrowserPane::Root => {
                let next = adjusted_index(self.root_index, chords::ROOTS.len(), delta);
                self.set_root(next);
            }
            BrowserPane::Family => {
                let next = adjusted_index(self.family_index, ChordFamily::all().len(), delta);
                self.set_family(next);
            }
            BrowserPane::Notes => {
                let next = adjusted_index(self.note_count_index, NOTE_COUNTS.len(), delta);
                self.set_note_count(next);
            }
            BrowserPane::Voicings => {
                self.selected_voicing =
                    adjusted_index(self.selected_voicing, self.voicings.len(), delta);
            }
        }
    }

    fn set_root(&mut self, index: usize) {
        self.root_index = index.min(chords::ROOTS.len() - 1);
        self.refresh_voicings();
    }

    fn set_family(&mut self, index: usize) {
        self.family_index = index.min(ChordFamily::all().len() - 1);
        self.refresh_voicings();
    }

    fn set_note_count(&mut self, index: usize) {
        self.note_count_index = index.min(NOTE_COUNTS.len() - 1);
        self.refresh_voicings();
    }

    fn root(&self) -> &'static str {
        chords::ROOTS[self.root_index]
    }

    fn family(&self) -> ChordFamily {
        ChordFamily::all()[self.family_index]
    }

    fn selected_quality(&self) -> &'static ChordQuality {
        self.voicings
            .get(self.selected_voicing)
            .map(|voicing| voicing.quality)
            .unwrap_or_else(|| find_quality(self.family().quality_names()[0]))
    }

    fn chord_name_for(&self, quality: &ChordQuality) -> String {
        chords::chord_name(self.root(), quality)
    }

    fn refresh_voicings(&mut self) {
        self.voicings = self.generate_voicings();
        self.selected_voicing = self
            .selected_voicing
            .min(self.voicings.len().saturating_sub(1));
    }

    fn generate_voicings(&self) -> Vec<VoicingData> {
        let fretboard = Fretboard::standard_tuning();
        let note_count = self.current_note_count();
        let rules = VoicingRules {
            min_strings: note_count as u8,
            max_strings: note_count as u8,
            max_fret_span: 5,
            max_fret: 15,
            require_root: false,
        };
        let root_pc = chords::root_to_pc(self.root()).unwrap();
        let recipes = [
            VoicingRecipe::Shell,
            VoicingRecipe::Closed,
            VoicingRecipe::Drop2,
            VoicingRecipe::Drop3,
            VoicingRecipe::RootlessA,
            VoicingRecipe::RootlessB,
            VoicingRecipe::Quartal,
            VoicingRecipe::UpperStructureTriad,
            VoicingRecipe::TriadPair,
        ];
        let mut voicings = Vec::new();

        for quality_name in self.family().quality_names() {
            let quality = find_quality(quality_name);

            for recipe in recipes {
                let voice_sets = voice_sets_for(recipe, root_pc, quality);
                for voice_set in voice_sets
                    .iter()
                    .filter(|voice_set| voice_set.len() == note_count)
                {
                    let mut fingerings = map_voice_set(voice_set, &fretboard, &rules);
                    rank_fingerings(&mut fingerings, voice_set, &fretboard);
                    voicings.extend(fingerings.into_iter().take(VOICINGS_PER_VOICE_SET).map(
                        |fingering| VoicingData {
                            quality,
                            recipe: voice_set.recipe,
                            tension: tension_label(quality, voice_set.recipe),
                            fingering,
                        },
                    ));
                }
            }
        }

        voicings.sort_by(|a, b| {
            tension_score(a.quality, a.recipe)
                .cmp(&tension_score(b.quality, b.recipe))
                .then_with(|| {
                    quality_order(self.family(), a.quality)
                        .cmp(&quality_order(self.family(), b.quality))
                })
                .then_with(|| recipe_order(a.recipe).cmp(&recipe_order(b.recipe)))
                .then_with(|| a.fingering.positions.cmp(&b.fingering.positions))
        });
        voicings.dedup_by(|a, b| a.fingering.positions == b.fingering.positions);
        voicings.truncate(MAX_VOICINGS);
        voicings
    }

    fn render_selectors(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(20),
                    Constraint::Percentage(55),
                    Constraint::Percentage(25),
                ]
                .as_ref(),
            )
            .split(area);

        render_selector(
            frame,
            chunks[0],
            " Root ",
            self.root(),
            self.focused_pane == BrowserPane::Root,
        );
        render_selector(
            frame,
            chunks[1],
            " Family ",
            self.family().name(),
            self.focused_pane == BrowserPane::Family,
        );
        render_selector(
            frame,
            chunks[2],
            " Notes ",
            &self.current_note_count().to_string(),
            self.focused_pane == BrowserPane::Notes,
        );
    }

    fn render_voicings(&self, frame: &mut Frame, area: Rect) {
        let visible_rows = area.height.saturating_sub(2) as usize;
        let (start, end) = visible_range(self.selected_voicing, self.voicings.len(), visible_rows);
        let items: Vec<ListItem> = self.voicings[start..end]
            .iter()
            .enumerate()
            .map(|(offset, voicing)| {
                let index = start + offset;
                let style = if index == self.selected_voicing {
                    selected_style(self.focused_pane == BrowserPane::Voicings)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(self.voicing_label(voicing), style)))
            })
            .collect();

        let title = format!(" Voicings {} ", self.voicings.len());
        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
        frame.render_widget(list, area);
    }

    fn render_diagram(&self, frame: &mut Frame, area: Rect) {
        let diagram_text = if let Some(voicing) = self.voicings.get(self.selected_voicing) {
            Text::from(render_fingering(
                &voicing.fingering,
                &Fretboard::standard_tuning(),
                &self.chord_name_for(voicing.quality),
            ))
        } else {
            Text::from("No voicings for this note count")
        };

        let title = if let Some(voicing) = self.voicings.get(self.selected_voicing) {
            format!(
                " {} {} {} ",
                self.chord_name_for(voicing.quality),
                recipe_label(voicing.recipe),
                voicing.tension
            )
        } else {
            format!(" {} ", self.current_chord_name())
        };
        let diagram =
            Paragraph::new(diagram_text).block(Block::default().title(title).borders(Borders::ALL));
        frame.render_widget(diagram, area);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let status = format!(
            " {} {} | {} | {}n | {}/{} | h/l fields  j/k move  g/G edge  q quit",
            self.root(),
            self.family().name(),
            self.current_chord_name(),
            self.current_note_count(),
            self.selected_voicing
                .saturating_add(1)
                .min(self.voicings.len()),
            self.voicings.len()
        );
        let status_bar = Paragraph::new(Line::from(Span::styled(
            status,
            Style::default().add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(status_bar, area);
    }

    fn voicing_label(&self, voicing: &VoicingData) -> String {
        let chord_name = self.chord_name_for(voicing.quality);
        let intervals: Vec<&str> = voicing
            .fingering
            .played_intervals()
            .iter()
            .map(|interval| compact_interval_name(*interval))
            .collect();
        format!(
            "{:<8} {:<3} {:<8} {}",
            chord_name,
            tension_code(voicing.tension),
            recipe_list_label(voicing.recipe),
            intervals.join(" ")
        )
    }
}

impl Default for BrowserScreen {
    fn default() -> Self {
        Self::new()
    }
}

fn adjusted_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as usize).min(len - 1)
    }
}

fn find_quality(name: &str) -> &'static ChordQuality {
    ChordQuality::ALL
        .iter()
        .find(|quality| quality.name == name)
        .unwrap()
}

fn voice_sets_for(
    recipe: VoicingRecipe,
    root_pc: u8,
    quality: &'static ChordQuality,
) -> Vec<VoiceSet> {
    match recipe {
        VoicingRecipe::Shell => recipe.generate_shell(root_pc, quality),
        VoicingRecipe::RootlessA => recipe.generate_rootless(root_pc, quality),
        VoicingRecipe::RootlessB => recipe.generate_rootless_b(root_pc, quality),
        VoicingRecipe::Drop2 => recipe.generate_drop2(root_pc, quality),
        VoicingRecipe::Drop3 => recipe.generate_drop3(root_pc, quality),
        VoicingRecipe::Quartal => recipe.generate_quartal(root_pc, quality),
        VoicingRecipe::UpperStructureTriad => {
            recipe.generate_upper_structure_triad(root_pc, quality)
        }
        VoicingRecipe::TriadPair => recipe.generate_triad_pair(root_pc, quality),
        VoicingRecipe::Closed => vec![VoiceSet::from_quality(root_pc, quality, recipe)],
    }
}

fn render_selector(frame: &mut Frame, area: Rect, title: &str, value: &str, focused: bool) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(if focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    let text = Paragraph::new(Line::from(Span::styled(
        value.to_string(),
        if focused {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        },
    )))
    .block(block);
    frame.render_widget(text, area);
}

fn recipe_label(recipe: VoicingRecipe) -> &'static str {
    match recipe {
        VoicingRecipe::Closed => "closed",
        VoicingRecipe::Shell => "shell",
        VoicingRecipe::RootlessA => "rootless-a",
        VoicingRecipe::RootlessB => "rootless-b",
        VoicingRecipe::Drop2 => "drop2",
        VoicingRecipe::Drop3 => "drop3",
        VoicingRecipe::Quartal => "quartal",
        VoicingRecipe::UpperStructureTriad => "upper",
        VoicingRecipe::TriadPair => "triadpair",
    }
}

fn recipe_list_label(recipe: VoicingRecipe) -> &'static str {
    match recipe {
        VoicingRecipe::RootlessA => "rless-a",
        VoicingRecipe::RootlessB => "rless-b",
        VoicingRecipe::UpperStructureTriad => "upper",
        VoicingRecipe::TriadPair => "triads",
        _ => recipe_label(recipe),
    }
}

fn tension_code(tension: &str) -> &'static str {
    match tension {
        "inside" => "in",
        "color" => "col",
        "out" => "out",
        _ => "?",
    }
}

fn compact_interval_name(interval: Interval) -> &'static str {
    if interval == Interval::UNISON {
        "R"
    } else if interval == Interval::M7 {
        "M7"
    } else if interval == Interval::dim7 {
        "dim7"
    } else {
        interval.name
    }
}

fn selected_style(focused: bool) -> Style {
    let mut style = Style::default().add_modifier(Modifier::REVERSED);
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

fn visible_range(selected: usize, len: usize, visible_rows: usize) -> (usize, usize) {
    if len == 0 || visible_rows == 0 {
        return (0, 0);
    }
    let half = visible_rows / 2;
    let mut start = selected.saturating_sub(half);
    if start + visible_rows > len {
        start = len.saturating_sub(visible_rows);
    }
    let end = (start + visible_rows).min(len);
    (start, end)
}

fn recipe_order(recipe: VoicingRecipe) -> usize {
    match recipe {
        VoicingRecipe::Shell => 0,
        VoicingRecipe::Closed => 1,
        VoicingRecipe::Drop2 => 2,
        VoicingRecipe::Drop3 => 3,
        VoicingRecipe::RootlessA => 4,
        VoicingRecipe::RootlessB => 5,
        VoicingRecipe::Quartal => 6,
        VoicingRecipe::UpperStructureTriad => 7,
        VoicingRecipe::TriadPair => 8,
    }
}

fn quality_order(family: ChordFamily, quality: &ChordQuality) -> usize {
    family
        .quality_names()
        .iter()
        .position(|name| *name == quality.name)
        .unwrap_or(usize::MAX)
}

fn quality_tension(name: &str) -> usize {
    match name {
        "maj7" | "m7" | "dom7" => 0,
        "maj9" | "m9" | "dom9" | "m7b5" | "dim7" => 1,
        "maj13" | "m11" | "m13" | "dom13" => 2,
        "dom7b9" | "dom7#9" | "dom7#5" | "m9b11" => 3,
        _ => 2,
    }
}

fn recipe_tension(recipe: VoicingRecipe) -> usize {
    match recipe {
        VoicingRecipe::Shell => 0,
        VoicingRecipe::Closed => 1,
        VoicingRecipe::Drop2 => 1,
        VoicingRecipe::Drop3 => 1,
        VoicingRecipe::RootlessA => 1,
        VoicingRecipe::RootlessB => 2,
        VoicingRecipe::Quartal => 3,
        VoicingRecipe::UpperStructureTriad => 4,
        VoicingRecipe::TriadPair => 4,
    }
}

fn tension_score(quality: &ChordQuality, recipe: VoicingRecipe) -> usize {
    quality_tension(quality.name) + recipe_tension(recipe)
}

fn tension_label(quality: &ChordQuality, recipe: VoicingRecipe) -> &'static str {
    match tension_score(quality, recipe) {
        0 | 1 => "inside",
        2 | 3 => "color",
        _ => "out",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::intervals::Interval;

    #[test]
    fn default_selection_is_c_major_four_note() {
        let screen = BrowserScreen::new();

        assert_eq!(screen.root(), "C");
        assert_eq!(screen.family(), ChordFamily::Major);
        assert_eq!(screen.current_note_count(), 4);
        assert_eq!(screen.current_chord_name(), "Cmaj7");
        assert!(
            screen
                .voicings
                .iter()
                .any(|voicing| voicing.recipe == VoicingRecipe::Drop2),
            "default Cmaj7 view should include drop2 inversions"
        );
    }

    #[test]
    fn browser_includes_rootless_cmaj9_four_note_voicings() {
        let screen = BrowserScreen::new();

        assert!(
            screen.voicings.iter().any(|voicing| {
                voicing.quality.name == "maj9"
                    && voicing.recipe == VoicingRecipe::RootlessA
                    && !voicing.fingering.has_interval(Interval::UNISON)
                    && voicing.fingering.played_count() == 4
            }),
            "browser should expose rootless Cmaj9 four-note voicings"
        );
    }

    #[test]
    fn browser_includes_quartal_and_triad_pair_colors() {
        let mut screen = BrowserScreen::new();
        screen.set_note_count(2);

        assert_eq!(screen.current_note_count(), 4);
        assert!(
            screen
                .voicings
                .iter()
                .any(|voicing| voicing.quality.name == "maj13"
                    && voicing.recipe == VoicingRecipe::Quartal),
            "browser should expose quartal Cmaj13 four-note voicings"
        );
        assert!(
            screen
                .voicings
                .iter()
                .any(|voicing| voicing.quality.name == "maj13"
                    && voicing.recipe == VoicingRecipe::TriadPair),
            "browser should expose triad-pair Cmaj13 four-note voicings"
        );
    }

    #[test]
    fn selected_voicing_controls_displayed_chord_color() {
        let mut screen = BrowserScreen::new();
        screen.selected_voicing = screen
            .voicings
            .iter()
            .position(|voicing| voicing.quality.name == "maj9")
            .unwrap();

        assert_eq!(screen.current_chord_name(), "Cmaj9");
    }

    #[test]
    fn note_count_filters_voicings_exactly() {
        let mut screen = BrowserScreen::new();
        screen.set_note_count(1);

        assert_eq!(screen.current_note_count(), 3);
        assert!(!screen.voicings.is_empty());
        assert!(screen
            .voicings
            .iter()
            .all(|voicing| voicing.fingering.played_count() == 3));
    }

    #[test]
    fn family_navigation_resets_to_valid_color() {
        let mut screen = BrowserScreen::new();
        screen.focused_pane = BrowserPane::Family;
        screen.select_last();

        assert_eq!(screen.family(), ChordFamily::Diminished);
        assert_eq!(screen.current_chord_name(), "Cdim7");
    }
}
