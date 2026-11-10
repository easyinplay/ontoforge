'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const { extractFrontmatter } = require((process.env.GSD_CORE_DIR || 'C:/Users/easyi/.codex/gsd-core') + '/bin/lib/frontmatter.cjs');
const prefix = '.planning/milestones/v1.0-audit/evidence/2026-11-05/';
const base = '.planning/milestones/v1.0-audit/';
const expectedHead = '060019fe52005c6b85ae5c5f59da9868f464e579';
const sources = new Map();
function read(file) {
  const bytes = fs.readFileSync(file);
  sources.set(file, { path: file, bytes: bytes.length, sha256: crypto.createHash('sha256').update(bytes).digest('hex') });
  return bytes.toString('utf8').replace(/^\uFEFF/, '');
}
function clean(line) { return line.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '').replace(/^\d{4}-\d\d-\d\dT\S+\s+/, '').trimEnd(); }
function targetIdentity(header) {
  const executable = header.match(/\(([^)]+)\)$/)?.[1];
  return executable ? executable.split(/[\\/]/).at(-1) : header.trim();
}
function targetName(header) { return targetIdentity(header).replace(/-[a-f0-9]+(?:\.exe)?$/, ''); }
function parseRust(text, file) {
  const results = [];
  const pending = [];
  let step = null;
  text.split(/\r?\n/).forEach((raw, offset) => {
    const line = clean(raw);
    if (line.startsWith('##[group]Run ')) { step = { command: line.slice('##[group]Run '.length), line: offset + 1 }; pending.length = 0; }
    if (/^\s*Running (unittests |tests[\\/])/.test(line) || /^\s*Doc-tests /.test(line)) {
      const last = pending.at(-1);
      if (!last || last.header !== line.trim() || last.announced !== null) pending.push({ file, line: offset + 1, header: line.trim(), identity: targetIdentity(line), name: targetName(line), step, announced: null });
    }
    if (/^running \d+ tests?$/.test(line) && pending.length) pending[0].announced = Number(line.match(/\d+/)[0]);
    const match = line.match(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/);
    if (match) {
      const target = pending.shift();
      assert(target, 'Result without target at ' + file + ':' + (offset + 1));
      results.push({ ...target, result_line: offset + 1, timestamp: raw.match(/^(\d{4}-\d\d-\d\dT\S+)/)?.[1] || null, status: match[1], passed: +match[2], failed: +match[3], ignored: +match[4], measured: +match[5], filtered: +match[6] });
    }
  });
  return results;
}
function parseDiscovery(text) {
  const targets = [];
  const pending = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = clean(raw);
    if (/^\s*Running (unittests |tests[\\/])/.test(line) || /^\s*Doc-tests /.test(line)) pending.push({ identity: targetIdentity(line), discovered: null });
    const summary = line.match(/^(\d+) tests?, (\d+) benchmarks?$/);
    if (summary && pending.length) { const target = pending.shift(); target.discovered = +summary[1]; targets.push(target); }
  }
  return targets;
}
function totals(targets) { return targets.reduce((sum, item) => { for (const key of ['passed', 'failed', 'ignored', 'measured', 'filtered']) sum[key] += item[key]; return sum; }, { passed: 0, failed: 0, ignored: 0, measured: 0, filtered: 0 }); }
function nodeSummary(text) {
  const result = {};
  for (const raw of text.split(/\r?\n/)) { const match = clean(raw).match(/^(?:# |ℹ )?(tests|pass|fail|cancelled|skipped|todo) (\d+)$/); if (match) result[match[1]] = +match[2]; }
  return result;
}
module.exports = function finalDynamicEvidence(requirements, verifications) {
  sources.clear();
  const finalReport = read(base + 'FINAL-REVERIFY.md');
  const finalReview = read(base + 'FINAL-REVIEW.md');
  assert(finalReport.includes(expectedHead));
  assert(finalReview.includes('2/2 CLOSED'));
  const finalIndexPath = prefix + 'reverify-final-evidence-index.json';
  const finalIndex = JSON.parse(read(finalIndexPath));
  assert.equal(finalIndex.head, expectedHead);
  assert.equal(finalIndex.entries.length, 56);
  for (const entry of finalIndex.entries) {
    read(entry.file);
    const observed = sources.get(entry.file);
    assert.equal(observed.bytes, entry.bytes, 'Indexed byte size changed: ' + entry.file);
    assert.equal(observed.sha256, entry.sha256, 'Indexed SHA256 changed: ' + entry.file);
  }
  const metadataFile = prefix + 'ci-41994479295-final.json';
  const metadata = JSON.parse(read(metadataFile));
  assert.equal(metadata.headSha, expectedHead);
  assert.equal(metadata.databaseId, 41994479295);
  assert.equal(metadata.status, 'completed');
  assert.equal(metadata.conclusion, 'success');
  assert.equal(metadata.jobs.length, 5);
  assert(metadata.jobs.every(job => job.status === 'completed' && job.conclusion === 'success'));
  const ownerStatus = JSON.parse(read(prefix + 'reverify-status-after.json'));
  assert.equal(ownerStatus.status, 'passed');
  read(prefix + 'reverify-phase-statuses-after.json');
  const phase7Path = '.planning/phases/07-rest-mcp-shell/07-VERIFICATION.md';
  const phase7Front = extractFrontmatter(read(phase7Path), phase7Path);
  const fingerprint = JSON.parse(read(prefix + 'reverify-fingerprint.json'));
  assert.equal(phase7Front.covered_files.length, 248);
  assert.equal(phase7Front.covered_digest, fingerprint.covered_digest);
  assert.deepEqual(phase7Front.covered_files, fingerprint.covered_files);
  assert.equal(phase7Front.score, '80/80 must-haves verified');
  const localFile = prefix + 'reverify-workspace.log';
  const discoveryFile = prefix + 'reverify-workspace-discovery-c.log';
  const localText = read(localFile);
  const discoveryText = read(discoveryFile);
  assert(localText.includes('head=' + expectedHead) && discoveryText.includes('head=' + expectedHead));
  assert(/^exit_code=0\s*$/m.test(localText) && /^exit_code=0\s*$/m.test(discoveryText));
  const local = parseRust(localText, localFile);
  const discovery = parseDiscovery(discoveryText);
  assert.equal(local.length, 152);
  assert.equal(discovery.length, 152);
  assert.equal(new Set(local.map(item => item.identity)).size, 152);
  assert.equal(new Set(discovery.map(item => item.identity)).size, 152);
  assert.deepEqual(local.map(item => item.identity).sort(), discovery.map(item => item.identity).sort());
  for (const target of local) {
    const found = discovery.find(item => item.identity === target.identity);
    assert.equal(target.passed + target.failed + target.ignored + target.measured + target.filtered, found.discovered, target.identity);
  }
  const localTotals = totals(local);
  assert.deepEqual(localTotals, { passed: 1528, failed: 0, ignored: 119, measured: 0, filtered: 0 });
  assert.equal(local.filter(item => item.passed > 0).length, 128);
  const ownerLocal = JSON.parse(read(prefix + 'reverify-workspace-counts.json'));
  assert.equal(ownerLocal.complete, true);
  assert.equal(ownerLocal.successful_targets, 128);
  assert.deepEqual(ownerLocal.totals, localTotals);
  const ciFiles = { windows: 'test (windows-latest)', ubuntu: 'test (ubuntu-latest)', integration: 'integration', 'phase7-real-services': 'phase7-real-services', 'isolated-query-bench': 'isolated-query-bench' };
  const ci = [];
  for (const [label, jobName] of Object.entries(ciFiles)) {
    const file = prefix + 'ci-41994479295-' + label + '.log';
    const text = read(file);
    assert(text.includes(expectedHead), 'CI raw head absent: ' + label);
    assert(!/^.*##\[error\]/m.test(text), 'CI raw error marker: ' + label);
    const job = metadata.jobs.find(item => item.name === jobName);
    assert(job);
    const targets = parseRust(text, file);
    assert(targets.length > 0 && targets.every(item => item.status === 'ok' && item.failed === 0));
    const workspace = targets.filter(item => item.step?.command === 'cargo test --workspace --locked -- --test-threads=1');
    let workspaceTotals = null;
    if (['windows', 'ubuntu'].includes(label)) {
      const window = job.steps.find(item => item.name === 'Run cargo test --workspace --locked -- --test-threads=1');
      assert(window && window.conclusion === 'success');
      assert.equal(workspace.length, 152);
      assert.equal(new Set(workspace.map(item => item.identity)).size, 152);
      workspaceTotals = totals(workspace);
      assert.deepEqual(workspaceTotals, { passed: label === 'ubuntu' ? 1535 : 1528, failed: 0, ignored: 119, measured: 0, filtered: 0 });
      for (const item of workspace) {
        const time = Date.parse(item.timestamp);
        assert(time >= Date.parse(window.startedAt) && time <= Date.parse(window.completedAt) + 1000, 'Workspace result outside metadata window');
      }
    }
    const node = nodeSummary(text);
    if (label === 'phase7-real-services') assert.deepEqual(node, { tests: 17, pass: 17, fail: 0, cancelled: 0, skipped: 0, todo: 0 });
    const bench = text.split(/\r?\n/).map(clean).filter(line => /^[\w_]+: p95=/.test(line));
    if (label === 'isolated-query-bench') assert.equal(bench.filter(line => line.includes('passed=true')).length, 4);
    ci.push({ label, file, job_name: jobName, job_id: job.databaseId, conclusion: job.conclusion, workspace_harnesses: workspace.length, workspace_totals: workspaceTotals, node, targets, bench, cleanup_steps: job.steps.filter(step => /cleanup|clean up|remove|stop|upload/i.test(step.name)).map(step => ({ name: step.name, conclusion: step.conclusion })) });
  }
  const atomicFile = prefix + 'reverify-sqlite-atomic.log';
  const atomicText = read(atomicFile);
  assert(atomicText.includes('head=' + expectedHead) && /^exit_code=0\s*$/m.test(atomicText));
  const atomic = parseRust(atomicText, atomicFile);
  assert.equal(atomic.length, 1);
  assert.equal(atomic[0].passed, 5);
  assert.equal(atomic[0].failed, 0);
  for (const file of ['reverify-tls-hermetic.log', 'reverify-tls-real.log']) assert(/^exit_code=0\s*$/m.test(read(prefix + file)));
  const tddText = read(prefix + 'reverify-final-validation.log');
  const counterTdd = nodeSummary(tddText);
  assert.deepEqual(counterTdd, { tests: 11, pass: 11, fail: 0, cancelled: 0, skipped: 0, todo: 0 });
  assert(tddText.includes('final_assert_exit_code=0') && tddText.includes('node_test_exit_code=0'));
  const lastProductEnd = Date.parse(localText.match(/ended_utc=([^\r\n]+)/)[1]);
  const lastCiEnd = Math.max(...metadata.jobs.flatMap(job => job.steps.map(step => Date.parse(step.completedAt))).filter(Number.isFinite));
  assert(Date.parse(phase7Front.verified) > lastProductEnd && Date.parse(phase7Front.verified) > lastCiEnd, 'Fingerprint/report must postdate real product execution');
  const map = {
    'SCHEMA-01':'loader load_examples cli', 'SCHEMA-02':'object_types', 'SCHEMA-03':'object_types', 'SCHEMA-04':'link_types', 'SCHEMA-05':'action_types analyze criteria', 'SCHEMA-06':'diagnostics cli cli_color invalid_examples', 'SCHEMA-07':'schema_export cli_schema',
    'LINT-01':'cli_lint ontoforge_schema lint_examples', 'LINT-02':'cli_lint lint_examples ontoforge_schema', 'LINT-03':'cli_lint cli_color', 'LINT-04':'cli_lint ontoforge_schema', 'DX-03':'cli dev_lifecycle',
    'BIND-01':'bindings bind_check_isolation', 'BIND-02':'bindings doc_type_table', 'BIND-03':'cli_online cli_check_pk pk_check drift drift_budget source_drift source_drift_trino_it cli_trino_it',
    'ENGINE-01':'trino_client trino_resilience trino_it', 'ENGINE-02':'health_lifecycle_it health_trino_it trino_resilience', 'ENGINE-03':'printer_prop printer_literals sqlite_engine trino_it', 'ENGINE-04':'sqlite_engine dev_lifecycle', 'ENGINE-05':'trino_it conformance_pg',
    'STORE-01':'conformance_sqlite conformance_pg', 'STORE-02':'reload migrate cli_migrate reload_pg migrate_pg reload_trino_it', 'STORE-03':'conformance_sqlite conformance_pg frozen_history', 'STORE-04':'history_masking history_pagination history_errors', 'STORE-05':'cardinality_check cli_check_cardinality conformance_sqlite conformance_pg cli_trino_it', 'STORE-06':'import cli_import', 'STORE-07':'model_prop overlay_prop conformance_sqlite',
    'SCOPE-01':'scope_store cli_scope oidc', 'SCOPE-02':'scope_compile scope_documents', 'SCOPE-03':'scope_merge', 'SCOPE-04':'rowpred_diff rowpred_trino_it', 'SCOPE-05':'scope_compile security', 'SCOPE-06':'criteria_coverage reload migrate', 'SCOPE-07':'request_snapshot scope_store mcp_contract',
    'AUTH-01':'oidc', 'AUTH-02':'dev_token cli_dev_token',
    'QUERY-01':'pk_read sqlite_query', 'QUERY-02':'sqlite_query sql_shape trino_query_it', 'QUERY-03':'sqlite_query trino_query_it', 'QUERY-04':'sqlite_query search_around', 'QUERY-05':'search_around trino_query_it', 'QUERY-06':'set_ops', 'QUERY-07':'aggregate', 'QUERY-08':'overlay_prop sqlite_query trino_query_it', 'QUERY-09':'security audit_pk', 'QUERY-10':'sql_shape query_bench_it',
    'ACTION-01':'action_types action', 'ACTION-02':'action', 'ACTION-03':'action security', 'ACTION-04':'action', 'ACTION-05':'action idempotency http_actions', 'ACTION-06':'conformance_sqlite conformance_pg action_pg', 'ACTION-07':'entrypoints import', 'ACTION-08':'security http_action_safety', 'ACTION-09':'source action_pg', 'ACTION-10':'idempotency idempotency_pg', 'ACTION-11':'outbox cli_webhook_worker outbox_pg', 'ACTION-12':'source action_source_budget',
    'L2-01':'l2_discovery l2_execute mcp_l2', 'L2-02':'l2_gate l2_explain', 'L2-03':'l2_gate mcp_l2_defaults trino_l2_acl_it', 'L2-04':'l2_monitor l2_health health_trino_it trino_l2_acl_it', 'AUDIT-01':'audit_l0 audit_pk conformance_pg',
    'API-01':'http_contract health_lifecycle_it', 'API-02':'http_actions history_pagination http_contract', 'API-03':'openapi_snapshot error_diagnostics', 'API-04':'contract ontoforge_client', 'API-05':'NODE_TS17',
    'MCP-01':'mcp_transport cli_stdio', 'MCP-02':'mcp_contract', 'MCP-03':'mcp_contract', 'MCP-04':'mcp_actions', 'MCP-05':'mcp_actions', 'MCP-06':'mcp_actions http_action_safety', 'MCP-07':'mcp_l2_defaults mcp_contract', 'MCP-08':'mcp_contract', 'DX-01':'dev_lifecycle cli_shutdown cli_dev_token', 'DX-02':'dev_lifecycle http_contract', 'DX-04':''
  };
  assert.equal(Object.keys(map).length, 79);
  const externalGuardTargets = new Set(['trino_it', 'cli_trino_it', 'trino_action_it', 'rowpred_trino_it', 'trino_query_it', 'query_bench_it', 'private_fixtures']);
  const available = [...local.filter(item => !externalGuardTargets.has(item.name)), ...atomic, ...ci.flatMap(job => job.targets).filter(item => !['windows', 'ubuntu'].some(label => item.file.endsWith('-' + label + '.log')) || !externalGuardTargets.has(item.name))].filter(item => item.passed > 0 && item.failed === 0);
  const phase7Gate = verifications.find(item => item.phase === '7');
  for (const requirement of requirements) {
    assert(Object.hasOwn(map, requirement.id), requirement.id);
    const names = map[requirement.id].split(' ').filter(Boolean);
    for (const name of names.filter(value => value !== 'NODE_TS17')) assert(available.some(item => item.name === name), 'Mapped target has no positive execution: ' + requirement.id + ':' + name);
    const evidence = available.filter(item => names.includes(item.name)).map(({ file, line, result_line, identity, name, passed, ignored, step }) => ({ file, line, result_line, identity, name, passed, ignored, invocation: step?.command || (file === atomicFile ? 'local SQLite action_atomic feature execution' : 'local workspace') }));
    if (names.includes('NODE_TS17')) evidence.push({ file: prefix + 'ci-41994479295-phase7-real-services.log', kind: 'typescript-node-tests', passed: 17, failed: 0, node_summary: ci.find(item => item.label === 'phase7-real-services').node });
    if (requirement.id !== 'DX-04') assert(evidence.length > 0, 'No actual positive execution: ' + requirement.id);
    const ownerSigned = requirement.phases.includes('7') && requirement.id !== 'DX-04' && phase7Gate.canonical_status === 'passed';
    requirement.current_execution = { status: requirement.id === 'DX-04' ? 'historical_only' : requirement.id === 'ENGINE-05' ? 'partial_current_execution' : 'mapped_tests_executed', head: expectedHead, evidence, scope: requirement.id === 'DX-04' ? '本轮无三平台native release/archive重跑；保留Phase07历史发行验收，不将workspace替代发行证据' : requirement.id === 'ENGINE-05' ? '当前Trino+PG integration真实执行；MinIO为07.2历史独立验收，SQL Server私有fixture未执行/未闭合' : '仅签署映射的实际非ignored target/子合同执行，不推导需求全部路径或历史judgment通过', owner_signed_current_scope: ownerSigned, owner_signature_scope: ownerSigned ? 'Phase07最终80/80复签中的当前映射行为；不扩签其他phase报告或历史发行产物' : null, report_resign_pending: requirement.phases.some(phase => !requirement.report_gates.some(gate => gate.phase === phase && gate.canonical_status === 'passed')), preserved_boundaries: ['原始ignored不计通过', '平台/调用计数不相加为唯一测试数', ...(requirement.id === 'SCHEMA-07' ? ['编辑器体验仍在Phase01历史human边界'] : []), ...(requirement.phases.some(phase => phase === '1' || phase === '2') ? ['01/02历史judgment保留，不归为功能失败'] : [])] };
    requirement.dynamic_owner_signed = ownerSigned;
    requirement.freshness = requirement.id === 'DX-04' ? 'historical_release_not_rerun' : requirement.id === 'ENGINE-05' ? 'current_execution_partial_contract' : requirement.current_execution.report_resign_pending ? 'current_execution_report_resign_pending' : 'current_execution_reports_current';
    requirement.current_functional_status = requirement.id === 'ENGINE-05' ? 'Partial: MinIO已07.2历史闭合；本轮PG/Trino实跑；SQL Server私有fixture未闭合' : requirement.id === 'DX-04' ? '历史发行验收保留；本轮未重跑发行，不以CI整体success代替' : '本轮映射target真实执行通过；非完整需求全路径保证；历史01/02及stale门保留';
  }
  return { head: expectedHead, final_evidence_index: { path: finalIndexPath, entries: finalIndex.entries.length, mismatches: [], raw_byte_sha256_match: true }, ci_run: metadata.databaseId, dispatch_actor: '主会话按用户继续指令dispatch', final_logs_reviewed: true, owner_dynamic_complete: true, raw_log_count: 5, local: { harnesses: local.length, successful_targets: local.filter(item => item.passed > 0).length, totals: localTotals, discovery_identity_match: true }, ci: ci.map(({ targets, ...job }) => ({ ...job, rust_target_invocations: targets.length })), phase07: { status: phase7Gate.canonical_status, score: phase7Front.score, verified_at: phase7Front.verified, covered_files: phase7Front.covered_files.length, covered_digest: phase7Front.covered_digest, signature_after_product_and_ci: true }, counter_tdd: { ...counterTdd, standards_findings: 2, standards_closed: 2 }, requirement_signature_counts: { current_execution_evidence_ids: requirements.filter(item => item.current_execution.evidence.length).length, mapped_tests_executed_ids: requirements.filter(item => item.current_execution.status === 'mapped_tests_executed').length, partial_current_execution_ids: requirements.filter(item => item.current_execution.status === 'partial_current_execution').length, historical_only_ids: requirements.filter(item => item.current_execution.status === 'historical_only').length, owner_signed_current_phase07_scope_ids: requirements.filter(item => item.dynamic_owner_signed).length, owner_signed_phase07_only_responsibility_ids: requirements.filter(item => item.dynamic_owner_signed && item.phases.length === 1).length, owner_signed_crossphase_scope_ids: requirements.filter(item => item.dynamic_owner_signed && item.phases.length > 1).length, current_execution_reports_current_ids: requirements.filter(item => item.freshness === 'current_execution_reports_current').length, current_execution_report_resign_pending_ids: requirements.filter(item => item.current_execution.evidence.length > 0 && item.current_execution.report_resign_pending).length }, source_manifest: [...sources.values()] };
};
