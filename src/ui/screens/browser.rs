use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::render::diagram::render_fingering;
use crate::theory::chords::{self, ChordQuality};
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::{map_voice_set, Fingering};
use crate::voicings::ranking::rank_fingerings;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;

/// Focusable panes in the browser screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserPane {
    Chords,
    Voicings,
}

/// A single chord with its name and generated fingerings.
#[derive(Clone, Debug)]
pub struct ChordData {
    pub name: String,
    pub voicings: Vec<VoicingData>,
}

/// A generated fingering plus the recipe that produced it.
#[derive(Clone, Debug)]
pub struct VoicingData {
    pub recipe: VoicingRecipe,
    pub fingering: Fingering,
}

/// The browser screen state: chord list, voicing list, and diagram.
pub struct BrowserScreen {
    /// All available chords with their fingerings.
    pub chords: Vec<ChordData>,
    /// Index into `chords` for the currently selected chord.
    pub selected_chord: usize,
    /// Index into the selected chord's fingerings.
    pub selected_voicing: usize,
    /// Which list receives navigation keys.
    pub focused_pane: BrowserPane,
}

impl BrowserScreen {
    /// Build the browser screen by generating shell voicings for a set of
    /// chord qualities across all 12 roots.
    pub fn new() -> Self {
        let fretboard = Fretboard::standard_tuning();
        let rules = VoicingRules {
            min_strings: 2,
            max_strings: 6,
            max_fret_span: 5,
            max_fret: 15,
            require_root: false,
        };

        let quality_names = &[
            "maj7", "maj9", "maj13", "m7", "m9", "m11", "dom7", "dom9", "dom13",
        ];
        let recipes = [VoicingRecipe::RootlessA, VoicingRecipe::Shell];

        let mut chords = Vec::new();

        for &root in &chords::ROOTS {
            let root_pc = chords::root_to_pc(root).unwrap();

            for &qname in quality_names {
                let quality = ChordQuality::ALL.iter().find(|q| q.name == qname).unwrap();

                let name = chords::chord_name(root, quality);

                let mut voicings = Vec::new();
                for recipe in recipes {
                    let voice_sets = match recipe {
                        VoicingRecipe::Shell => recipe.generate_shell(root_pc, quality),
                        VoicingRecipe::RootlessA => recipe.generate_rootless(root_pc, quality),
                        _ => Vec::new(),
                    };

                    for voice_set in &voice_sets {
                        let mut fingerings = map_voice_set(voice_set, &fretboard, &rules);
                        rank_fingerings(&mut fingerings, voice_set, &fretboard);
                        voicings.extend(fingerings.into_iter().take(3).map(|fingering| {
                            VoicingData {
                                recipe: voice_set.recipe,
                                fingering,
                            }
                        }));
                    }
                }

                voicings.sort_by(|a, b| {
                    recipe_order(a.recipe)
                        .cmp(&recipe_order(b.recipe))
                        .then_with(|| a.fingering.positions.cmp(&b.fingering.positions))
                });
                voicings.dedup_by(|a, b| a.fingering.positions == b.fingering.positions);
                voicings.truncate(8);

                chords.push(ChordData { name, voicings });
            }
        }

        Self {
            chords,
            selected_chord: 0,
            selected_voicing: 0,
            focused_pane: BrowserPane::Chords,
        }
    }

    /// Navigate up in the focused list.
    pub fn move_up(&mut self) {
        match self.focused_pane {
            BrowserPane::Chords => {
                if self.selected_chord > 0 {
                    self.selected_chord -= 1;
                    self.selected_voicing = 0;
                }
            }
            BrowserPane::Voicings => {
                if self.selected_voicing > 0 {
                    self.selected_voicing -= 1;
                }
            }
        }
    }

    /// Navigate down in the focused list.
    pub fn move_down(&mut self) {
        match self.focused_pane {
            BrowserPane::Chords => {
                if self.selected_chord + 1 < self.chords.len() {
                    self.selected_chord += 1;
                    self.selected_voicing = 0;
                }
            }
            BrowserPane::Voicings => {
                let chord = &self.chords[self.selected_chord];
                if self.selected_voicing + 1 < chord.voicings.len() {
                    self.selected_voicing += 1;
                }
            }
        }
    }

    /// Move focus left.
    pub fn focus_previous(&mut self) {
        self.focused_pane = BrowserPane::Chords;
    }

    /// Move focus right.
    pub fn focus_next(&mut self) {
        self.focused_pane = BrowserPane::Voicings;
    }

