import type { KernelClient } from "./client";
import { usesNativeKernel } from '../platform';
export type {
  KernelClient,
  Request,
  Response,
  Event,
  Unsubscribe,
} from "./client";
export async function createKernelClient(
  onCreated?: (client: KernelClient) => void,
): Promise<KernelClient> {
  const client =
    usesNativeKernel()
      ? new (await import("./tauriClient")).TauriClient()
      : new (await import("./wasmClient")).WasmClient();
  onCreated?.(client);
  try {
    await client.ready;
  } catch (error) {
    client.dispose();
    throw error;
  }
  return client;
}
