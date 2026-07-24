import { describe, it, expect } from 'vitest';
import { staffStep, ledgerSteps } from './notation';

describe('staffStep', () => {
  it('puts written E4 on the bottom line', () => {
    // Sounding E3 (octave 3, step 2) writes as E4 under the 8vb clef.
    expect(staffStep({ step: 2, alter: 0, octave: 3 })).toBe(0);
  });

  it('puts written F5 on the top line', () => {
    // Sounding F4 writes as F5.
    expect(staffStep({ step: 3, alter: 0, octave: 4 })).toBe(8);
  });

  it('puts the guitar open low E three ledger lines below', () => {
    // Sounding E2 = MIDI 40.
    expect(staffStep({ step: 2, alter: 0, octave: 2 })).toBe(-7);
  });

  it('ignores the accidental — Eb and E share a staff position', () => {
    expect(staffStep({ step: 2, alter: -1, octave: 3 })).toBe(
      staffStep({ step: 2, alter: 0, octave: 3 }),
    );
  });
});

describe('ledgerSteps', () => {
  it('returns nothing for a note inside the staff', () => {
    expect(ledgerSteps(4)).toEqual([]);
    expect(ledgerSteps(0)).toEqual([]);
    expect(ledgerSteps(8)).toEqual([]);
  });

  it('returns the even steps down to a low note', () => {
    expect(ledgerSteps(-7)).toEqual([-2, -4, -6]);
  });

  it('returns the even steps up to a high note', () => {
    expect(ledgerSteps(13)).toEqual([10, 12]);
  });

  it('includes the note own line when it sits exactly on a ledger', () => {
    expect(ledgerSteps(-4)).toEqual([-2, -4]);
  });
});
