import assert from 'node:assert/strict';
import { createServer, request as httpRequest } from 'node:http';
import { DatabaseSync } from 'node:sqlite';

export function durableState(path) {
  const database = new DatabaseSync(path, { readOnly: true });
  try {
    database.exec('PRAGMA busy_timeout = 5000');
    const names = database.prepare(`SELECT name FROM sqlite_master WHERE type = 'table' AND (
      name GLOB 'of_obj_*' OR name GLOB 'of_lnk_*' OR name IN (
        'of_edit_log', 'of_edit_log_entry', 'of_edit_history', 'of_action_command', 'of_webhook_outbox'
      )) ORDER BY name`).all();
    return Object.fromEntries(names.map(({ name }) => {
      const statement = database.prepare(`SELECT * FROM "${name}" ORDER BY rowid`);
      statement.setReadBigInts(true);
      return [name, statement.all().map((row) => ({ ...row }))];
    }));
  } finally { database.close(); }
}

export function revokeDispatchers(path) {
  const database = new DatabaseSync(path);
  database.exec('PRAGMA busy_timeout = 5000');
  const rows = database.prepare("SELECT role, subject_kind, subject FROM of_role_binding WHERE role = 'dispatcher'").all();
  assert.ok(rows.length > 0, 'a valid dispatcher binding must exist before revocation');
  database.prepare("DELETE FROM of_role_binding WHERE role = 'dispatcher'").run();
  database.close();
  return () => {
    const restore = new DatabaseSync(path);
    try {
      restore.exec('PRAGMA busy_timeout = 5000');
      for (const row of rows) restore.prepare('INSERT INTO of_role_binding (role, subject_kind, subject) VALUES (?, ?, ?)')
        .run(row.role, row.subject_kind, row.subject);
    } finally { restore.close(); }
  };
}

export async function faultProxy(baseUrl, mode) {
  let requests = 0;
  let forwards = 0;
  let receipt;
  let completed = 0;
  const reached = Promise.withResolvers();
  const release = Promise.withResolvers();
  const failures = [];
  const server = createServer(async (incoming, outgoing) => {
    try {
      requests += 1;
      const ordinal = requests;
      const chunks = [];
      for await (const chunk of incoming) chunks.push(chunk);
      if (mode === 'before') { outgoing.destroy(); return; }
      forwards += 1;
      const upstream = await new Promise((resolve, reject) => {
        const request = httpRequest(new URL(incoming.url, baseUrl), {
          method: incoming.method,
          headers: { ...incoming.headers, host: new URL(baseUrl).host, connection: 'close' },
        }, resolve);
        request.on('error', reject);
        request.setTimeout(5000, () => request.destroy(new Error('proxy upstream timed out')));
        request.end(Buffer.concat(chunks));
      });
      const body = [];
      for await (const chunk of upstream) body.push(chunk);
      const text = Buffer.concat(body).toString();
      if (mode === 'after') {
        assert.equal(upstream.statusCode, 200, text);
        receipt = text;
        outgoing.destroy();
        return;
      }
      if (mode === 'hold-both' && ordinal <= 2) {
        completed += 1;
        if (completed === 2) reached.resolve();
        await release.promise;
      }
      outgoing.writeHead(upstream.statusCode, { 'Content-Type': 'application/json', Connection: 'close' });
      outgoing.end(text);
    } catch (error) { failures.push(error); outgoing.destroy(); reached.reject(error); }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return {
    url: `http://127.0.0.1:${server.address().port}`,
    reached: reached.promise,
    release: () => release.resolve(),
    counts: () => ({ requests, forwards }),
    receipt: () => receipt,
    async close() {
      release.resolve();
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
      assert.deepEqual(failures, []);
    },
  };
}
