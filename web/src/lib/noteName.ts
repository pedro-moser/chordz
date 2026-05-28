const PITCH_CLASS: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };

/** Parse a note name like "A2", "Cs3" (C#3) or "C#3" into a MIDI number (C4 = 60). */
export function noteNameToMidi(name: string): number {
  const m = name.match(/^([A-G])(s|#)?(-?\d+)$/);
  if (!m) throw new Error(`bad note name: ${name}`);
  let pc = PITCH_CLASS[m[1]];
  if (m[2]) pc += 1;
  const octave = parseInt(m[3], 10);
  return 12 * (octave + 1) + pc;
}
