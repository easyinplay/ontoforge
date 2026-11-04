import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { mkdtemp, rm, access, readFile, writeFile } from 'node:fs/promises';
import { constants } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { before, after, test } from 'node:test';
import { createServer } from 'node:net';
import { createServer as createHttpServer } from 'node:http';
import { durableState, revokeDispatchers, faultProxy } from './support-security.mjs';
import './numeric-security.test.mjs';

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
  const actionsPath = join(dataDir, 'ontology', 'actions.yaml');
  const actions = await readFile(actionsPath, 'utf8');
  assert.ok(actions.includes('      delayMinutes: { param: delayMinutes }'));
  await writeFile(actionsPath, actions.replace(
    '      delayMinutes: { param: delayMinutes }',
    '  - kind: modifyObject\n    object: flight\n    properties:\n      delayMinutes: { param: delayMinutes }',
  ));
  run(binary, ['reload', join(dataDir, 'ontology'), '--server-config', config]);
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
  assert.equal(health.status, 'healthy');
  assert.equal(Object.hasOwn(health, 'l2'), false);
});

test('ts_client_health_l2_types_and_decode_preserve_optional_reason', async () => {
  run(process.execPath, [
    join(repoRoot, 'clients/typescript/node_modules/typescript/bin/tsc'),
    '--noEmit', '--module', 'NodeNext', '--moduleResolution', 'NodeNext',
    '--target', 'ES2022', '--strict',
    'clients/typescript/tests/health-types.ts',
  ]);
  const { createOntoforgeClient } = await import('../dist/index.js');
  for (const health of [
    { status: 'healthy', engineStatus: 'available', l2: { status: 'enabled' } },
    { status: 'degraded', engineStatus: 'available', l2: { status: 'disabled', reason: 'EXPIRED' } },
  ]) {
    const api = createOntoforgeClient({ baseUrl, fetch: async () => new Response(JSON.stringify(health)) });
    assert.deepEqual(await api.health(), health);
  }
});

test('ts_client_action_error_diagnostics_survive_real_http', async () => {
  for (const validateOnly of [true, false]) {
    await assert.rejects(() => client.action('delayFlight', {
      commandId: `ts-diagnostics-params-${validateOnly}`,
      validateOnly,
      items: [{ params: { delayMinutes: 'wrong' } }],
    }), (error) => {
      assert.equal(error.code, 'INVALID_PARAMETERS');
      assert.equal(error.message, 'INVALID_PARAMETERS (400)');
      assert.deepEqual(error.body.parameterErrors, [
        { itemOrdinal: 0n, name: 'delayMinutes', reason: 'expected an integer, found a string' },
        { itemOrdinal: 0n, name: 'flight', reason: 'required parameter is missing' },
      ]);
      assert.deepEqual(error.envelope.parameterErrors, error.body.parameterErrors);
      assert.equal(error.envelope.message, undefined);
      return true;
    });
    await assert.rejects(() => client.action('delayFlight', {
      commandId: `ts-diagnostics-criteria-${validateOnly}`,
      validateOnly,
      items: [{ params: { flight: 'F-1001', delayMinutes: 0n } }],
    }), (error) => {
      assert.equal(error.code, 'CRITERIA_FAILED');
      assert.equal(error.message, 'CRITERIA_FAILED (400)');
      assert.equal(error.envelope.message, 'Delay must be positive and cancelled flights cannot be delayed');
      assert.equal(error.envelope.message, error.body.message);
      assert.equal(error.envelope.parameterErrors, undefined);
      return true;
    });
  }
});

test('ts_client_action_error_diagnostics_types', () => {
  run(process.execPath, [
    join(repoRoot, 'clients/typescript/node_modules/typescript/bin/tsc'),
    '--noEmit', '--module', 'NodeNext', '--moduleResolution', 'NodeNext',
    '--target', 'ES2022', '--strict',
    'clients/typescript/tests/error-types.ts',
  ]);
});

