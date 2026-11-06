const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = process.cwd();
const runtime = 'C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs';
const collection = JSON.parse(fs.readFileSync(path.join(__dirname, 'collection.json'), 'utf8'));
const evidence = path.relative(root, __dirname).replaceAll('\\', '/');
const common = ['contracts.json', 'source-review.json', 'test-source-review.json', 'review.md', 'boundaries.json', 'governance-inputs.json', 'log-manifest.json', 'collection.json', 'collect.cjs', 'fingerprint.cjs'];
const logs = JSON.parse(fs.readFileSync(path.join(__dirname, 'log-manifest.json'), 'utf8')).map(row => path.posix.dirname(evidence) + '/' + row.file);
const governance = JSON.parse(fs.readFileSync(path.join(__dirname, 'governance-inputs.json'), 'utf8')).filter(row => !row.error).map(row => row.file);
const results = [];
for (const phase of collection.phases) {
  const number = phase.phase.slice(0, 2);
  const files = [...new Set([...phase.covered_before, ...common.map(file => evidence + '/' + file), evidence + '/' + number + '-VERIFICATION.before.md', evidence + '/' + number + '-covered-source.diff', ...logs, ...governance])];
  const args = ['query', 'verification.fingerprint', phase.dir, ...files];
  const run = spawnSync(process.execPath, [runtime, ...args], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(run.stderr + run.stdout);
  const fingerprint = JSON.parse(run.stdout);
  fs.writeFileSync(path.join(__dirname, number + '-fingerprint.json'), JSON.stringify({ args, exit: run.status, stderr: run.stderr, ...fingerprint }, null, 2) + '\n');
  const removed = phase.covered_before.filter(file => !fingerprint.covered_files.includes(file));
  const added = fingerprint.covered_files.filter(file => !phase.covered_before.includes(file));
  const diff = { phase: phase.phase, old_count: phase.covered_before.length, new_count: fingerprint.covered_files.length, original_is_subset: removed.length === 0, removed, added, governance_removed: phase.covered_before.filter(file => file.startsWith('.planning/') && !fingerprint.covered_files.includes(file)), covered_digest: fingerprint.covered_digest };
  fs.writeFileSync(path.join(__dirname, number + '-covered-diff.json'), JSON.stringify(diff, null, 2) + '\n');
  if (removed.length) throw new Error('Original covered input removed: ' + removed.join(', '));
  results.push(diff);
}
console.log(JSON.stringify(results, null, 2));
