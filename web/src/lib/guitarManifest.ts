// MIDI number -> public sample URL. Karoryfer Shinyguitar (CC0), electric/DI pickup,
// velocity layer vl2. Sampled notes are spaced ~a minor third apart (flat spellings):
// db2=37 e2=40 gb2=42 a2=45 c3=48 eb3=51 gb3=54 a3=57 c4=60 eb4=63 gb4=66 a4=69
// c5=72 eb5=75 gb5=78 a5=81 c6=84. MIDI = 12*(octave+1) + pitchClass.
export const GUITAR_MANIFEST: Record<number, string> = {
  37: '/samples/guitar-jazz/db2.mp3',
  40: '/samples/guitar-jazz/e2.mp3',
  42: '/samples/guitar-jazz/gb2.mp3',
  45: '/samples/guitar-jazz/a2.mp3',
  48: '/samples/guitar-jazz/c3.mp3',
  51: '/samples/guitar-jazz/eb3.mp3',
  54: '/samples/guitar-jazz/gb3.mp3',
  57: '/samples/guitar-jazz/a3.mp3',
  60: '/samples/guitar-jazz/c4.mp3',
  63: '/samples/guitar-jazz/eb4.mp3',
  66: '/samples/guitar-jazz/gb4.mp3',
  69: '/samples/guitar-jazz/a4.mp3',
  72: '/samples/guitar-jazz/c5.mp3',
  75: '/samples/guitar-jazz/eb5.mp3',
  78: '/samples/guitar-jazz/gb5.mp3',
  81: '/samples/guitar-jazz/a5.mp3',
  84: '/samples/guitar-jazz/c6.mp3',
};
