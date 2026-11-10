const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const {test} = require('node:test');
const fixtureRoot = 'C:/Users/easyi/AppData/Local/Temp/ontoforge-reverify-060019f-20261105/count-fixtures';
function evaluate(discovered, executed) {
  fs.mkdirSync(fixtureRoot, {recursive:true});
  const dir = fs.mkdtempSync(path.join(fixtureRoot,'case-'));
  fs.copyFileSync(path.join(__dirname,'reverify-counts.cjs'),path.join(dir,'reverify-counts.cjs'));
  const header = entry => entry.doc ? `Doc-tests ${entry.name}` : `Running tests/${entry.name}.rs (target/debug/deps/${entry.name}-fixture.exe)`;
  const discovery = discovered.map(entry => [header(entry),...Array.from({length:entry.count},(_,index)=>`${entry.name}_${index}: test`),`${entry.count} tests, 0 benchmarks`].join('\n')).join('\n');
  const execution = executed.map(entry => [header(entry),`running ${entry.passed+(entry.ignored||0)} tests`,...Array.from({length:entry.passed},(_,index)=>`test ${entry.name}_${index} ... ok`),`test result: ok. ${entry.passed} passed; 0 failed; ${entry.ignored||0} ignored; 0 measured; 0 filtered out; finished in 0.01s`].join('\n')).join('\n');
  fs.writeFileSync(path.join(dir,'reverify-workspace-discovery-c.log'),discovery+'\nexit_code=0\n');
  fs.writeFileSync(path.join(dir,'reverify-workspace.log'),execution+'\nexit_code=0\n');
  const run = cp.spawnSync(process.execPath,[path.join(dir,'reverify-counts.cjs')],{encoding:'utf8'});
  assert.equal(run.status,0,run.stderr);
  return JSON.parse(fs.readFileSync(path.join(dir,'reverify-workspace-counts.json'),'utf8'));
}
test('required target with zero discovery cannot complete beside a passing target',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'beta',count:0}],[{name:'alpha',passed:1},{name:'beta',passed:0}]);
  assert.equal(result.complete,false);
});
test('duplicate execution identity cannot conceal a missing required target',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'beta',count:1}],[{name:'alpha',passed:1},{name:'alpha',passed:1}]);
  assert.equal(result.complete,false);
});
test('explicitly named empty doctest harness is exempt and never counted successful',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'ontoforge_api',doc:true,count:0}],[{name:'alpha',passed:1},{name:'ontoforge_api',doc:true,passed:0}]);
  assert.equal(result.complete,true);
  assert.equal(result.successful_targets,1);
  assert.equal(result.targets[1].classification,'EXEMPT_EMPTY_HARNESS');
});
test('explicitly ignored documentation example remains excluded not nonignored success',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'ontoforge_engine',doc:true,count:1}],[{name:'alpha',passed:1},{name:'ontoforge_engine',doc:true,passed:0,ignored:1}]);
  assert.equal(result.complete,true);
  assert.equal(result.successful_targets,1);
  assert.equal(result.targets[1].classification,'EXEMPT_IGNORED_ONLY');
  assert.equal(result.totals.passed,1);
  assert.equal(result.totals.ignored,1);
});
test('unknown ignored-only required target is not successful execution',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'beta',count:1}],[{name:'alpha',passed:1},{name:'beta',passed:0,ignored:1}]);
  assert.equal(result.complete,false);
  assert.equal(result.successful_targets,1);
  assert.equal(result.targets[1].classification,'IGNORED_ONLY_NOT_A_PASS');
});
test('unknown empty doctest is not automatically exempt',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'unknown',doc:true,count:0}],[{name:'alpha',passed:1},{name:'unknown',doc:true,passed:0}]);
  assert.equal(result.complete,false);
  assert.equal(result.targets[1].exemption,null);
});
test('duplicate discovery identities are rejected even with equal aggregate counts',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'alpha',count:1}],[{name:'alpha',passed:1},{name:'beta',passed:1}]);
  assert.equal(result.complete,false);
  assert.equal(result.identities_match,false);
});
test('different identity sets cannot complete with equal counts',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'beta',count:1}],[{name:'alpha',passed:1},{name:'gamma',passed:1}]);
  assert.equal(result.complete,false);
  assert.equal(result.identities_match,false);
});
test('one-to-one target identities may execute in a different order',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'beta',count:2}],[{name:'beta',passed:2},{name:'alpha',passed:1}]);
  assert.equal(result.complete,true);
  assert.equal(result.successful_targets,2);
  assert.equal(result.totals.passed,3);
});
test('a named empty exemption cannot hide missing execution or create success alone',()=>{
  assert.equal(evaluate([{name:'alpha',count:1},{name:'ontoforge_api',doc:true,count:0}],[{name:'alpha',passed:1}]).complete,false);
  const onlyEmpty=evaluate([{name:'ontoforge_api',doc:true,count:0}],[{name:'ontoforge_api',doc:true,passed:0}]);
  assert.equal(onlyEmpty.complete,false);
  assert.equal(onlyEmpty.successful_targets,0);
});
test('ignored-only policy requires its exact reviewed discovery count',()=>{
  const result=evaluate([{name:'alpha',count:1},{name:'ontoforge_engine',doc:true,count:2}],[{name:'alpha',passed:1},{name:'ontoforge_engine',doc:true,passed:0,ignored:2}]);
  assert.equal(result.complete,false);
  assert.equal(result.targets[1].exemption,null);
});
