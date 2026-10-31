import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { mkdtemp, rm, access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { before, after, test } from 'node:test';
import { createServer } from 'node:net';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const binary = process.env.ONTOFORGE_BIN ?? join(repoRoot, 'target', 'debug', process.platform === 'win32' ? 'ontoforge.exe' : 'ontoforge');
const cargo = process.env.CARGO ?? join(process.env.USERPROFILE ?? '', '.cargo', 'bin', process.platform === 'win32' ? 'cargo.exe' : 'cargo');
let dataDir;
let child;
let baseUrl;
let bearer;
let client;

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: repoRoot,
    env: { ...process.env, CARGO_BUILD_JOBS: '2', NO_COLOR: '1' },
    encoding: 'utf8',
    ...options,
  });
}

async function freePort() {
  const server = createServer();
  await new Promise((resolvePromise, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolvePromise);
  });
  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;
  await new Promise((resolvePromise) => server.close(resolvePromise));
  return port;
}

async function waitFor(predicate, timeoutMs = 30000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (child?.exitCode !== null && child?.exitCode !== undefined) {
      throw new Error(`ontoforge serve exited with ${child.exitCode}`);
    }
    if (await predicate()) return;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 100));
  }
  throw new Error('timed out waiting for ontoforge serve');
}

before(async () => {
  try {
    await access(binary, constants.F_OK);
  } catch {
    run(cargo, ['build', '-p', 'ontoforge', '--locked'], { stdio: 'inherit' });
  }
  dataDir = await mkdtemp(join(tmpdir(), 'ontoforge-ts-'));
  const port = await freePort();
  child = spawn(binary, ['serve', '--dev', '--data-dir', dataDir, '--bind', `127.0.0.1:${port}`], {
    cwd: repoRoot,
    stdio: ['ignore', 'ignore', 'pipe'],
    env: { ...process.env, NO_COLOR: '1' },
  });
  let stderr = '';
  child.stderr?.on('data', (chunk) => { stderr += chunk.toString(); });
  baseUrl = `http://127.0.0.1:${port}`;
  await waitFor(async () => {
    try {
      const response = await fetch(`${baseUrl}/health`);
      return response.status === 200 || response.status === 503;
    } catch {
      return false;
    }
  });
  const config = join(dataDir, 'server.toml');
  bearer = run(binary, ['dev-token', '--server-config', config, '--sub', 'alice', '--groups', 'dispatchers']).trim();
  client = (await import('../dist/index.js')).createOntoforgeClient({ baseUrl, bearer });
  child._ontoforgeStderr = () => stderr;
});

after(async () => {
  if (child && child.exitCode === null) {
    child.kill();
    await new Promise((resolvePromise) => child.once('exit', resolvePromise));
  }
  if (dataDir) await rm(dataDir, { recursive: true, force: true });
});

test('ts_client_generated_metadata_pk_query_aggregate_health', async () => {
  const metadata = await client.metadata();
  assert.equal(metadata.apiName, 'aviation');
  assert.match(metadata.description ?? '', /航空数据|aviation/i);
  const row = await client.get('Flight', 'F-1001');
  assert.equal(row.pk, 'F-1001');
  const page = await client.query({
    set: { kind: 'type', object_type: 'Flight' },
    select: ['flightId', 'status'],
    page: { first: 3n },
  });
  assert.ok(page.rows.length > 0);
  const aggregate = await client.aggregate({
    set: { kind: 'type', object_type: 'Flight' },
    groupBy: ['status'],
    measures: [{ op: 'count' }],
  });
  assert.ok(aggregate.buckets.length > 0);
  const health = await client.health();
  assert.equal(health.engineStatus, 'available');
});

test('ts_client_typecheck_generation_and_instance_abort', async () => {
  const { createOntoforgeClient } = await import('../dist/index.js');
  const first = createOntoforgeClient({ baseUrl, bearer: 'invalid-token' });
  const second = createOntoforgeClient({ baseUrl, bearer });
  await assert.rejects(() => first.metadata(), (error) => error.status === 401);
  assert.equal((await second.metadata()).apiName, 'aviation');
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(() => second.metadata({ signal: controller.signal }), (error) => error?.name === 'AbortError');
});

test('ts_lossless_integer_decimal_boundary_roundtrips', async () => {
  const { decodeWire, encodeWire } = await import('../dist/index.js');
  const wire = '{"safe":9007199254740991,"above":9007199254740993,"i64Min":-9223372036854775808,"i64Max":9223372036854775807,"decimal":"0.123456789012345678901234567890","unicode":"航空\ud83d\ude80"}';
  const decoded = decodeWire(wire);
  assert.equal(decoded.safe, 9007199254740991n);
  assert.equal(decoded.above, 9007199254740993n);
  assert.equal(decoded.i64Min, -9223372036854775808n);
  assert.equal(decoded.i64Max, 9223372036854775807n);
  assert.equal(decoded.decimal, '0.123456789012345678901234567890');
  assert.equal(decoded.unicode, '航空🚀');
  assert.equal(encodeWire(decoded), wire);
});

test('ts_client_action_validate_history_result_and_replay', async () => {
  const commandId = `ts-client-${Date.now()}`;
  const validation = await client.validate('delayFlight', {
    items: [{ params: { flight: 'F-1001', delayMinutes: 15 } }],
  });
  assert.equal(validation.validated, true);
  const first = await client.action('delayFlight', {
    commandId,
    items: [{ params: { flight: 'F-1001', delayMinutes: 15 } }],
  });
  assert.equal(typeof first.commandId, 'string');
  const replay = await client.action('delayFlight', {
    commandId,
    items: [{ params: { flight: 'F-1001', delayMinutes: 15 } }],
  });
  assert.deepEqual(replay, first);
  assert.deepEqual(await client.commandResult(commandId), first);
  const history = await client.history('Flight', 'F-1001', { limit: 20n });
  assert.equal(history.entries.filter((entry) => entry.action === 'delayFlight').length, 1);
});

