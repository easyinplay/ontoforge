const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const crypto = require('node:crypto');
const output = __dirname;
const base = path.dirname(output);
const root = process.cwd();
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
function save(name, value) { fs.writeFileSync(path.join(output, name), JSON.stringify(value, null, 2) + '\n'); }
function run(command, args) { const result = cp.spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 }); return { command, args, exit: result.status, stdout: result.stdout, stderr: result.stderr }; }
const contracts = ['03', '04'].map(phase => JSON.parse(fs.readFileSync(path.join(output, phase + '-contracts.json'), 'utf8')));
const logs = Object.fromEntries(['ubuntu', 'windows', 'integration', 'phase7-real-services', 'isolated-query-bench'].map(key => [key, fs.readFileSync(path.join(base, 'ci-41994479295-' + key + '.log'), 'utf8').split('\n')]));
const tests = [];
for (const [job, lines] of Object.entries(logs)) {
  let target = '';
  lines.forEach((line, index) => {
    const clean = line.replace(/\x1b\[[0-9;]*m/g, '');
    if (/Running .*\(.+\)/.test(clean)) target = clean;
    const match = line.match(/\btest (\S+) \.\.\. ok\s*$/);
    if (match) tests.push({ job, line: index + 1, name: match[1], target, output: line });
  });
}
save('executed-tests.json', tests);
const covered = [...new Set(contracts.flatMap(item => item.old.data.covered_files))];
const files = covered.map(file => {
  if (!fs.existsSync(file)) return { file, exists: false };
  const bytes = fs.readFileSync(file);
  const lines = bytes.toString('utf8').split('\n');
  const markers = lines.map((text, index) => ({ line: index + 1, text })).filter(item => /\b(TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER)\b/.test(item.text));
  const functions = file.endsWith('.rs') ? lines.map((text, index) => ({ line: index + 1, text })).filter(item => /\b(?:async )?fn \w+/.test(item.text)).map(item => {
    const name = item.text.match(/fn (\w+)/)[1];
    return { ...item, name, executions: tests.filter(test => test.name === name || test.name.endsWith('::' + name)) };
  }) : undefined;
  return { file, exists: true, bytes: bytes.length, sha256: hash(bytes), markers, functions };
});
save('covered-file-inspection.json', files);
const moved = [...new Set(contracts.flatMap(item => item.before.data.moved_files))];
save('changed-source-inspection.json', { head: run('git', ['rev-parse', 'HEAD']), workingSource: run('git', ['diff', '--name-only', 'HEAD', '--', 'crates', 'Cargo.toml', 'Cargo.lock', 'ci', '.github', 'examples', 'schemas']), moved, diffFromPriorHead: run('git', ['diff', 'f8851ca', 'HEAD', '--', ...moved]), phase03ManifestHistory: run('git', ['log', '--format=%h %s', 'e6d4823..1df4a7f', '--', 'Cargo.toml', 'Cargo.lock', 'crates/ontoforge-storage/Cargo.toml', 'crates/ontoforge-auth/Cargo.toml', 'crates/ontoforge-expr/Cargo.toml', 'crates/ontoforge/Cargo.toml']), baselineHeadSourceDiff: run('git', ['diff', '--name-only', 'f8851ca', 'HEAD', '--', 'crates/ontoforge-storage', 'crates/ontoforge-auth', 'crates/ontoforge-expr', 'crates/ontoforge-query']) });
const governance = [...new Set([...covered.filter(file => file.startsWith('.planning/') || file.startsWith('docs/')), '.planning/REQUIREMENTS.md', '.planning/ROADMAP.md', '.planning/milestones/v1.0-audit/task_plan.md', '.planning/milestones/v1.0-audit/INTEGRATION.md', '.planning/milestones/v1.0-audit/REQUIREMENTS-AUDIT.md', '.planning/config.json'])];
save('governance-inputs.json', governance.map(file => { const bytes = fs.readFileSync(file); return { file, sha256: hash(bytes), bytes: bytes.length, headings: bytes.toString('utf8').split('\n').map((text,index)=>({line:index+1,text})).filter(item=>/^#{1,4} /.test(item.text)) }; }));
save('service-environment-evidence.json', Object.fromEntries(Object.entries(logs).filter(([job]) => ['integration', 'phase7-real-services', 'isolated-query-bench'].includes(job)).map(([job, lines]) => [job, lines.map((text, index) => ({ line: index + 1, text })).filter(item => /ONTOFORGE_(?:TRINO|PG|SERVER|BENCH)[A-Z_]*:|Run .*query-services|Run .*server-services|ignored tests|p95=|Artifact .*finalized|phase4_real_backend_p95|\btest (?:trino_matches|cached_column|fixed_semantics|aggregate_corpus|where_search|snapshot_pg|pg_http_mixed|trino_http_long)/.test(item.text))])));
console.log(JSON.stringify({ tests: tests.length, covered: files.length, missing: files.filter(file => !file.exists).map(file => file.file), moved: moved.length, productionMarkers: files.filter(file => file.file.includes('/src/') && file.markers.length).map(file => ({ file: file.file, markers: file.markers })) }, null, 2));
