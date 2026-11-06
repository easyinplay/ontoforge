'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const dynamicEvidence = require('../requirements-final-dynamic.cjs');

function requirement(id, gates) {
  return {
    id,
    phases: gates.map(([phase]) => phase),
    report_gates: gates.map(([phase, canonical_status]) => ({ phase, canonical_status })),
  };
}

function evaluate(requirements, phase7Status = 'passed') {
  const result = dynamicEvidence(requirements, [{ phase: '7', canonical_status: phase7Status }]);
  return { requirements, counts: result.requirement_signature_counts };
}

test('passed assigned reports make non-Phase07 execution current without extending owner signature', () => {
  const entry = requirement('QUERY-02', [['4', 'passed']]);
  const { counts } = evaluate([entry]);
  assert.equal(entry.current_execution.status, 'mapped_tests_executed');
  assert(entry.current_execution.evidence.length > 0);
  assert.equal(entry.current_execution.report_resign_pending, false);
  assert.equal(entry.freshness, 'current_execution_reports_current');
  assert.equal(entry.dynamic_owner_signed, false);
  assert.equal(counts.current_execution_reports_current_ids, 1);
  assert.equal(counts.current_execution_report_resign_pending_ids, 0);
  assert.equal(counts.owner_signed_current_phase07_scope_ids, 0);
});

for (const canonicalStatus of ['stale', 'human_needed', 'gaps_found']) {
  test(`${canonicalStatus} canonical gate keeps executed evidence pending despite a passed raw claim`, () => {
    const entry = requirement('QUERY-02', [['4', canonicalStatus]]);
    entry.report_gates[0].raw_status = 'passed';
    const { counts } = evaluate([entry]);
    assert.equal(entry.freshness, 'current_execution_report_resign_pending');
    assert.equal(entry.current_execution.report_resign_pending, true);
    assert.equal(counts.current_execution_reports_current_ids, 0);
    assert.equal(counts.current_execution_report_resign_pending_ids, 1);
    assert.equal(entry.dynamic_owner_signed, false);
  });
}

for (const [id, phase] of [['STORE-01', '3'], ['ACTION-01', '5'], ['L2-01', '6'], ['STORE-05', '7.1']]) {
  test(`passed Phase${phase} report makes ${id} execution current without a Phase07 owner signature`, () => {
    const entry = requirement(id, [[phase, 'passed']]);
    const { counts } = evaluate([entry]);
    assert.equal(entry.freshness, 'current_execution_reports_current');
    assert.equal(entry.dynamic_owner_signed, false);
    assert.equal(counts.current_execution_reports_current_ids, 1);
    assert.equal(counts.current_execution_report_resign_pending_ids, 0);
  });
}

test('one stale assigned crossphase gate blocks freshness after the other report passes', () => {
  const entry = requirement('STORE-05', [['3', 'passed'], ['7.1', 'stale']]);
  const { counts } = evaluate([entry]);
  assert.equal(entry.freshness, 'current_execution_report_resign_pending');
  assert.equal(counts.current_execution_reports_current_ids, 0);
  assert.equal(counts.current_execution_report_resign_pending_ids, 1);
});

test('duplicate passed report cannot cover a different missing assigned phase', () => {
  const entry = requirement('STORE-05', [['3', 'passed'], ['7.1', 'passed']]);
  entry.report_gates[1] = { phase: '3', canonical_status: 'passed' };
  const { counts } = evaluate([entry]);
  assert.equal(entry.current_execution.report_resign_pending, true);
  assert.equal(entry.freshness, 'current_execution_report_resign_pending');
  assert.equal(counts.current_execution_report_resign_pending_ids, 1);
});

test('pending metric counts executed evidence with unresolved gates rather than freshness labels or owner signatures', () => {
  const entries = [
    requirement('QUERY-02', [['4', 'passed']]),
    requirement('STORE-05', [['3', 'passed'], ['7.1', 'stale']]),
    requirement('ENGINE-05', [['2', 'gaps_found'], ['7.2', 'passed']]),
    requirement('DX-04', [['7', 'stale']]),
    requirement('SCHEMA-05', [['1', 'human_needed'], ['7', 'passed']]),
    requirement('API-01', [['7', 'passed']]),
  ];
  const { counts } = evaluate(entries);
  assert.equal(counts.current_execution_evidence_ids, 5);
  assert.equal(counts.mapped_tests_executed_ids, 4);
  assert.equal(counts.partial_current_execution_ids, 1);
  assert.equal(counts.historical_only_ids, 1);
  assert.equal(counts.current_execution_reports_current_ids, 2);
  assert.equal(counts.current_execution_report_resign_pending_ids, 3);
  assert.equal(counts.owner_signed_current_phase07_scope_ids, 2);
  assert.equal(counts.owner_signed_phase07_only_responsibility_ids, 1);
  assert.equal(counts.owner_signed_crossphase_scope_ids, 1);
});

test('existing 79-requirement assignment retains exactly 21 Phase07 scoped signatures', () => {
  const matrix = require('../../../requirements-matrix.json');
  const entries = structuredClone(matrix.requirements);
  assert.equal(entries.length, 79);
  const { counts } = evaluate(entries);
  assert.equal(counts.owner_signed_current_phase07_scope_ids, 21);
  assert.equal(counts.owner_signed_phase07_only_responsibility_ids, 15);
  assert.equal(counts.owner_signed_crossphase_scope_ids, 6);
  assert.equal(entries.find(entry => entry.id === 'DX-04').dynamic_owner_signed, false);
  assert.equal(entries.find(entry => entry.id === 'ENGINE-05').current_execution.status, 'partial_current_execution');
});

