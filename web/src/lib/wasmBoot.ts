export type WasmBootState =
  | { status: 'loading' }
  | { status: 'ready' }
  | { status: 'error'; error: unknown };

export async function attemptWasmBoot(
  initialize: () => Promise<void>
): Promise<Exclude<WasmBootState, { status: 'loading' }>> {
  try {
    await initialize();
    return { status: 'ready' };
  } catch (error) {
    return { status: 'error', error };
  }
}
