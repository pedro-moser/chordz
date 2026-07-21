import { describe, it, expect } from 'vitest';
import { withBase } from './assetUrl';

describe('withBase', () => {
  it('leaves the path untouched when there is no base', () => {
    expect(withBase('', '/samples/guitar-jazz/c4.mp3')).toBe('/samples/guitar-jazz/c4.mp3');
  });
  it('prefixes the base for a subpath deploy', () => {
    expect(withBase('/chordz', '/samples/guitar-jazz/c4.mp3')).toBe(
      '/chordz/samples/guitar-jazz/c4.mp3'
    );
  });
  it('does not double the slash when the path lacks a leading one', () => {
    expect(withBase('/chordz', 'samples/c4.mp3')).toBe('/chordz/samples/c4.mp3');
  });
  it('drops a trailing slash on the base', () => {
    expect(withBase('/chordz/', '/samples/c4.mp3')).toBe('/chordz/samples/c4.mp3');
  });
});
