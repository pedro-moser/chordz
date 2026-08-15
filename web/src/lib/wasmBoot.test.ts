import { describe, expect, it, vi } from 'vitest';
import { attemptWasmBoot } from './wasmBoot';

describe('attemptWasmBoot', () => {
  it('turns a rejected initialization into an explicit error result', async () => {
    const failure = new Error('network unavailable');

    await expect(attemptWasmBoot(() => Promise.reject(failure))).resolves.toEqual({
      status: 'error',
      error: failure
    });
  });

  it('can recover when a later retry succeeds', async () => {
    const init = vi
      .fn<() => Promise<void>>()
      .mockRejectedValueOnce(new Error('temporary failure'))
      .mockResolvedValueOnce();

    expect((await attemptWasmBoot(init)).status).toBe('error');
    expect(await attemptWasmBoot(init)).toEqual({ status: 'ready' });
    expect(init).toHaveBeenCalledTimes(2);
  });
});
