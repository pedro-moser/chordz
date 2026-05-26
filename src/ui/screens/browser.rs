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
    pub fingerings: Vec<Fingering>,
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
            min_strings: 3,
            max_strings: 6,
            max_fret_span: 5,
            max_fret: 15,
            require_root: false,
        };

        // Qualities to browse: maj7, m7, dom7, maj9.
        let quality_names = &["maj7", "m7", "dom7", "maj9"];

        let mut chords = Vec::new();

        for &root in &chords::ROOTS {
            let root_pc = chords::root_to_pc(root).unwrap();

            for &qname in quality_names {
                let quality = ChordQuality::ALL.iter().find(|q| q.name == qname).unwrap();

                let name = chords::chord_name(root, quality);

                // Generate shell voice sets.
                let voice_sets = VoicingRecipe::Shell.generate_shell(root_pc, quality);
                let mut all_fingerings = Vec::new();

                for vs in &voice_sets {
                    let fingerings = map_voice_set(vs, &fretboard, &rules);
                    all_fingerings.extend(fingerings);
                }

                // Rank and deduplicate.
                if let Some(vs) = voice_sets.first() {
                    rank_fingerings(&mut all_fingerings, vs, &fretboard);
                }
                all_fingerings.dedup();

                // Keep top 5 fingerings per chord to keep the list manageable.
                all_fingerings.truncate(5);

                chords.push(ChordData {
                    name,
                    fingerings: all_fingerings,
                });
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
                if self.selected_voicing + 1 < chord.fingerings.len() {
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
            .fingerings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let label = self.voicing_label(f);
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
        let diagram_text = if chord.fingerings.is_empty() {
            Text::from("No fingerings available")
        } else {
            let fingering = &chord.fingerings[self.selected_voicing];
            let diagram = render_fingering(fingering, &Fretboard::standard_tuning(), &chord.name);
            Text::from(diagram)
        };

        let diagram_title = format!(" {} ", chord.name);
        let diagram = Paragraph::new(diagram_text).block(
            Block::default()
                .title(diagram_title.as_str())
                .borders(Borders::ALL),
        );
        frame.render_widget(diagram, main_chunks[2]);

        // --- Status bar ---
        let status = if chord.fingerings.is_empty() {
            format!(
                " {} (no fingerings)  |  j/k navigate  h/l pane  q quit",
                chord.name
            )
        } else {
            format!(
                " {} | Voicing {}/{}  |  j/k navigate  h/l pane  q quit",
                chord.name,
                self.selected_voicing + 1,
                chord.fingerings.len()
            )
        };
        let status_bar = Paragraph::new(Line::from(Span::styled(
            status,
            Style::default().add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(status_bar, chunks[1]);
    }

    /// Format a voicing label showing the fret positions for played strings.
    fn voicing_label(&self, fingering: &Fingering) -> String {
        let labels: Vec<&str> = vec!["E", "A", "D", "G", "B", "e"];
        let parts: Vec<String> = fingering
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
        parts.join(" ")
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
