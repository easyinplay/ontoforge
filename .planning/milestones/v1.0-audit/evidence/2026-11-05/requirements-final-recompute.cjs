'use strict';
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const assert = require('node:assert/strict');
const gsd = process.env.GSD_CORE_DIR || 'C:/Users/easyi/.codex/gsd-core';
const parser = gsd + '/bin/lib/frontmatter.cjs';
const { extractFrontmatter, FRONTMATTER_UNPARSEABLE } = require(parser);
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const read = file => fs.readFileSync(file, 'utf8');
const sha = file => hash(fs.readFileSync(file));
const commands = [];
function run(program, args) {
  const result = spawnSync(program, args, { encoding: 'utf8', timeout: 10000, windowsHide: true });
  const record = { command: [program, ...args], exit_code: result.status, stdout: result.stdout || '', stderr: result.stderr || '' };
  commands.push(record);
  assert.equal(result.status, 0, JSON.stringify(record));
  return result.stdout.trim();
}
const head = run('git', ['rev-parse', 'HEAD']);
const sourceDiff = run('git', ['diff', '--name-only', '8d9d8c6', head, '--', 'crates', 'ci', 'clients', '.github', 'Cargo.toml', 'Cargo.lock']);
assert.equal(sourceDiff, '', 'Declared implementation baseline differs');
assert.equal(run('git', ['diff', '--name-only', '--', 'crates', 'ci', 'clients', '.github', 'Cargo.toml', 'Cargo.lock']), '', 'Uncommitted implementation changed after dynamic validation');
const requirementLines = read('.planning/REQUIREMENTS.md').split(/\r?\n/);
const definitions = new Map();
requirementLines.forEach((line, index) => {
  const match = line.match(/^- \[([ xX])\] \*\*([A-Z][A-Z0-9]*-\d{2})\*\*:\s*(.*)$/);
  if (match) definitions.set(match[2], { checked: match[1].toLowerCase() === 'x', description: match[3], definition_line: index + 1 });
});
const requirements = [];
requirementLines.forEach((line, index) => {
  const match = line.match(/^\|\s*([A-Z][A-Z0-9]*-\d{2})\s*\|\s*(.*?)\s*\|\s*(.*?)\s*\|/);
  if (!match) return;
  assert(definitions.has(match[1]));
  const phases = [...match[2].matchAll(/Phase\s*(\d+(?:\.\d+)?)/g)].map(value => Number(value[1]).toString());
  if (/\/\s*07\.2/.test(match[2])) phases.push('7.2');
  requirements.push({ id: match[1], ...definitions.get(match[1]), assigned: match[2], phases: [...new Set(phases)], traceability_status: match[3], traceability_line: index + 1 });
});
assert.equal(requirements.length, 79);
const ids = new Set(requirements.map(item => item.id));
assert.equal(ids.size, 79);
const documents = [];
for (const directory of fs.readdirSync('.planning/phases', { withFileTypes: true }).filter(item => item.isDirectory()).sort((left, right) => left.name.localeCompare(right.name))) {
  const phase = directory.name.split('-')[0];
  for (const filename of fs.readdirSync('.planning/phases/' + directory.name).sort()) {
    const kind = filename.endsWith('-SUMMARY.md') ? 'SUMMARY' : filename.endsWith('-PLAN.md') ? 'PLAN' : filename === phase + '-VERIFICATION.md' ? 'VERIFICATION' : null;
    if (!kind) continue;
    const file = '.planning/phases/' + directory.name + '/' + filename;
    const text = read(file);
    const frontmatter = extractFrontmatter(text, file);
    assert.notEqual(frontmatter[FRONTMATTER_UNPARSEABLE], true, file);
    documents.push({ path: file, phase: Number(phase).toString(), kind, text, frontmatter, sha256: sha(file) });
  }
}
const summaries = documents.filter(item => item.kind === 'SUMMARY');
const plans = documents.filter(item => item.kind === 'PLAN');
const verifications = documents.filter(item => item.kind === 'VERIFICATION');
assert.equal(summaries.length, 93);
assert.equal(plans.length, 93);
assert.equal(verifications.length, 9);
const summaryRecords = summaries.map(item => {
  const completed = item.frontmatter['requirements-completed'] ?? item.frontmatter.requirements_completed ?? [];
  assert(Array.isArray(completed), item.path);
  const cli = JSON.parse(run('node', [gsd + '/bin/gsd-tools.cjs', 'query', 'summary-extract', item.path, '--fields', 'requirements_completed']));
  assert.deepEqual(cli.requirements_completed, completed, item.path);
  const missing = !Object.hasOwn(item.frontmatter, 'requirements-completed') && !Object.hasOwn(item.frontmatter, 'requirements_completed');
  return { path: item.path, phase: item.phase, completed, field_missing: missing, explicit_empty: !missing && !completed.length, sha256: item.sha256 };
});
const unknownCompletedIds = [...new Set(summaryRecords.flatMap(item => item.completed).filter(id => !ids.has(id)))];
assert.deepEqual(unknownCompletedIds, [], 'Unknown completed ID must not enter coverage');
function expandIds(text) {
  const found = [];
  for (const match of text.matchAll(/([A-Z][A-Z0-9]*)-(\d{2})(?:(?:\.\.|-|–)(\d{2}))?((?:\/\d{2})*)/g)) {
    const numbers = match[3] ? Array.from({ length: Number(match[3]) - Number(match[2]) + 1 }, (_, offset) => Number(match[2]) + offset) : [Number(match[2]), ...match[4].split('/').filter(Boolean).map(Number)];
    for (const number of numbers) {
      const id = match[1] + '-' + String(number).padStart(2, '0');
      if (ids.has(id)) found.push(id);
    }
  }
  return [...new Set(found)];
}
const verificationRows = [];
const verificationRecords = verifications.map(item => {
  const gate = JSON.parse(run('node', [gsd + '/bin/gsd-tools.cjs', 'query', 'verification.status', path.posix.dirname(item.path), '--raw']));
  let inSection = false;
  let headingLevel = 0;
  let statusIndex = -1;
  item.text.split(/\r?\n/).forEach((line, index) => {
    const heading = line.match(/^(#{1,6})\s+(.*)/);
    if (heading) {
      if (/Requirements Coverage|PLAN \/ Requirements Coverage|Requirements \/ PLAN Coverage/.test(heading[2])) { inSection = true; headingLevel = heading[1].length; statusIndex = -1; }
      else if (heading[1].length <= headingLevel) inSection = false;
      return;
    }
    if (!inSection || !line.startsWith('|')) return;
    const cells = line.split('|').slice(1, -1).map(cell => cell.trim());
    if (cells.some(cell => /^(Status|状态)$/i.test(cell))) { statusIndex = cells.findIndex(cell => /^(Status|状态)$/i.test(cell)); return; }
    const rowIds = expandIds(cells[item.phase === '1' ? 1 : 0] || '');
    const rowStatus = cells[statusIndex] || cells.find(cell => /SATISFIED|PARTIAL|BLOCKED|VERIFIED/.test(cell)) || 'UNCLASSIFIED';
    for (const id of rowIds) verificationRows.push({ id, path: item.path, phase: item.phase, line: index + 1, status: rowStatus, source_row: line, local_pass_claim: /SATISFIED|VERIFIED/.test(rowStatus) && !/PARTIAL|BLOCKED|UNSATISFIED/.test(rowStatus) });
  });
  return { path: item.path, phase: item.phase, raw_status: item.frontmatter.status, score: item.frontmatter.score || null, canonical_status: gate.status, gate, sha256: item.sha256 };
});
const supplementalVerification = [];
const transport = verifications.find(item => item.phase === '7');
const transportLines = transport.text.split(/\r?\n/);
const assignmentLine = transportLines.findIndex(line => line.includes('当前22项含P07 transport L2-03'));
const acceptanceLine = transportLines.findIndex(line => line.startsWith('| INT-01 / R3 / P08 / P16：') && line.includes('| VERIFIED |'));
assert(assignmentLine >= 0 && acceptanceLine >= 0, 'L2-03 explicit transport acceptance missing');
supplementalVerification.push({ id: 'L2-03', phase: '7', path: transport.path, kind: 'explicit_supplemental_requirement_assignment_and_verified_increment', line: assignmentLine + 1, source_text: transportLines[assignmentLine], acceptance_line: acceptanceLine + 1, acceptance_text: transportLines[acceptanceLine], local_pass_claim: true });
for (const item of requirements) {
  item.claimed_by_plans = plans.filter(plan => expandIds(JSON.stringify(plan.frontmatter.requirements || [])).includes(item.id)).map(plan => plan.path);
  item.verification_rows = verificationRows.filter(row => row.id === item.id);
  item.summary_refs = summaryRecords.filter(summary => summary.completed.includes(item.id)).map(summary => summary.path);
  item.summary_completed = item.summary_refs.map(file => path.posix.basename(file).replace('-SUMMARY.md', ''));
  item.orphaned = item.verification_rows.length === 0;
  item.metadata_gap = item.summary_refs.length === 0;
  item.report_gates = verificationRecords.filter(verification => item.phases.includes(verification.phase)).map(({ path: file, phase, raw_status, canonical_status }) => ({ path: file, phase, raw_status, canonical_status }));
  item.historical_row_coverage = item.phases.every(phase => item.verification_rows.some(row => row.phase === phase));
  item.verification_supplements = supplementalVerification.filter(row => row.id === item.id);
  item.structured_verification_table_gap = !item.historical_row_coverage;
  item.historical_local_claim_passed = item.phases.every(phase => [...item.verification_rows, ...item.verification_supplements].some(row => row.phase === phase && row.local_pass_claim));
  if (item.id === 'STORE-05') item.historical_local_claim_passed = item.verification_rows.some(row => row.phase === '7.1' && row.source_row.includes('完整需求') && row.local_pass_claim);
  if (item.id === 'ENGINE-05') item.historical_local_claim_passed = false;
  item.historical_three_source = item.orphaned ? 'unsatisfied' : item.metadata_gap || !item.historical_local_claim_passed || !item.checked ? 'partial' : 'satisfied (historical claims only)';
  item.workflow_three_source_status = item.orphaned || item.report_gates.some(gate => gate.canonical_status === 'gaps_found') ? 'unsatisfied' : item.metadata_gap || !item.historical_local_claim_passed || item.report_gates.length !== item.phases.length || !item.report_gates.every(gate => gate.canonical_status === 'passed') ? 'partial' : 'satisfied';
  item.confirmed_current_functional_failure = false;
}
assert.equal(requirements.filter(item => item.orphaned).length, 0);
assert.equal(requirements.find(item => item.id === 'L2-03').checked, true);
assert.equal(requirements.find(item => item.id === 'STORE-05').checked, true);
assert.equal(requirements.find(item => item.id === 'ENGINE-05').checked, false);
assert.equal(verificationRecords.find(item => item.phase === '1').raw_status, 'passed');
assert.equal(verificationRecords.find(item => item.phase === '2').raw_status, 'gaps_found');
const staticSpecs = [
  ['L2-03', 'crates/ontoforge-engine/src/trino/mod.rs', 'pub fn l2_edit_catalog', '部署ACL配置提供可信edit catalog'],
  ['L2-03', 'crates/ontoforge-server/src/mcp.rs', '.l2_edit_catalog()', 'MCP adapter取可信catalog并注入六工具'],
  ['L2-03', 'crates/ontoforge-server/src/mcp_l2.rs', 'edit_catalog: self.edit_catalog.to_owned()', 'SQL gate使用可信排除'],
  ['L2-03', 'crates/ontoforge-server/src/mcp_l2.rs', '!name.eq_ignore_ascii_case(&policy.edit_catalog)', 'metadata使用相同大小写不敏感排除'],
  ['L2-03', 'crates/ontoforge-server/tests/mcp_l2_defaults.rs', 'mcp_l2_custom_edit_catalog', 'custom catalog测试存在，本轮未执行'],
  ['STORE-05', 'crates/ontoforge/src/cli.rs', 'pub check_cardinality: bool', 'CLI opt-in入口'],
  ['STORE-05', 'crates/ontoforge/src/online.rs', 'args.check_cardinality', 'online接收检查开关'],
  ['STORE-05', 'crates/ontoforge/src/online.rs', 'match check_cardinality(engine, &target, &target_ctx).await', '真实查询结果生成诊断'],
  ['STORE-05', 'crates/ontoforge-engine/src/cardinality_check.rs', 'link.cardinality != Cardinality::OneToOne', 'source OneToOne筛选'],
  ['STORE-05', 'crates/ontoforge-engine/src/cardinality_check.rs', 'Expr::CountDistinct', '按logical PK去重计数，不以PK检查替代FK基数'],
  ['STORE-05', 'crates/ontoforge-engine/src/cardinality_check.rs', 'engine.execute(&query, ctx).await?', 'typed SQL执行真实engine查询'],
  ['STORE-05', 'crates/ontoforge-engine/tests/cardinality_check.rs', 'check_cardinality', '真实SQLite测试存在，本轮未执行'],
  ['ENGINE-05', 'ci/trino/iceberg/iceberg.properties', 's3.endpoint=http://minio:9000', 'MinIO S3 endpoint'],
  ['ENGINE-05', 'ci/trino/iceberg/iceberg.properties', 'jdbc', 'JDBC catalog'],
  ['ENGINE-05', '.github/workflows/nightly.yml', 'matrix.trino', '双版本nightly调用fixture'],
  ['ENGINE-05', 'ci/iceberg-services.sh', 'call(["docker", "build", "--platform", "linux/amd64", "-t", IMAGE, "ci/minio"]', 'fixture实际构建MinIO artifact的调用接线'],
  ['ENGINE-05', 'CONTRIBUTING.md', '私有 fixture（SQL Server', 'SQL Server仍需仓库外fixture验收']
];
const staticChecks = staticSpecs.map(([id, file, needle, meaning]) => {
  const content = read(file);
  const index = content.indexOf(needle);
  assert(index >= 0, 'Static link missing: ' + file + ': ' + needle);
  const line = content.slice(0, index).split('\n').length;
  return { id, path: file, line, needle, meaning, status: 'STATIC_PRESENT_NOT_EXECUTED', snippet: content.split(/\r?\n/).slice(Math.max(0, line - 3), line + 3).join('\n'), sha256: sha(file) };
});
for (const item of requirements) item.targeted_static_evidence = staticChecks.filter(check => check.id === item.id).map(({ path: file, line, meaning }) => ({ path: file, line, meaning }));
const dynamicEvidence = require('./requirements-final-dynamic.cjs')(requirements, verificationRecords);
const count = predicate => requirements.filter(predicate).length;
const counts = { requirements: 79, root_plans: plans.length, root_summaries: summaries.length, formal_verifications: verifications.length, checked: count(item => item.checked), summary_declared_ids: count(item => !item.metadata_gap), metadata_gap_ids: count(item => item.metadata_gap), summary_missing_field_files: summaryRecords.filter(item => item.field_missing).length, summary_explicit_empty_files: summaryRecords.filter(item => item.explicit_empty).length, verification_covered_ids: count(item => !item.orphaned), orphaned: count(item => item.orphaned), historical_three_source_claims: count(item => item.historical_three_source.startsWith('satisfied')), workflow_satisfied: count(item => item.workflow_three_source_status === 'satisfied'), workflow_partial: count(item => item.workflow_three_source_status === 'partial'), workflow_unsatisfied: count(item => item.workflow_three_source_status === 'unsatisfied'), confirmed_current_functional_failures: 0, full_contract_partial: 1 };
Object.assign(counts, dynamicEvidence.requirement_signature_counts);
const manifest = documents.map(({ path: file, kind, sha256 }) => ({ path: file, kind, sha256 }));
for (const file of ['.planning/REQUIREMENTS.md', ...new Set(staticChecks.map(item => item.path))]) manifest.push({ path: file, kind: 'REQUIREMENTS_OR_STATIC_SOURCE', sha256: sha(file) });
for (const source of dynamicEvidence.source_manifest) if (!manifest.some(item => item.path === source.path)) manifest.push({ ...source, kind: 'FINAL_RUNTIME_EVIDENCE' });
for (const file of ['.planning/milestones/v1.0-audit/evidence/2026-11-05/requirements-final-recompute.cjs', '.planning/milestones/v1.0-audit/evidence/2026-11-05/requirements-final-dynamic.cjs']) manifest.push({ path: file, kind: 'RECOMPUTATION_TOOL', sha256: sha(file) });
const matrix = { schema_version: 3, audited_utc: new Date().toISOString(), head, baseline: '8d9d8c6', implementation_diff: sourceDiff, status: 'gaps_found', audit_completion: 'requirements_recomputation_and_scoped_log_review_complete', scope: '79 current requirements; 93 phase-root SUMMARY/PLAN pairs; 9 exact phase VERIFICATION; current raw logs mapped by requirement; reviews excluded', native_parser: { path: parser, sha256: sha(parser), summary_cli_cross_checks: 93, completed_alias_precedence: 'requirements-completed ?? requirements_completed ?? []' }, counts, dynamic_validation: dynamicEvidence, policy: { report_gate_is_not_functional_failure: true, historical_claim_is_not_current_behavior_proof: true, missing_metadata_is_not_missing_implementation: true, source_equality_is_not_dynamic_signature: true, owner_signature_is_phase07_scoped_not_all_requirements: true, ignored_or_fixture_guard_is_not_service_success: true, phase01_raw_status: 'passed', phase02_raw_status: 'gaps_found', platform_totals_must_not_be_added: true }, missing_completed_ids: requirements.filter(item => item.metadata_gap).map(item => item.id), missing_summary_fields: summaryRecords.filter(item => item.field_missing).map(item => item.path), explicit_empty_summary_fields: summaryRecords.filter(item => item.explicit_empty).map(item => item.path), requirements, verifications: verificationRecords, summaries: summaryRecords, static_checks: staticChecks, input_manifest: manifest };
assert(plans.every(plan => summaries.some(summary => summary.path === plan.path.replace('-PLAN.md', '-SUMMARY.md'))), 'Root PLAN without SUMMARY');
if (process.argv.includes('--check')) {
  const saved = JSON.parse(read('.planning/milestones/v1.0-audit/requirements-matrix.json'));
  const comparable = value => { const copy = structuredClone(value); delete copy.audited_utc; return copy; };
  assert.deepEqual(comparable(saved), comparable(matrix), 'Input drift or matrix mismatch: review before refresh');
  console.log(JSON.stringify({ result: 'PASS', mode: 'read-only', head, counts }, null, 2));
} else console.log(JSON.stringify({ matrix, snapshot: { audited_utc: matrix.audited_utc, head, counts, commands, dynamic_validation: dynamicEvidence, input_manifest: manifest, static_checks: staticChecks } }, null, 2));
