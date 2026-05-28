/** Closest sampled MIDI to `midi`; ties resolve to the lower (first-found minimum). */
export function nearestSampledMidi(midi: number, sampled: number[]): number {
  return sampled.reduce(
    (best, cur) => (Math.abs(cur - midi) < Math.abs(best - midi) ? cur : best),
    sampled[0],
  );
}
