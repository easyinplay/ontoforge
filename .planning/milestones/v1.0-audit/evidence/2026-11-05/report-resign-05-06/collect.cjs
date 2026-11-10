const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const root = process.cwd();
const out = __dirname;
const runtime = 'C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs';
const phases = ['05-online-action', '06-l2-raw-dql-toolsets-audit'];
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const read = name => fs.readFileSync(path.join(root, name), 'utf8');
const save = (name, value) => fs.writeFileSync(path.join(out, name), typeof value === 'string' ? value : JSON.stringify(value, null, 2) + '\n');
const run = args => {
  const result = spawnSync(process.execPath, [runtime, ...args], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
  let json;
  try { json = JSON.parse(result.stdout); } catch {}
  return { args, exit: result.status, stdout: result.stdout, stderr: result.stderr, json };
};
const git = args => spawnSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }).stdout;
const report = { captured_at: new Date().toISOString(), head: git(['rev-parse', 'HEAD']).trim(), runtime_identity: run(['runtime-identity', '--raw']), agent_skills: run(['query', 'agent-skills', 'gsd-verifier', '--skills-only']), phases: [] };
for (const phase of phases) {
  const number = phase.slice(0, 2);
  const dir = '.planning/phases/' + phase;
  const original = fs.readFileSync(path.join(root, dir, number + '-VERIFICATION.md'));
  const snapshot = path.join(out, number + '-VERIFICATION.before.md');
  if (!fs.existsSync(snapshot)) fs.writeFileSync(snapshot, original);
  const old = fs.readFileSync(snapshot, 'utf8');
  const covered = JSON.parse(old.match(/^covered_files: (.+)$/m)[1]);
  const plans = fs.readdirSync(path.join(root, dir)).filter(name => /^\d+-\d+-PLAN.md$/.test(name)).sort();
  const inputs = [];
  for (const name of plans) {
    const plan = dir + '/' + name;
    inputs.push({ plan, frontmatter: read(plan).split(/^---\s*$/m)[1], artifacts: run(['query', 'verify.artifacts', plan]), links: run(['query', 'verify.key-links', plan]) });
  }
  const context = fs.readdirSync(path.join(root, dir)).find(name => name.endsWith('-CONTEXT.md'));
  const entry = { phase, dir, original_sha256: sha(Buffer.from(old)), covered_before: covered, native_status_before: run(['query', 'verification.status', dir]), roadmap: run(['query', 'roadmap.get-phase', number, '--raw']), decisions: run(['query', 'check.decision-coverage-verify', dir, dir + '/' + context]), plans: inputs, covered_inputs: covered.map(file => { try { const bytes = fs.readFileSync(path.join(root, file)); return { file, bytes: bytes.length, sha256: sha(bytes) }; } catch (error) { return { file, error: error.code }; } }) };
  report.phases.push(entry);
  save(number + '-covered-source.diff', git(['diff', 'f8851ca073fed34e80e605f0ac91880259df7e12', report.head, '--', ...covered]));
}
report.roadmap_analysis = run(['query', 'roadmap.analyze', '--raw']);
report.original_worktree_status = git(['status', '--short']);
save('collection.json', report);
const names = ['ci-41994479295-final.json', 'ci-41994479295-ubuntu.log', 'ci-41994479295-windows.log', 'ci-41994479295-integration.log', 'ci-41994479295-phase7-real-services.log', 'reverify-workspace.log', 'reverify-sqlite-atomic.log', 'reverify-sqlite-atomic-discovery.log'];
save('log-manifest.json', names.map(file => { const bytes = fs.readFileSync(path.join(out, '..', file)); return { file, bytes: bytes.length, lines: bytes.toString('utf8').split('\n').length, sha256: sha(bytes) }; }));
const hits = [];
for (const name of names.filter(name => name.endsWith('.log'))) {
  const lines = fs.readFileSync(path.join(out, '..', name), 'utf8').split('\n');
  for (let index = 0; index < lines.length; index++) {
    const match = lines[index].match(/test ([\w:]+) \.\.\. (ok|ignored|FAILED)/);
    if (match) hits.push({ log: name, line: index + 1, test: match[1], status: match[2], raw: lines[index] });
  }
}
save('named-test-results.json', hits);
console.log(JSON.stringify({ head: report.head, phases: report.phases.map(p => ({ phase: p.phase, count: p.covered_before.length, before: p.native_status_before.json?.status, decision: p.decisions.json, plans: p.plans.map(p => ({ path: p.plan, artifact_exit: p.artifacts.exit, link_exit: p.links.exit })) })), tests_indexed: hits.length }, null, 2));

