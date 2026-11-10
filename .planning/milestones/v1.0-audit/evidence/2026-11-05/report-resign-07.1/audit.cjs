const fs = require('node:fs');
const crypto = require('node:crypto');
const child = require('node:child_process');
const phase = '.planning/phases/07.1-source-cardinality-gap-closure';
const evidence = '.planning/milestones/v1.0-audit/evidence/2026-11-05';
const dest = `${evidence}/report-resign-07.1`;
const report = `${phase}/07.1-VERIFICATION.md`;
const tools = 'C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs';
const patcher = 'C:/Users/easyi/.codex/packages/app-server-daemon/releases/0.166.0-x86_64-pc-windows-msvc/bin/codex.exe';
const read = file => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
const sha = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const covered = text => JSON.parse(text.match(/covered_files:\s*(\[[\s\S]*?\])/)[1]);
const run = (exe, args) => {
  const result = child.spawnSync(exe, args, { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
  if (result.error) throw result.error;
  return { command: [exe, ...args], exit_code: result.status, stdout: result.stdout, stderr: result.stderr };
};
const patch = text => {
  const result = run(patcher, ['--codex-run-as-apply-patch', text]);
  if (result.exit_code !== 0) throw new Error(JSON.stringify(result));
};
const add = (file, text) => {
  if (fs.existsSync(file)) throw new Error(`Refusing to overwrite ${file}`);
  const lines = text.replace(/\r\n/g, '\n').replace(/\n$/, '').split('\n');
  for (let offset = 0; offset < lines.length; offset += 25) {
    const batch = lines.slice(offset, offset + 25);
    const operation = offset === 0
      ? `*** Add File: ${file}\n${batch.map(line => `+${line}`).join('\n')}`
      : `*** Update File: ${file}\n@@\n ${lines[offset - 1]}\n${batch.map(line => `+${line}`).join('\n')}\n*** End of File`;
    patch(`*** Begin Patch\n${operation}\n*** End Patch`);
  }
  if (read(file) !== `${lines.join('\n')}\n`) throw new Error(`Snapshot mismatch ${file}`);
};
const addJson = (file, value) => add(file, `${JSON.stringify(value, null, 2)}\n`);
const query = args => run(process.execPath, [tools, 'query', ...args]);
const mode = process.argv[2];
if (mode === 'capture') {
  const original = read(report);
  const oldFiles = covered(original);
  if (oldFiles.length !== 256) throw new Error(`Expected 256, got ${oldFiles.length}`);
  const head = run('git', ['rev-parse', 'HEAD']).stdout.trim();
  if (head !== '060019fe52005c6b85ae5c5f59da9868f464e579') throw new Error(`Unexpected HEAD ${head}`);
  add(`${dest}/previous-VERIFICATION.md`, original);
  addJson(`${dest}/before.json`, {
    recorded_at: new Date().toISOString(), head, report_sha256: sha(report), covered_files: oldFiles,
    file_hashes: oldFiles.map(file => ({ file, sha256: sha(file) })),
    native_status: query(['verification.status', phase]), git_status: run('git', ['status', '--short']),
    snapshot_sha256: sha(`${dest}/previous-VERIFICATION.md`),
  });
  console.log(`CAPTURE_OK covered=${oldFiles.length}; apply_patch snapshot saved`);
}
if (mode === 'inspect') {
  const old = JSON.parse(read(`${dest}/before.json`));
  const gates = [];
  for (const number of ['01', '02', '03']) {
    const plan = `${phase}/07.1-${number}-PLAN.md`;
    for (const verb of ['verify.artifacts', 'verify.key-links']) gates.push(query([verb, plan]));
  }
  gates.push(query(['roadmap.get-phase', '07.1', '--raw']));
  gates.push(query(['roadmap.analyze', '--raw']));
  addJson(`${dest}/native-gates.json`, gates);
  const governance = fs.readdirSync(phase).filter(file => file.endsWith('.md') && file !== '07.1-VERIFICATION.md').map(file => `${phase}/${file}`);
  const readInputs = [...new Set([...old.covered_files, ...governance,
    '.planning/REQUIREMENTS.md', '.planning/ROADMAP.md', '.planning/config.json',
    '.planning/milestones/v1.0-audit/task_plan.md', `${evidence}/report-resign-baseline-07.1.json`,
    `${evidence}/reverify-workspace.log`, `${evidence}/reverify-local-results.json`,
    `${evidence}/ci-41994479295-final.json`, `${evidence}/ci-41994479295-integration.log`,
    `${evidence}/ci-41994479295-ubuntu.log`, `${evidence}/ci-41994479295-windows.log`])];
  const sourceFiles = old.covered_files.filter(file => !file.startsWith('.planning/'));
  add(`${dest}/source-since-previous.diff`, run('git', ['diff', '01aa2dfd29262105c52cecbaecd142404215bee4', 'HEAD', '--', ...sourceFiles]).stdout || '(no changes)\n');
  const worktree = run('git', ['diff', 'HEAD', '--', ...sourceFiles]);
  if (worktree.stdout.trim()) throw new Error('Covered implementation has uncommitted changes');
  const observations = [];
  const observe = (file, pattern) => {
    const matches = read(file).split(/\r?\n/).flatMap((line, index) => pattern.test(line) ? [{ line: index + 1, text: line }] : []);
    observations.push({ file, pattern: pattern.source, matches });
    return matches;
  };
  for (const file of readInputs) fs.readFileSync(file);
  const workspace = `${evidence}/reverify-workspace.log`;
  const integration = `${evidence}/ci-41994479295-integration.log`;
  const engineTests = [...read('crates/ontoforge-engine/tests/cardinality_check.rs').matchAll(/#\[tokio::test[^\]]*\]\s*async fn (\w+)/g)].map(match => match[1]);
  const cliTests = [...read('crates/ontoforge/tests/cli_check_cardinality.rs').matchAll(/#\[(?:tokio::)?test[^\]]*\]\s*(?:async )?fn (\w+)/g)].map(match => match[1]);
  const unitTests = ['online::tests::cardinality_total_deadline_is_shared', 'online::tests::cardinality_total_deadline_configuration', 'online::tests::cardinality_cleanup_failure_after_success', 'online::tests::cardinality_cleanup_failure_after_metadata_error', 'printer::tests::count_distinct_quotes_identifiers', 'doc_diagnostic_codes_exist', 'missing_env_panics', 'ci_configs_use_dynamic_urls_and_isolated_unchanged_templates', 'ci_config_rejects_toml_string_injection'];
  for (const name of [...engineTests, ...cliTests, ...unitTests]) {
    if (!observe(workspace, new RegExp(`^test ${name} \\.\\.\\. ok$`)).length) throw new Error(`No passing execution for ${name}`);
  }
  for (const name of ['check_cardinality_counts_distinct_objects_on_ci_trino', 'check_cardinality_aliases_on_ci_trino']) {
    if (!observe(integration, new RegExp(`test ${name} \\.\\.\\. ok$`)).length) throw new Error(`Missing real Trino target ${name}`);
  }
  observe(integration, /Trino|PostgreSQL|postgres|cardinality_records|HEAD is now|060019f|cli_trino_it|one_to_one_violation_is_reported|concurrent_fk_claim/);
  for (const file of sourceFiles.filter(file => /\.(rs|sql|yml|md|toml)$/.test(file))) observe(file, /\b(TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER)\b/);
  const ci = JSON.parse(read(`${evidence}/ci-41994479295-final.json`));
  if (ci.headSha !== old.head || ci.conclusion !== 'success') throw new Error('CI identity mismatch');
  const totals = [...read(workspace).matchAll(/test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)]
    .reduce((sum, match) => ({ harnesses: sum.harnesses + 1, passed: sum.passed + Number(match[1]), failed: sum.failed + Number(match[2]), ignored: sum.ignored + Number(match[3]) }), { harnesses: 0, passed: 0, failed: 0, ignored: 0 });
  addJson(`${dest}/source-log-audit.json`, {
    recorded_at: new Date().toISOString(), head: old.head,
    inputs: readInputs.map(file => ({ file, bytes: fs.statSync(file).size, sha256: sha(file) })),
    ci_identity: { head: ci.headSha, conclusion: ci.conclusion, url: ci.url },
    source_files: sourceFiles, engine_tests: engineTests, cli_tests: cliTests, unit_tests: unitTests,
    workspace_totals: totals, observations, implementation_worktree_diff: worktree.stdout,
    extra_test_execution: false, container_operations: false,
  });
  console.log(`AUDIT_OK engine=${engineTests.length}, CLI=${cliTests.length}, unit/doc/config=${unitTests.length}; ${JSON.stringify(totals)}`);
  console.log(gates.map(gate => `${gate.command.slice(3, 5).join(' ')} => ${gate.exit_code}`).join('\n'));
}
if (mode === 'resign') {
  const original = read(report);
  const marker = '# Phase 07.1: Source Cardinality Gap Closure Verification Report';
  const contract = read(`${dest}/contract-map.md`).trimEnd();
  if (original.includes('# Phase 07.1 当前合同重新验收')) throw new Error('Already re-signed');
  patch(`*** Begin Patch\n*** Update File: ${report}\n@@\n-${marker}\n${contract.split('\n').map(line => `+${line}`).join('\n')}\n+\n+# 2026-11-04 历史报告正文（非本轮权威判定）\n+\n+> 以下保留旧签署正文的原始时态、测试数与证据归属；当前结论、源码和执行映射以本报告前面的2026-11-05重新验收节为准。\n*** End Patch`);
  const timestamp = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
  patch(`*** Begin Patch\n*** Update File: ${report}\n@@\n-verified: 2026-11-04T09:04:50Z\n+verified: ${timestamp}\n@@\n-verified_source_commit: 01aa2dfd29262105c52cecbaecd142404215bee4\n-observed_head: 0884afa0e22a6df0cecb02efadf275c22a7b1815\n+verified_source_commit: 060019fe52005c6b85ae5c5f59da9868f464e579\n+observed_head: 060019fe52005c6b85ae5c5f59da9868f464e579\n+re_verification:\n+  previous_status: passed\n+  previous_score: 18/18\n+  previous_native_status: stale\n+  reason: Full contract re-verification after cli_trino_it.rs changed\n+  gaps_closed: []\n+  gaps_remaining: []\n+  regressions: []\n*** End Patch`);
  console.log(`RESIGN_WRITTEN ${timestamp}; full contract mapped`);
}
if (mode === 'fingerprint') {
  const old = JSON.parse(read(`${dest}/before.json`));
  const additions = [...fs.readdirSync(dest).filter(file => !['fingerprint.json', 'covered-diff.json', 'after-gate.json'].includes(file)).map(file => `${dest}/${file}`),
    ...['reverify-workspace.log', 'reverify-local-results.json', 'ci-41994479295-final.json', 'ci-41994479295-integration.log', 'ci-41994479295-ubuntu.log', 'ci-41994479295-windows.log', 'report-resign-baseline-07.1.json'].map(file => `${evidence}/${file}`)];
  const result = query(['verification.fingerprint', phase, ...old.covered_files, ...additions]);
  if (result.exit_code !== 0) throw new Error(JSON.stringify(result));
  const fingerprint = JSON.parse(result.stdout);
  const removed = old.covered_files.filter(file => !fingerprint.covered_files.includes(file));
  if (removed.length) throw new Error(`Coverage dropped: ${JSON.stringify(removed)}`);
  addJson(`${dest}/fingerprint.json`, fingerprint);
  addJson(`${dest}/covered-diff.json`, {
    before: old.covered_files.length, after: fingerprint.covered_files.length, removed,
    added: fingerprint.covered_files.filter(file => !old.covered_files.includes(file)),
    preserved_governance: old.covered_files.filter(file => file.startsWith('.planning/')),
    excluded_derived_receipts: ['fingerprint.json', 'covered-diff.json', 'after-gate.json'],
    reason: 'Derived receipts excluded to avoid fingerprint self-reference; no old coverage removed.',
    command_receipt: { command: result.command, exit_code: result.exit_code, stderr: result.stderr },
  });
  console.log(`FINGERPRINT_OK ${old.covered_files.length} -> ${fingerprint.covered_files.length}, removed=0`);
}
if (mode === 'finish') {
  const fingerprint = JSON.parse(read(`${dest}/fingerprint.json`));
  let current = covered(read(report));
  for (let index = 0; index < fingerprint.covered_files.length; index++) {
    const file = fingerprint.covered_files[index];
    if (current.includes(file)) continue;
    const next = fingerprint.covered_files.slice(index + 1).find(candidate => current.includes(candidate));
    if (!next) throw new Error(`No insertion anchor for ${file}`);
    const line = read(report).split('\n').find(candidate => candidate.trim() === `"${next}",`);
    if (!line) throw new Error(`No canonical anchor for ${file}`);
    patch(`*** Begin Patch\n*** Update File: ${report}\n@@\n+    ${JSON.stringify(file)},\n ${line}\n*** End Patch`);
    current = covered(read(report));
  }
  if (JSON.stringify(current) !== JSON.stringify(fingerprint.covered_files)) throw new Error('Covered array differs from native result');
  const oldDigest = read(report).match(/covered_digest: "[^"]+"/)[0];
  patch(`*** Begin Patch\n*** Update File: ${report}\n@@\n-${oldDigest}\n+covered_digest: "${fingerprint.covered_digest}"\n*** End Patch`);
  const status = query(['verification.status', phase]);
  const picked = query(['verification.status', phase, '--pick', 'status']);
  if (status.exit_code !== 0 || picked.exit_code !== 0 || picked.stdout.trim() !== 'passed') throw new Error(JSON.stringify({ status, picked }));
  const old = JSON.parse(read(`${dest}/before.json`));
  const drift = old.file_hashes.filter(entry => sha(entry.file) !== entry.sha256);
  if (drift.length) throw new Error(`Concurrent inputs changed: ${JSON.stringify(drift)}`);
  const nextSet = new Set(covered(read(report)));
  if (old.covered_files.some(file => !nextSet.has(file))) throw new Error('Coverage regression');
  addJson(`${dest}/after-gate.json`, {
    recorded_at: new Date().toISOString(), status, picked, coverage_before: old.covered_files.length,
    coverage_after: nextSet.size, removed: [], old_covered_input_drift_during_verification: drift,
    report_sha256: sha(report), head: run('git', ['rev-parse', 'HEAD']).stdout.trim(),
    git_status: run('git', ['status', '--short']),
  });
  console.log(`GATE_OK native=passed, coverage=${old.covered_files.length}->${nextSet.size}, dropped=0, input drift=0`);
}
