const fs = require("node:fs");
const path = require("node:path");
const cp = require("node:child_process");
const crypto = require("node:crypto");
const root = process.cwd();
const output = __dirname;
const phase = ".planning/phases/01-workspace-schema-lint";
const tool = "C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs";
const sha = bytes => crypto.createHash("sha256").update(bytes).digest("hex");
function run(command, args) { const result = cp.spawnSync(command, args, {cwd:root,encoding:"utf8",maxBuffer:32*1024*1024}); return {command,args,exit:result.status,stdout:result.stdout,stderr:result.stderr}; }
function query(args) { const result=run(process.execPath,[tool,"query",...args]); try{result.data=JSON.parse(result.stdout);}catch{result.data=result.stdout;} delete result.stdout;return result;}
function save(name,data){fs.writeFileSync(path.join(output,name),JSON.stringify(data,null,2)+"\n");}
const snapshot = path.join(output, '01-VERIFICATION.before.md');
if (!fs.existsSync(snapshot)) fs.copyFileSync(phase + '/01-VERIFICATION.md', snapshot);
const old = query(['frontmatter.get', snapshot]);
const documents = fs.readdirSync(phase).filter(name => /01-\d\d-(PLAN|SUMMARY)\.md$/.test(name)).map(name => {
  const file = phase + '/' + name;
  const bytes = fs.readFileSync(file);
  return {file, sha256: sha(bytes), bytes: bytes.length, text: bytes.toString('utf8'), frontmatter: query(['frontmatter.get', file]).data};
});
const plans = documents.filter(document => document.file.endsWith('-PLAN.md')).map(document => ({...document, artifacts: query(['verify.artifacts', document.file]), links: query(['verify.key-links', document.file])}));
save('contracts.json', {old, documents, plans, roadmap: query(['roadmap.get-phase', '01']), roadmapAnalysis: query(['roadmap.analyze', '--raw']), before: query(['verification.status', phase]), decisions: query(['check.decision-coverage-verify', phase, phase + '/01-CONTEXT.md'])});
const base = path.dirname(output);
const logPaths = ['ubuntu','windows','integration','phase7-real-services','isolated-query-bench'].map(job => ({job, file: path.relative(root, path.join(base, 'ci-41994479295-' + job + '.log')).replaceAll('\\','/')}));
logPaths.push({job: 'local-workspace', file: path.relative(root, path.join(base, 'reverify-workspace.log')).replaceAll('\\','/')});
const tests = [];
for (const {job,file} of logPaths) {
  let target = '';
  fs.readFileSync(file,'utf8').split('\n').forEach((line, index) => {
    const clean = line.replace(/\x1b\[[0-9;]*m/g, '');
    if (/Running .*\(.+\)/.test(clean)) target = clean;
    const match = clean.match(/\btest (\S+) \.\.\. ok\s*$/);
    if (match) tests.push({job, file, line: index + 1, name: match[1], target, output: clean});
  });
}
save('executed-tests.json', tests);
const covered = old.data.covered_files;
save('covered-baseline.json', {report: phase + '/01-VERIFICATION.md', sha256: sha(fs.readFileSync(snapshot)), bytes: fs.statSync(snapshot).size, covered_files: covered, covered_digest: old.data.covered_digest});
const files = covered.map(file => {
  if (!fs.existsSync(file)) return {file, exists: false};
  const bytes = fs.readFileSync(file);
  const lines = bytes.toString('utf8').split('\n');
  const functions = file.endsWith('.rs') ? lines.map((text,index) => ({line: index+1,text})).filter(item => /\b(?:async )?fn \w+/.test(item.text)).map(item => {
    const name = item.text.match(/fn (\w+)/)[1];
    return {...item, name, executions: tests.filter(test => test.name === name || test.name.endsWith('::' + name))};
  }) : undefined;
  return {file, exists: true, bytes: bytes.length, sha256: sha(bytes), text: bytes.toString('utf8'), functions, markers: lines.map((text,index) => ({line:index+1,text})).filter(item => /\b(TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER)\b/.test(item.text))};
});
save('covered-file-inspection.json', files);
const moved = ['CONTRIBUTING.md','Cargo.toml','crates/ontoforge-schema/src/diag.rs','crates/ontoforge/src/cli.rs','crates/ontoforge/src/lib.rs'];
save('changed-source-inspection.json', {head: run('git',['rev-parse','HEAD']), workingSource: run('git',['diff','--name-only','HEAD','--','crates','Cargo.toml','Cargo.lock','CONTRIBUTING.md','.github','ci','examples','schemas']), moved, diff: run('git',['diff','f8851ca','HEAD','--',...moved]), history: run('git',['log','--format=%h %s','--',...moved])});
const inputs = [...new Set([...covered, ...documents.map(document => document.file), ...logPaths.map(log => log.file), '.planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json', '.planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-local-results.json', '.planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-run.ps1', '.planning/REQUIREMENTS.md','.planning/ROADMAP.md','.planning/milestones/v1.0-audit/task_plan.md','.planning/milestones/v1.0-audit/progress.md','.planning/milestones/v1.0-audit/findings.md', phase+'/01-UAT.md',phase+'/01-CONTEXT.md'])];
save('evidence-inputs.json', inputs.map(file => { const bytes = fs.readFileSync(file); return {file, bytes: bytes.length, sha256: sha(bytes)}; }));
save('workspace-baseline.json', {at: new Date().toISOString(), head: run('git',['rev-parse','HEAD']), status: run('git',['status','--porcelain=v1']), pin: 'D:/DataSpellProjects/ontoforge/.planning/milestones/v1.0-audit', plan_id: 'v1.0-audit', pwf: 'resolve-plan-dir.ps1 -PlanRoot D:/DataSpellProjects/ontoforge/.planning/milestones; PLAN_ID=v1.0-audit; PWF_PLAN_ROOT unset'});
console.log(JSON.stringify({plans:plans.length,summaries:documents.length-plans.length,truths:plans.reduce((sum,plan)=>sum+(plan.frontmatter.must_haves?.truths?.length||0),0),covered:covered.length,missing:files.filter(file=>!file.exists).map(file=>file.file),tests:tests.length,prohibitions:plans.flatMap(plan=>plan.frontmatter.must_haves?.prohibitions||[]),before:query(['verification.status',phase]).data,artifacts:plans.map(plan=>({file:plan.file,artifacts:plan.artifacts.data,links:plan.links.data}))},null,2));
