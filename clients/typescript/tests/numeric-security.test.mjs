import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { test } from 'node:test';
import { createOntoforgeClient, decodeWire, encodeWire, LosslessCodecError } from '../dist/index.js';

test('ts_u64_max_codec_and_public_query_preserve_integer_tokens_and_decimal', async () => {
  const maximum = 18446744073709551615n;
  const decimal = '12345678901234567890.123456789012345678901234567890';
  const value = { maximum, decimal };
  const wire = `{"maximum":18446744073709551615,"decimal":"${decimal}"}`;
  assert.equal(encodeWire(value), wire);
  assert.deepEqual(decodeWire(wire), value);
  assert.throws(() => encodeWire({ maximum: Number(maximum) }), LosslessCodecError);

  let requests = 0;
  let received;
  const server = createServer(async (request, response) => {
    requests += 1;
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    received = Buffer.concat(chunks).toString();
    response.writeHead(200, { 'Content-Type': 'application/json', Connection: 'close' });
    response.end(encodeWire({
      rows: [{ pk: 'boundary', version: 'v1', properties: value }],
      hasMore: false, truncated: false,
    }));
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  try {
    const api = createOntoforgeClient({ baseUrl: `http://127.0.0.1:${server.address().port}` });
    const body = { set: { kind: 'pk', object_type: 'Boundary', pk: value }, page: { first: 1n } };
    const result = await api.query(body);
    assert.equal(requests, 1);
    assert.match(received, /"maximum":18446744073709551615[,}]/);
    assert.deepEqual(decodeWire(received).set.pk, value);
    assert.deepEqual(result.rows[0].properties, value);
    await assert.rejects(() => api.query({ ...body, set: { ...body.set, pk: Number(maximum) } }), LosslessCodecError);
    assert.equal(requests, 1, 'unsafe number must be rejected before transport');
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
