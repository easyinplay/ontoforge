const fs = require('node:fs');
const path = require('node:path');
function parseCargo(file) {
  const text = fs.readFileSync(file, 'utf8');
  const targets = [];
  const pending=[];
  for (const original of text.split(/\r?\n/)) {
    const line = original.replace(/^\d{4}-\d\d-\d\dT\S+\s+/, '').trim();
    if (/^Running (unittests |tests\/|tests\\)/.test(line) || /^Doc-tests /.test(line)) {
      const executable = line.match(/\(([^)]+)\)$/)?.[1];
      const target = {target: line, identity: executable ? path.win32.basename(executable) : line, discovered: 0, named_executed: 0, named_ignored: 0, result: null};
      targets.push(target);
      pending.push(target);
    }
    const current=pending[0];
    if (!current) continue;
    if (/^.+: test$/.test(line)) current.discovered++;
    if (/^test .+ \.\.\. ok$/.test(line)) current.named_executed++;
    if (/^test .+ \.\.\. ignored(?:,.*)?$/.test(line)) current.named_ignored++;
    const result = line.match(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/);
    if (result) {current.result = {status: result[1], passed: +result[2], failed: +result[3], ignored: +result[4], measured: +result[5], filtered: +result[6]};pending.shift();}
    if (/^\d+ tests?, \d+ benchmarks?$/.test(line)) {current.listed_count=+line.match(/^\d+/)[0];pending.shift();}
  }
  return {file, exit_code: text.match(/^exit_code=(\d+)$/m)?.[1] ?? null, targets};
}
const dir = __dirname;
const discovery = parseCargo(`${dir}/reverify-workspace-discovery-c.log`);
const execution = parseCargo(`${dir}/reverify-workspace.log`);
const discoveryIdentities=new Set(discovery.targets.map(target=>target.identity));
const executionIdentities=new Set(execution.targets.map(target=>target.identity));
const identitiesMatch=discoveryIdentities.size===discovery.targets.length&&executionIdentities.size===execution.targets.length&&discoveryIdentities.size===executionIdentities.size&&[...discoveryIdentities].every(identity=>executionIdentities.has(identity));
const byIdentity = new Map(discovery.targets.map(target => [target.identity, target]));
const emptyHarnessExemptions=new Map([
  ['ontoforge-59cfc5af2bc1f8a6.exe','CLI main has no unit tests; CLI behavior runs in named integration targets'],
  ['action_pg-96d6ffad486cfa75.exe','action_pg.rs:1 requires test-support; CI integration explicitly enables it and executes 4'],
  ['idempotency_pg-8394f8fa2a79d11c.exe','idempotency_pg.rs:1 requires test-support; CI integration explicitly enables it and executes 6'],
  ['Doc-tests ontoforge','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_action','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_api','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_auth','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_client','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_expr','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_schema','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_server','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
  ['Doc-tests ontoforge_storage','Named empty doctest harness at HEAD 060019f; no passing tests claimed'],
]);
const ignoredOnlyExemptions=new Map([
  ['outbox_pg-6ae8c6053b056d64.exe',[2,'External PG; CI integration explicitly executes webhook 2']],
  ['trino_l2_acl_it-837a182814b3b013.exe',[6,'External Trino; CI phase7-real-services executes ACL 6']],
  ['health_trino_it-2a22b9c18e6edd91.exe',[2,'External Trino; CI phase7-real-services executes health 2']],
  ['postgres_http_it-031a3162ed342f50.exe',[4,'External PG; CI phase7-real-services executes HTTP 4']],
  ['source_drift_trino_it-fef7a8cde2063cc4.exe',[2,'External Trino; CI phase7-real-services executes source drift 2']],
  ['trino_http_it-8455493ade653e33.exe',[2,'External Trino; CI phase7-real-services executes HTTP 2']],
  ['conformance_pg-6e1a67f4d83a0acf.exe',[45,'External PG; CI integration explicitly executes conformance 45']],
  ['migrate_pg-c82bca4eabba7b09.exe',[4,'External PG; CI integration explicitly executes migrations 4']],
  ['query_read_pg-fba053d23de5db89.exe',[2,'External PG; CI integration explicitly executes query reads 2']],
  ['reload_pg-181387145679bc34.exe',[11,'External PG; CI integration explicitly executes reload 11']],
  ['reload_trino_it-a69493e382f61c94.exe',[3,'External Trino; CI integration explicitly executes reload 3']],
  ['Doc-tests ontoforge_engine',[1,'Explicit ignore code example in src/trino/l2_monitor.rs:123; not runtime evidence']],
]);
const targets = execution.targets.map(target => {
  const found = byIdentity.get(target.identity);
  const total = target.result ? target.result.passed + target.result.failed + target.result.ignored + target.result.measured + target.result.filtered : null;
  const discovered = found?.discovered ?? null;
  const ignoredPolicy=ignoredOnlyExemptions.get(target.identity);
  const exemption=discovered===0&&emptyHarnessExemptions.has(target.identity)?{kind:'empty_harness',reason:emptyHarnessExemptions.get(target.identity)}:ignoredPolicy&&discovered===ignoredPolicy[0]&&target.result?.ignored===discovered?{kind:'ignored_only',reason:ignoredPolicy[1]}:null;
  return {...target, discovered, exemption, discovery_matches_execution: discovered !== null && total === discovered, classification: !target.result ? 'INCOMPLETE' : target.result.failed ? 'FAILED' : total !== discovered ? 'DISCOVERY_MISMATCH' : exemption?.kind==='empty_harness' ? 'EXEMPT_EMPTY_HARNESS' : exemption?.kind==='ignored_only' ? 'EXEMPT_IGNORED_ONLY' : discovered === 0 ? 'NO_TESTS_NOT_A_PASS' : target.result.passed === 0 ? 'IGNORED_ONLY_NOT_A_PASS' : 'PASS'};
});
const totals = targets.reduce((acc, target) => {if(target.result)for(const key of ['passed','failed','ignored','measured','filtered'])acc[key]+=target.result[key];return acc;}, {passed:0,failed:0,ignored:0,measured:0,filtered:0});
const result = {created_at:new Date().toISOString(),discovery_exit:discovery.exit_code,execution_exit:execution.exit_code,discovered_targets:discovery.targets.length,executed_targets:targets.length,discovered_tests:discovery.targets.reduce((total,target)=>total+target.discovered,0),identities_match:identitiesMatch,successful_targets:targets.filter(target=>target.classification==='PASS').length,totals,complete:discovery.exit_code==='0'&&execution.exit_code==='0'&&targets.length>0&&identitiesMatch&&targets.every(target=>target.discovery_matches_execution&&['PASS','EXEMPT_EMPTY_HARNESS','EXEMPT_IGNORED_ONLY'].includes(target.classification))&&totals.passed>0&&totals.failed===0,targets};
fs.writeFileSync(`${dir}/reverify-workspace-counts.json`,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({...result,targets:targets.filter(target=>target.classification!=='PASS')},null,2));