test('all assigned crossphase gates passed make current evidence reports current', () => {
  const entry = requirement('STORE-05', [['3', 'passed'], ['7.1', 'passed']]);
  const { counts } = evaluate([entry]);
  assert.equal(entry.freshness, 'current_execution_reports_current');
  assert.equal(entry.dynamic_owner_signed, false);
  assert.equal(counts.current_execution_reports_current_ids, 1);
  assert.equal(counts.current_execution_report_resign_pending_ids, 0);
});

test('Phase07 owner signature cannot cover another assigned phase human gate', () => {
  const entry = requirement('SCHEMA-05', [['1', 'human_needed'], ['7', 'passed']]);
  const { counts } = evaluate([entry]);
  assert.equal(entry.dynamic_owner_signed, true);
  assert.equal(entry.current_execution.owner_signed_current_scope, true);
  assert.equal(entry.current_execution.owner_signature_scope, 'Phase07最终80/80复签中的当前映射行为；不扩签其他phase报告或历史发行产物');
  assert.equal(entry.current_execution.report_resign_pending, true);
  assert.equal(entry.freshness, 'current_execution_report_resign_pending');
  assert.equal(counts.owner_signed_current_phase07_scope_ids, 1);
  assert.equal(counts.owner_signed_crossphase_scope_ids, 1);
  assert.equal(counts.current_execution_report_resign_pending_ids, 1);
  assert.equal(counts.current_execution_reports_current_ids, 0);
});

test('Phase07-only passed report retains its scoped owner signature alongside report freshness', () => {
  const entry = requirement('API-01', [['7', 'passed']]);
  const { counts } = evaluate([entry]);
  assert.equal(entry.dynamic_owner_signed, true);
  assert.equal(entry.freshness, 'current_execution_reports_current');
  assert.equal(counts.owner_signed_phase07_only_responsibility_ids, 1);
  assert.equal(counts.current_execution_reports_current_ids, 1);
});

test('Phase03 historical human approval remains pending across Phase06 and signed Phase07', () => {
  const entry = requirement('SCOPE-07', [['3', 'human_needed'], ['6', 'passed'], ['7', 'passed']]);
  entry.report_gates[0].raw_status = 'human_needed';
  entry.report_gates[0].score = '138/139 must-haves verified';
  const { counts } = evaluate([entry]);
  assert.equal(entry.dynamic_owner_signed, true);
  assert.equal(entry.current_execution.owner_signed_current_scope, true);
  assert.equal(entry.current_execution.status, 'mapped_tests_executed');
  assert.equal(entry.current_execution.report_resign_pending, true);
  assert.equal(entry.freshness, 'current_execution_report_resign_pending');
  assert.equal(counts.current_execution_reports_current_ids, 0);
  assert.equal(counts.current_execution_report_resign_pending_ids, 1);
  assert.equal(counts.owner_signed_current_phase07_scope_ids, 1);
  assert.equal(counts.owner_signed_crossphase_scope_ids, 1);
  assert.equal(entry.report_gates[0].canonical_status, 'human_needed');
});

test('a stale Phase07 gate grants neither owner signature nor report freshness', () => {
  const entry = requirement('API-01', [['7', 'stale']]);
  const { counts } = evaluate([entry], 'stale');
  assert.equal(entry.dynamic_owner_signed, false);
  assert.equal(entry.freshness, 'current_execution_report_resign_pending');
  assert.equal(counts.owner_signed_current_phase07_scope_ids, 0);
  assert.equal(counts.current_execution_report_resign_pending_ids, 1);
});

for (const status of ['passed', 'stale']) {
  test(`DX-04 stays historical-only without executed-evidence pending count when report is ${status}`, () => {
    const entry = requirement('DX-04', [['7', status]]);
    const { counts } = evaluate([entry], status);
    assert.equal(entry.current_execution.status, 'historical_only');
    assert.deepEqual(entry.current_execution.evidence, []);
    assert.equal(entry.freshness, 'historical_release_not_rerun');
    assert.equal(entry.dynamic_owner_signed, false);
    assert.equal(counts.current_execution_reports_current_ids, 0);
    assert.equal(counts.current_execution_report_resign_pending_ids, 0);
    assert.equal(counts.historical_only_ids, 1);
    assert.equal(entry.current_functional_status, '历史发行验收保留；本轮未重跑发行，不以CI整体success代替');
  });
}

for (const status of ['passed', 'gaps_found']) {
  test(`ENGINE-05 stays partial with report gate ${status} and counts only actual pending gates`, () => {
    const entry = requirement('ENGINE-05', [['2', status], ['7.2', 'passed']]);
    const { counts } = evaluate([entry]);
    assert.equal(entry.current_execution.status, 'partial_current_execution');
    assert(entry.current_execution.evidence.length > 0);
    assert.equal(entry.freshness, 'current_execution_partial_contract');
    assert.equal(entry.dynamic_owner_signed, false);
    assert.equal(entry.current_execution.report_resign_pending, status !== 'passed');
    assert.equal(counts.current_execution_reports_current_ids, 0);
    assert.equal(counts.current_execution_report_resign_pending_ids, status === 'passed' ? 0 : 1);
    assert.equal(counts.partial_current_execution_ids, 1);
    assert.equal(entry.current_functional_status, 'Partial: MinIO已07.2历史闭合；本轮PG/Trino实跑；SQL Server私有fixture未闭合');
  });
}

test('missing assigned canonical report remains pending rather than vacuously current', () => {
  const entry = requirement('STORE-05', [['3', 'passed'], ['7.1', 'passed']]);
  entry.report_gates.pop();
  const { counts } = evaluate([entry]);
  assert.equal(entry.current_execution.report_resign_pending, true);
  assert.equal(entry.freshness, 'current_execution_report_resign_pending');
  assert.equal(counts.current_execution_report_resign_pending_ids, 1);
});
