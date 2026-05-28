export interface EffectsChain {
  /** Connect sample sources here. */
  input: AudioNode;
  /** 0 = dry, 1 = max ambience (reverb + delay). */
  setAmbience(amount: number): void;
}

/** Stereo decaying-noise impulse response — a plate/hall-ish reverb with zero asset. */
function makeImpulseResponse(ctx: AudioContext, seconds = 2.2, decay = 3.0): AudioBuffer {
  const length = Math.floor(ctx.sampleRate * seconds);
  const ir = ctx.createBuffer(2, length, ctx.sampleRate);
  for (let ch = 0; ch < 2; ch++) {
    const data = ir.getChannelData(ch);
    for (let i = 0; i < length; i++) {
      const t = i / length;
      data[i] = (Math.random() * 2 - 1) * Math.pow(1 - t, decay);
    }
  }
  return ir;
}

export function createEffectsChain(ctx: AudioContext): EffectsChain {
  const input = ctx.createGain();
  const lowpass = ctx.createBiquadFilter();
  lowpass.type = 'lowpass';
  lowpass.frequency.value = 2800; // gentle warmth (DI archtop is already mellow); tune by ear
  lowpass.Q.value = 0.7;

  const dry = ctx.createGain();
  const wet = ctx.createGain();
  const convolver = ctx.createConvolver();
  convolver.buffer = makeImpulseResponse(ctx);
  const master = ctx.createGain();
  master.gain.value = 0.9;

  // Short feedback delay (modern-jazz ambience), mixed in low.
  const delay = ctx.createDelay(1.0);
  delay.delayTime.value = 0.3; // 300 ms
  const feedback = ctx.createGain();
  feedback.gain.value = 0.3; // a few decaying repeats
  const delayWet = ctx.createGain();

  input.connect(lowpass);
  lowpass.connect(dry);
  lowpass.connect(convolver);
  convolver.connect(wet);
  lowpass.connect(delay);
  delay.connect(feedback);
  feedback.connect(delay);
  delay.connect(delayWet);
  dry.connect(master);
  wet.connect(master);
  delayWet.connect(master);
  master.connect(ctx.destination);

  function setAmbience(amount: number) {
    const a = Math.min(1, Math.max(0, amount));
    wet.gain.value = a;
    delayWet.gain.value = a * 0.3; // ~10% delay mix at the default ambience (0.35)
    dry.gain.value = 1 - a * 0.4; // keep dry mostly present even when wet
  }
  setAmbience(0.35); // moderate default

  return { input, setAmbience };
}
