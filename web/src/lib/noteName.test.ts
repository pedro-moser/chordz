import { describe, it, expect } from 'vitest';
import { noteNameToMidi } from './noteName';

describe('noteNameToMidi', () => {
  it('maps natural notes', () => {
    expect(noteNameToMidi('C4')).toBe(60);
    expect(noteNameToMidi('A4')).toBe(69);
    expect(noteNameToMidi('E2')).toBe(40);
  });
  it('maps sharps written with "s" or "#"', () => {
    expect(noteNameToMidi('Cs3')).toBe(49); // C3=48
    expect(noteNameToMidi('C#3')).toBe(49);
    expect(noteNameToMidi('Ds4')).toBe(63); // D4=62
  });
  it('throws on garbage', () => {
    expect(() => noteNameToMidi('H9')).toThrow();
  });
});