test('ts_client_action_error_diagnostics_reject_malformed_fields', async () => {
  const { createOntoforgeClient, OntoforgeHttpError, encodeWire } = await import('../dist/index.js');
  for (const diagnostic of [
    { parameterErrors: 'wrong' },
    { parameterErrors: [{}] },
    { parameterErrors: [{ itemOrdinal: -1n, name: 'flight', reason: 'bad' }] },
    { parameterErrors: [{ itemOrdinal: 4294967296n, name: 'flight', reason: 'bad' }] },
    { parameterErrors: [{ itemOrdinal: 0n, name: 1n, reason: 'bad' }] },
    { parameterErrors: [{ itemOrdinal: 0n, name: 'flight', reason: false }] },
    { message: 42n },
  ]) {
    const api = createOntoforgeClient({
      baseUrl,
      fetch: async () => new Response(encodeWire({
        code: 'INVALID_PARAMETERS', apiName: 'delayFlight', request_id: 'request-1', ...diagnostic,
      }), { status: 400 }),
    });
    await assert.rejects(() => api.metadata(), (error) => {
      assert.equal(error instanceof OntoforgeHttpError, false);
      assert.match(error.message, /response/i);
      return true;
    });
  }
});

test('ts_client_recognizes_envelope_before_validating_diagnostics', async () => {
  const { createOntoforgeClient, OntoforgeHttpError } = await import('../dist/index.js');
  let body;
  const server = createHttpServer((request, response) => {
    response.writeHead(502, { 'Content-Type': 'application/json', Connection: 'close' });
    response.end(JSON.stringify(body));
  });
  await new Promise((resolvePromise) => server.listen(0, '127.0.0.1', resolvePromise));
  try {
    const peer = createOntoforgeClient({ baseUrl: `http://127.0.0.1:${server.address().port}` });
    for (const ordinaryBody of [
      { message: { reason: 'upstream' } },
      { parameterErrors: 'upstream' },
      { code: 'UPSTREAM', message: { reason: 'upstream' } },
      { code: 'UPSTREAM', apiName: 'gateway', message: { reason: 'upstream' } },
      { code: 'UPSTREAM', apiName: 'gateway', request_id: false, message: { reason: 'upstream' } },
    ]) {
      body = ordinaryBody;
      await assert.rejects(() => peer.metadata(), (error) => {
        assert.ok(error instanceof OntoforgeHttpError);
        assert.equal(error.status, 502);
        assert.deepEqual(error.body, ordinaryBody);
        assert.equal(error.envelope, undefined);
        assert.equal(error.message, ordinaryBody.code ? 'UPSTREAM (502)' : 'Ontoforge request failed with HTTP 502');
        return true;
      });
    }
    for (const diagnostics of [{ message: { reason: 'upstream' } }, { parameterErrors: 'wrong' }]) {
      body = { code: 'CRITERIA_FAILED', apiName: 'delayFlight', request_id: 'request-1', ...diagnostics };
      await assert.rejects(() => peer.metadata(), (error) => {
        assert.equal(error instanceof OntoforgeHttpError, false);
        assert.match(error.message, /response/i);
        return true;
      });
    }
  } finally {
    await new Promise((resolvePromise) => server.close(resolvePromise));
  }
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

test('ts_valid_principals_real_revoke_and_inflight_abort_are_isolated', { timeout: 15000 }, async () => {
  const { createOntoforgeClient } = await import('../dist/index.js');
  const miaBearer = run(binary, ['dev-token', '--server-config', join(dataDir, 'server.toml'), '--sub', 'mia']).trim();
  const mia = createOntoforgeClient({ baseUrl, bearer: miaBearer });
  const [aliceMetadata, miaMetadata] = await Promise.all([client.metadata(), mia.metadata()]);
  assert.ok(aliceMetadata.objectTypes.some((type) => type.apiName === 'Flight'));
  assert.ok(!aliceMetadata.objectTypes.some((type) => type.apiName === 'Aircraft'));
  assert.ok(miaMetadata.objectTypes.some((type) => type.apiName === 'Aircraft'));
  assert.ok(!miaMetadata.objectTypes.some((type) => type.apiName === 'Flight'));
  const errors = await Promise.allSettled([client.get('Aircraft', 'N100'), mia.get('Flight', 'F-1001')]);
  assert.ok(errors.every((result) => result.status === 'rejected' && result.reason.code === 'NOT_FOUND'));
  assert.ok(errors.every((result) => typeof result.reason.requestId === 'string' && result.reason.requestId.length > 0));
  assert.notEqual(errors[0].reason.requestId, errors[1].reason.requestId);

  const proxy = await faultProxy(baseUrl, 'hold-both');
  try {
    const isolated = createOntoforgeClient({ baseUrl: proxy.url, bearer });
    const controller = new AbortController();
    const pending = isolated.get('Flight', 'F-1001', { signal: controller.signal });
    const rejected = assert.rejects(pending, (error) => error.name === 'AbortError');
    const independent = isolated.get('Flight', 'F-1001');
    await Promise.race([proxy.reached, new Promise((_, reject) => {
      const timer = setTimeout(() => reject(new Error('proxy did not observe an actual upstream response')), 5000);
      timer.unref();
    })]);
    controller.abort();
    await rejected;
    proxy.release();
    assert.equal((await independent).pk, 'F-1001');
    assert.deepEqual(proxy.counts(), { requests: 2, forwards: 2 });
  } finally { await proxy.close(); }

  const restore = revokeDispatchers(join(dataDir, 'editstore.db'));
  try {
    await assert.rejects(() => client.get('Flight', 'F-1001'), (error) => error.code === 'NOT_FOUND' && !!error.requestId);
    assert.ok((await mia.metadata()).objectTypes.some((type) => type.apiName === 'Aircraft'));
  } finally { restore(); }
  assert.equal((await client.get('Flight', 'F-1001')).pk, 'F-1001');
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
  assert.equal(first.items[0].itemOrdinal, 0n);
  assert.equal(first.items[0].edits.length, 2);
  assert.deepEqual(first.items[0].edits.map((edit) => edit.ruleOrdinal), [0n, 1n]);
  assert.deepEqual(first.items[0].edits.map((edit) => edit.editOrdinal), [0n, 1n]);
  assert.ok(first.items[0].edits.every((edit) => edit.kind === 'object' && edit.operation === 'set' && edit.typeName === 'Flight' && edit.pk === 'F-1001' && typeof edit.versionAfter === 'bigint' && edit.versionAfter > 0n));
  const replay = await client.action('delayFlight', {
    commandId,
    items: [{ params: { flight: 'F-1001', delayMinutes: 15 } }],
  });
  assert.deepEqual(replay, first);
  assert.deepEqual(await client.commandResult(commandId), first);
  const history = await client.history('Flight', 'F-1001', { limit: 20n });
  assert.equal(history.entries.filter((entry) => entry.action === 'delayFlight').length, 2);
  assert.equal(history.entries[0].eventId, first.eventId);
  assert.equal(history.entries[0].committedAt, first.committedAt);
  assert.deepEqual(history.entries[0].maskedChanges, []);
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
  ];
  for (const request of requests) {
    assert.equal((await request()).defVersion, 9007199254740993n);
  }
});

test('ts_client_rejects_malformed_receipt_history_and_validation', async () => {
  const { createOntoforgeClient, encodeWire } = await import('../dist/index.js');
  const receipt = {
    commandId: 'c', eventId: 'e', logSeq: 9007199254740993n,
    committedAt: '2026-11-03T00:00:00Z', chosenCurrentTime: '2026-11-03T00:00:00Z',
    items: [{ itemOrdinal: 0n, edits: [{ ruleOrdinal: 0n, editOrdinal: 0n, kind: 'object', typeName: 'Flight', pk: 'F-1001', operation: 'set', versionAfter: 1n }] }],
  };
  const history = { entries: [{ seq: 1n, eventId: 'e', actor: 'a', action: 'delayFlight', committedAt: 'now', op: 'set', version: 1n, values: {}, unset: [], maskedChanges: ['secret'], params: null }], truncated: false, nextBeforeSeq: 1n };
  const apiFor = (body) => createOntoforgeClient({ baseUrl, fetch: async () => new Response(encodeWire(body)) });
  assert.equal((await apiFor(receipt).commandResult('c')).logSeq, 9007199254740993n);
  assert.deepEqual((await apiFor(history).history('Flight', 'F-1001')).entries[0].maskedChanges, ['secret']);
  assert.deepEqual((await apiFor({ ...receipt, items: [{ itemOrdinal: 0n, edits: [] }] }).commandResult('c')).items[0].edits, []);
  for (const field of ['commandId', 'eventId', 'committedAt', 'chosenCurrentTime', 'logSeq', 'items']) {
    const missing = structuredClone(receipt);
    delete missing[field];
    await assert.rejects(() => apiFor(missing).commandResult('c'), /response/);
  }
  for (const field of ['ruleOrdinal', 'editOrdinal', 'kind', 'typeName', 'pk', 'operation', 'versionAfter']) {
    const missing = structuredClone(receipt);
    delete missing.items[0].edits[0][field];
    await assert.rejects(() => apiFor(missing).commandResult('c'), /response/);
  }
  for (const field of ['eventId', 'committedAt', 'maskedChanges', 'unset', 'values', 'seq', 'version', 'actor', 'action', 'op']) {
    const missing = structuredClone(history);
    delete missing.entries[0][field];
    await assert.rejects(() => apiFor(missing).history('Flight', 'F-1001'), /response/);
  }
  for (const ordinal of [-1n, 4294967296n, '0']) {
    const malformed = structuredClone(receipt);
    malformed.items[0].itemOrdinal = ordinal;
    await assert.rejects(() => apiFor(malformed).commandResult('c'), /response/);
  }
  await assert.rejects(() => apiFor({ ...history, truncated: 'false' }).history('Flight', 'F-1001'), /response/);
  await assert.rejects(() => apiFor({ validated: 'true' }).validate('delayFlight', { items: [] }), /response/);
  await assert.rejects(() => apiFor(receipt).validate('delayFlight', { items: [] }), /response/);
  await assert.rejects(() => apiFor({ validated: true }).action('delayFlight', { items: [] }), /response/);
});

test('ts_transport_failures_never_retry_and_replay_has_one_durable_commit', { timeout: 15000 }, async () => {
  const { createOntoforgeClient, decodeWire } = await import('../dist/index.js');
  const database = join(dataDir, 'editstore.db');
  const request = { commandId: 'ts-loss-after-commit', items: [{ params: { flight: 'F-1001', delayMinutes: 23n } }] };
  const before = durableState(database);
  const early = await faultProxy(baseUrl, 'before');
  try {
    const api = createOntoforgeClient({ baseUrl: early.url, bearer });
    await assert.rejects(() => api.action('delayFlight', request), TypeError);
    assert.deepEqual(early.counts(), { requests: 1, forwards: 0 });
    assert.deepEqual(durableState(database), before);
  } finally { await early.close(); }

  const lost = await faultProxy(baseUrl, 'after');
  let receipt;
  try {
    const api = createOntoforgeClient({ baseUrl: lost.url, bearer });
    await assert.rejects(() => api.action('delayFlight', request), TypeError);
    assert.deepEqual(lost.counts(), { requests: 1, forwards: 1 });
    receipt = decodeWire(lost.receipt());
  } finally { await lost.close(); }
  const committed = durableState(database);
  for (const [table, delta] of Object.entries({ of_edit_log: 1, of_edit_log_entry: 2, of_edit_history: 2, of_action_command: 1, of_webhook_outbox: 1 })) {
    assert.equal(committed[table].length - before[table].length, delta, table);
  }
  for (const [table, rows] of Object.entries(before)) {
    if ((table.startsWith('of_obj_') || table.startsWith('of_lnk_')) && table !== 'of_obj_flight') {
      assert.deepEqual(committed[table], rows, `unrelated business table ${table}`);
    }
  }
  const original = before.of_obj_flight.find((row) => row.flight_id === 'F-1001');
  const changed = committed.of_obj_flight.find((row) => row.flight_id === 'F-1001');
  assert.equal(changed.of_version - (original?.of_version ?? 0n), 2n);
  assert.equal(changed.delay_minutes, 23n);
  assert.equal(changed.status, 'DELAYED');
  assert.deepEqual(await client.action('delayFlight', request), receipt);
  assert.deepEqual(await client.commandResult(request.commandId), receipt);
  assert.deepEqual(durableState(database), committed, 'replay must preserve all persisted business and transaction rows');
});
