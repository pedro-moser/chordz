export interface EffectsChain {
  /** Connect sample sources here. */
  input: AudioNode;
  /** 0 = dry, 1 = max reverb. */
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
  lowpass.frequency.value = 3500; // warmth; tune by ear
  lowpass.Q.value = 0.7;

  const dry = ctx.createGain();
  const wet = ctx.createGain();
  const convolver = ctx.createConvolver();
  convolver.buffer = makeImpulseResponse(ctx);
  const master = ctx.createGain();
  master.gain.value = 0.9;

  input.connect(lowpass);
  lowpass.connect(dry);
  lowpass.connect(convolver);
  convolver.connect(wet);
  dry.connect(master);
  wet.connect(master);
  master.connect(ctx.destination);

  function setAmbience(amount: number) {
    const a = Math.min(1, Math.max(0, amount));
    wet.gain.value = a;
    dry.gain.value = 1 - a * 0.4; // keep dry mostly present even when wet
  }
  setAmbience(0.35); // moderate default

  return { input, setAmbience };
}
