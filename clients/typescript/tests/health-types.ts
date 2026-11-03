import type { OntoforgeClient } from '../src/index.js';

export async function healthStatus(client: OntoforgeClient): Promise<[string, string | null | undefined]> {
  const health = await client.health();
  return [health.l2?.status ?? health.status, health.l2?.reason];
}