    /// Render the browser screen onto the given frame.
    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(frame.area());

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                ]
                .as_ref(),
            )
            .split(chunks[0]);

        // --- Chord list ---
        let chord_items: Vec<ListItem> = self
            .chords
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let style = if i == self.selected_chord {
                    selected_style(self.focused_pane == BrowserPane::Chords)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(&c.name, style)))
            })
            .collect();

        let chord_list =
            List::new(chord_items).block(Block::default().title(" Chords ").borders(Borders::ALL));
        frame.render_widget(chord_list, main_chunks[0]);

        // --- Voicing list ---
        let chord = &self.chords[self.selected_chord];
        let voicing_items: Vec<ListItem> = chord
            .voicings
            .iter()
            .enumerate()
            .map(|(i, voicing)| {
                let label = self.voicing_label(voicing);
                let style = if i == self.selected_voicing {
                    selected_style(self.focused_pane == BrowserPane::Voicings)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let voicing_list = List::new(voicing_items)
            .block(Block::default().title(" Voicings ").borders(Borders::ALL));
        frame.render_widget(voicing_list, main_chunks[1]);

        // --- Diagram ---
        let diagram_text = if chord.voicings.is_empty() {
            Text::from("No fingerings available")
        } else {
            let fingering = &chord.voicings[self.selected_voicing].fingering;
            let diagram = render_fingering(fingering, &Fretboard::standard_tuning(), &chord.name);
            Text::from(diagram)
        };

        let diagram_title = if chord.voicings.is_empty() {
            format!(" {} ", chord.name)
        } else {
            format!(
                " {} {} ",
                chord.name,
                chord.voicings[self.selected_voicing].recipe.name()
            )
        };
        let diagram = Paragraph::new(diagram_text).block(
            Block::default()
                .title(diagram_title.as_str())
                .borders(Borders::ALL),
        );
        frame.render_widget(diagram, main_chunks[2]);

        // --- Status bar ---
        let status = if chord.voicings.is_empty() {
            format!(
                " {} (no fingerings)  |  j/k navigate  h/l pane  q quit",
                chord.name
            )
        } else {
            let voicing = &chord.voicings[self.selected_voicing];
            format!(
                " {} | {} | Voicing {}/{}  |  j/k navigate  h/l pane  q quit",
                chord.name,
                voicing.recipe.name(),
                self.selected_voicing + 1,
                chord.voicings.len()
            )
        };
        let status_bar = Paragraph::new(Line::from(Span::styled(
            status,
            Style::default().add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(status_bar, chunks[1]);
    }

    /// Format a voicing label showing the fret positions for played strings.
    fn voicing_label(&self, voicing: &VoicingData) -> String {
        let labels: Vec<&str> = vec!["E", "A", "D", "G", "B", "e"];
        let parts: Vec<String> = voicing
            .fingering
            .positions
            .iter()
            .enumerate()
            .filter_map(|(i, pos)| {
                pos.map(|fret| {
                    if fret == 0 {
                        format!("{}:O", labels[i])
                    } else {
                        format!("{}:{}", labels[i], fret)
                    }
                })
            })
            .collect();
        let intervals: Vec<&str> = voicing
            .fingering
            .played_intervals()
            .iter()
            .map(|interval| interval.name)
            .collect();
        format!(
            "{} [{}] {}",
            voicing.recipe.name(),
            intervals.join(" "),
            parts.join(" ")
        )
    }
}

impl Default for BrowserScreen {
    fn default() -> Self {
        Self::new()
    }
}

fn selected_style(focused: bool) -> Style {
    let mut style = Style::default().add_modifier(Modifier::REVERSED);
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

fn recipe_order(recipe: VoicingRecipe) -> usize {
    match recipe {
        VoicingRecipe::RootlessA => 0,
        VoicingRecipe::Shell => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::intervals::Interval;

    #[test]
    fn browser_includes_rootless_voicings() {
        let screen = BrowserScreen::new();
        let cmaj9 = screen
            .chords
            .iter()
            .find(|chord| chord.name == "Cmaj9")
            .expect("Cmaj9 should be in the browser");

        assert!(
            cmaj9.voicings.iter().any(|voicing| {
                voicing.recipe == VoicingRecipe::RootlessA
                    && !voicing.fingering.has_interval(Interval::UNISON)
            }),
            "browser should expose rootless Cmaj9 voicings"
        );
    }

    #[test]
    fn browser_default_navigation_moves_chords() {
        let mut screen = BrowserScreen::new();
        assert_eq!(screen.selected_chord, 0);

        screen.move_down();

        assert_eq!(screen.selected_chord, 1);
        assert_eq!(screen.selected_voicing, 0);
    }
}
