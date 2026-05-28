// MIDI number -> public sample URL. Keys computed from filenames via noteNameToMidi
// (E2=40, Fs2=42, A2=45, C3=48, Ds3=51, Fs3=54, A3=57, C4=60, Ds4=63, Fs4=66, A4=69, C5=72, Ds5=75).
export const GUITAR_MANIFEST: Record<number, string> = {
  40: '/samples/guitar-electric/E2.mp3',
  42: '/samples/guitar-electric/Fs2.mp3',
  45: '/samples/guitar-electric/A2.mp3',
  48: '/samples/guitar-electric/C3.mp3',
  51: '/samples/guitar-electric/Ds3.mp3',
  54: '/samples/guitar-electric/Fs3.mp3',
  57: '/samples/guitar-electric/A3.mp3',
  60: '/samples/guitar-electric/C4.mp3',
  63: '/samples/guitar-electric/Ds4.mp3',
  66: '/samples/guitar-electric/Fs4.mp3',
  69: '/samples/guitar-electric/A4.mp3',
  72: '/samples/guitar-electric/C5.mp3',
  75: '/samples/guitar-electric/Ds5.mp3',
};