test('ts_client_auth_scope_stale_errors_limits_and_runtime_capability', async () => {
  const { LosslessCodecError, decodeWire, encodeWire, createOntoforgeClient } = await import('../dist/index.js');
  const noGrant = createOntoforgeClient({ baseUrl, bearer: run(binary, ['dev-token', '--server-config', join(dataDir, 'server.toml'), '--sub', 'bob']).trim() });
  await assert.rejects(
    () => noGrant.validate('delayFlight', { items: [{ params: { flight: 'F-1001', delayMinutes: 1 } }] }),
    (error) => error.status === 403 || error.status === 404,
  );
  const invalid = await fetch(`${baseUrl}/api/query`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${bearer}`, 'Content-Type': 'application/json' },
    body: '{"set":',
  });
  assert.equal(invalid.status, 400);
  const huge = await fetch(`${baseUrl}/api/query`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${bearer}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ set: { kind: 'type', object_type: 'Flight' }, page: { first: 1 }, padding: 'x'.repeat(17 * 1024 * 1024) }),
  });
  assert.equal(huge.status, 413);
  const original = JSON.rawJSON;
  try {
    Object.defineProperty(JSON, 'rawJSON', { configurable: true, value: undefined });
    assert.throws(() => encodeWire({ value: 1n }), LosslessCodecError);
  } finally {
    Object.defineProperty(JSON, 'rawJSON', { configurable: true, value: original });
  }
  const originalParse = JSON.parse;
  try {
    Object.defineProperty(JSON, 'parse', {
      configurable: true,
      value: (source, reviver) => originalParse(source, reviver ? (key, value) => reviver(key, value) : undefined),
    });
    assert.throws(() => decodeWire('{"value":9007199254740993}'), LosslessCodecError);
  } finally {
    Object.defineProperty(JSON, 'parse', { configurable: true, value: originalParse });
  }
});

async function withoutLosslessCapability(capability, callback) {
  const descriptor = Object.getOwnPropertyDescriptor(JSON, capability === 'rawJSON' ? 'rawJSON' : 'parse');
  const originalParse = JSON.parse;
  try {
    Object.defineProperty(JSON, capability === 'rawJSON' ? 'rawJSON' : 'parse', {
      configurable: true,
      value: capability === 'rawJSON'
        ? undefined
        : (source, reviver) => originalParse(source, reviver ? (key, value) => reviver(key, value) : undefined),
    });
    await callback();
  } finally {
    Object.defineProperty(JSON, capability === 'rawJSON' ? 'rawJSON' : 'parse', descriptor);
  }
}

test('ts_client_rejects_missing_lossless_runtime_before_construction', async () => {
  const { createOntoforgeClient, LosslessCodecError } = await import('../dist/index.js');
  for (const capability of ['rawJSON', 'context.source']) {
    let fetchCalls = 0;
    await withoutLosslessCapability(capability, async () => {
      assert.throws(() => createOntoforgeClient({
        baseUrl,
        fetch: async () => {
          fetchCalls += 1;
          return new Response('{}');
        },
      }), LosslessCodecError);
      assert.equal(fetchCalls, 0, capability);
    });
  }
});

test('ts_client_rechecks_lossless_runtime_before_every_request', async () => {
  const { createOntoforgeClient, LosslessCodecError } = await import('../dist/index.js');
  for (const capability of ['rawJSON', 'context.source']) {
    let fetchCalls = 0;
    const supportedClient = createOntoforgeClient({
      baseUrl,
      fetch: async () => {
        fetchCalls += 1;
        return new Response('{}');
      },
    });
    await withoutLosslessCapability(capability, async () => {
      await assert.rejects(() => supportedClient.action('delayFlight', {
        commandId: `unsupported-${capability}`,
        items: [{ params: { flight: 'F-1001', delayMinutes: 1 } }],
      }), LosslessCodecError);
      await assert.rejects(() => supportedClient.metadata(), LosslessCodecError);
      assert.equal(fetchCalls, 0, `${capability} must fail before any request is sent`);
    });
  }
});

test('ts_client_public_endpoints_preserve_bigint_without_raw_escape', async () => {
  const { createOntoforgeClient } = await import('../dist/index.js');
  const losslessClient = createOntoforgeClient({
    baseUrl,
    fetch: async () => new Response('{"defVersion":9007199254740993}', {
      headers: { 'Content-Type': 'application/json' },
    }),
  });
  assert.equal('raw' in losslessClient, false, 'only lossless endpoint methods are public');
  const requests = [
    () => losslessClient.metadata(),
    () => losslessClient.get('Flight', 'F-1001'),
    () => losslessClient.query({ set: { kind: 'type', object_type: 'Flight' } }),
    () => losslessClient.aggregate({ set: { kind: 'type', object_type: 'Flight' }, measures: [{ op: 'count' }] }),
    () => losslessClient.health(),
    () => losslessClient.action('delayFlight', { items: [{ params: { flight: 'F-1001', delayMinutes: 1 } }] }),
    () => losslessClient.validate('delayFlight', { items: [{ params: { flight: 'F-1001', delayMinutes: 1 } }] }),
    () => losslessClient.commandResult('lossless-result'),
    () => losslessClient.history('Flight', 'F-1001'),
  ];
  for (const request of requests) {
    assert.equal((await request()).defVersion, 9007199254740993n);
  }
});
