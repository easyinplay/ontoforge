const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const root = process.cwd();
const out = __dirname;
const runtime = 'C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs';
const load = name => JSON.parse(fs.readFileSync(path.join(out, name), 'utf8'));
const sha = data => crypto.createHash('sha256').update(data).digest('hex');
const errors = [];
const check = (condition, message) => { if (!condition) errors.push(message); };
const invoke = args => {
  const result = spawnSync(process.execPath, [runtime, ...args], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
  let json;
  try { json = JSON.parse(result.stdout); } catch {}
  return { args, exit: result.status, stdout: result.stdout, stderr: result.stderr, json };
};
const collection = load('collection.json');
const contracts = load('contracts.json');
const sources = load('source-review.json');
const manifest = load('log-manifest.json');
const head = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).stdout.trim();
check(head === collection.head, 'HEAD changed during verification');
const ci = JSON.parse(fs.readFileSync(path.join(out, '..', 'ci-41994479295-final.json'), 'utf8'));
check(ci.headSha === head && ci.status === 'completed' && ci.conclusion === 'success', 'CI provenance mismatch');
check(ci.jobs.length === 5 && ci.jobs.every(job => job.conclusion === 'success'), 'Expected five completed successful jobs');
for (const item of manifest) {
  check(sha(fs.readFileSync(path.join(out, '..', item.file))) === item.sha256, 'Log changed: ' + item.file);
}
for (const item of sources.files) {
  check(sha(fs.readFileSync(path.join(root, item.file))) === item.sha256, 'Reviewed source changed: ' + item.file);
}
for (const contract of contracts) {
  if (!contract.behavior_dependent) continue;
  check(contract.tests.length > 0, 'No behavioral evidence: ' + contract.phase + '-' + contract.id);
  for (const test of contract.tests) {
    check(test.definitions.length > 0 && test.executions.length > 0, 'Missing source/execution: ' + test.name);
    for (const hit of test.executions) {
      const line = fs.readFileSync(path.join(out, '..', hit.log), 'utf8').split('\n')[hit.line - 1];
      const pattern = new RegExp('test (?:[\\w]+::)*' + test.name + ' \\.\\.\\. ok(?:\\r)?$');
      check(pattern.test(line), 'Not an actual named ok: ' + test.name + ' at ' + hit.log + ':' + hit.line);
    }
  }
}
const results = [];
for (const before of collection.phases) {
  const number = before.phase.slice(0, 2);
  const reportPath = before.dir + '/' + number + '-VERIFICATION.md';
  const text = fs.readFileSync(path.join(root, reportPath), 'utf8');
  const covered = JSON.parse(text.match(/^covered_files: (.+)$/m)[1]);
  const digest = text.match(/^covered_digest: "(.+)"$/m)[1];
  const expected = load(number + '-fingerprint.json');
  const snapshot = fs.readFileSync(path.join(out, number + '-VERIFICATION.before.md'));
  check(sha(snapshot) === before.original_sha256, 'Original snapshot mismatch: ' + number);
  const baseline = JSON.parse(fs.readFileSync(path.join(out, '..', 'report-resign-baseline-' + number + '.json'), 'utf8'));
  check(JSON.stringify([...baseline.covered_files].sort()) === JSON.stringify([...before.covered_before].sort()), 'Orchestrator baseline mismatch: ' + number);
  const removed = before.covered_before.filter(file => !covered.includes(file));
  check(removed.length === 0, 'Original coverage removed: ' + removed.join(','));
  check(JSON.stringify(covered) === JSON.stringify(expected.covered_files) && digest === expected.covered_digest, 'Fingerprint receipt not copied verbatim: ' + number);
  const rehash = invoke(['query', 'verification.fingerprint', before.dir, ...covered]);
  check(rehash.exit === 0 && rehash.json?.covered_digest === digest, 'Fingerprint no longer current: ' + number);
  const status = invoke(['query', 'verification.status', before.dir]);
  check(status.exit === 0 && status.json?.status === 'passed', 'Native status did not pass: ' + number);
  const currentTruths = text.split('\n').filter(line => /^\| \d+ \|/.test(line)).map(line => line.split('|')[2].trim());
  const originalTruths = snapshot.toString('utf8').split('\n').filter(line => /^\| \d+ \|/.test(line)).map(line => line.split('|')[2].trim());
  check(JSON.stringify(currentTruths) === JSON.stringify(originalTruths), 'Original truth scope changed: ' + number);
  check(/^status: passed$/m.test(text) && /^behavior_unverified: 0$/m.test(text), 'Frontmatter decision mismatch: ' + number);
  check(/deferred，未验证/.test(text) && /BLOCKED/.test(text) && /时间敏感人工治理观察/.test(text), 'Release/governance boundary lost: ' + number);
  const story = before.roadmap.stdout.match(/\*\*Goal\*\*:\s*(.+)/)?.[1];
  const storyValidation = invoke(['query', 'user-story.validate', '--story', story]);
  check(storyValidation.exit === 0 && storyValidation.json?.valid === true, 'MVP user story invalid: ' + number);
  results.push({ phase: number, report: reportPath, truths: currentTruths.length, original_covered: before.covered_before.length, current_covered: covered.length, removed, governance_removed: removed.filter(file => file.startsWith('.planning/')), native_status: status, fingerprint_recheck: rehash, user_story: storyValidation, snapshot_sha256: sha(snapshot) });
}
const receipt = { at: new Date().toISOString(), head, verdict: errors.length ? 'GAPS_FOUND' : 'COMPLETE', errors, contracts: contracts.length, source_files_checked: sources.files.length, logs_checked: manifest.length, new_cargo_or_service_runs: 0, results };
fs.writeFileSync(path.join(out, 'final-gate.json'), JSON.stringify(receipt, null, 2) + '\n');
console.log(JSON.stringify({ verdict: receipt.verdict, errors, results: results.map(row => ({ phase: row.phase, truths: row.truths, original_covered: row.original_covered, current_covered: row.current_covered, removed: row.removed, status: row.native_status.json?.status })) }, null, 2));
process.exitCode = errors.length ? 1 : 0;
