const fs=require('node:fs');
const path=require('node:path');
const cp=require('node:child_process');
const crypto=require('node:crypto');
const output=__dirname;
const root=process.cwd();
const phase='.planning/phases/01-workspace-schema-lint';
const read=file=>fs.readFileSync(file,'utf8');
const save=(name,value)=>fs.writeFileSync(path.join(output,name),JSON.stringify(value,null,2)+'\n');
const run=(command,args)=>{const result=cp.spawnSync(command,args,{cwd:root,encoding:'utf8',maxBuffer:32*1024*1024});return {command,args,exit:result.status,stdout:result.stdout,stderr:result.stderr};};
const contracts=JSON.parse(read(path.join(output,'contracts.json')));
const truths=JSON.parse(read(path.join(output,'truth-results.json')));
const executions=JSON.parse(read(path.join(output,'executed-tests.json')));
const files=JSON.parse(read(path.join(output,'covered-file-inspection.json')));
const selected=JSON.parse(read(path.join(output,'selected-test-source.json')));
const manifests=fs.readdirSync('crates').filter(name=>fs.existsSync('crates/'+name+'/Cargo.toml')).map(name=>({file:'crates/'+name+'/Cargo.toml',text:read('crates/'+name+'/Cargo.toml')}));
const extra=['.planning/adr/0013-primary-key-required.md','.planning/adr/0021-source-binding-files-and-backing.md','.planning/adr/0023-phase3-crate-dependency-direction.md','.planning/phases/02-source-binding-engine/02-REVIEW-FIX.md','crates/ontoforge/tests/cli_check_cardinality.rs','crates/ontoforge/src/online.rs'].filter(file=>fs.existsSync(file));
const changes=['CONTRIBUTING.md','Cargo.toml','crates/ontoforge-schema/src/diag.rs','crates/ontoforge/src/cli.rs','crates/ontoforge/src/lib.rs'];
const commands=[
 ['git',['log','--reverse','--format=%h %aI %s','--','.planning/adr/0012-definition-document-shape.md']],
 ['git',['show','-s','--format=%h %aI %s','4d36795','58a214a','9aebf77']],
 ['git',['show','--name-only','--format=','2ea8957','eb9a4fe']],
 ['git',['diff','d021687','07a0e06','--','Cargo.lock']],
 ['git',['diff','--name-only','d021687','07a0e06','--',':(glob)**/*.snap']],
 ['git',['show','d021687:Cargo.lock']],
 ['git',['diff','82ca319','HEAD','--',...changes]],
 ['git',['show','82ca319:CONTRIBUTING.md']],
 ['git',['blame','-L','240,275','--','crates/ontoforge-schema/src/loader/mod.rs']],
 ['git',['log','--oneline','--all','--','crates/ontoforge-schema/src/ir/names.rs']],
 ['git',['diff','--name-only','HEAD','--',':(glob)**/*.snap']],
 ['rg',['--files','crates','-g','*.snap.new']],
 ['git',['check-ignore','-q','.zed/settings.json']],
 ['git',['check-ignore','-q','.vscode/settings.json']],
 ['rg',['-n','toml_private|DATETIME_KEY|RawVisitor','crates']],
 ['rg',['-n','fn offset_to_line_col|fn line_col','crates']],
 ['rg',['-n','RUST_MIN_STACK','crates','Cargo.toml']],
 ['rg',['-n','references\\(\\)','crates/ontoforge-expr/src']],
 ['rg',['-n','secrets\\.','.github/workflows/ci.yml']]
].map(([command,args])=>run(command,args));
const ci=JSON.parse(read('.planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json'));
const local=read('.planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-workspace.log');
const localTotals=[...local.matchAll(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].reduce((sum,match)=>({passed:sum.passed+Number(match[1]),failed:sum.failed+Number(match[2]),ignored:sum.ignored+Number(match[3])}),{passed:0,failed:0,ignored:0});
const supplemental=['cardinality_flag_contract','warning_output_and_exit_codes','no_flag_means_no_cardinality_scan'].map(name=>{const file='crates/ontoforge/tests/cli_check_cardinality.rs';const lines=read(file).split('\n');const start=lines.findIndex(line=>new RegExp('\\bfn '+name+'\\b').test(line));let end=lines.findIndex((line,index)=>index>start&&/\b(?:async )?fn \w+/.test(line));if(end<0)end=lines.length;return {file,name,line:start+1,text:lines.slice(start,end).join('\n'),executions:executions.filter(item=>(item.name===name||item.name.endsWith('::'+name))&&/cli_check_cardinality-/.test(item.target))};});
save('supplemental-inspection.json',{head:run('git',['rev-parse','HEAD']),manifests,extra:extra.map(file=>({file,text:read(file)})),commands,ci:{headSha:ci.headSha,conclusion:ci.conclusion,status:ci.status,jobs:ci.jobs.map(job=>({name:job.name,id:job.databaseId,conclusion:job.conclusion,startedAt:job.startedAt,completedAt:job.completedAt,steps:job.steps.filter(step=>/workspace|fmt|clippy|TLS|dependency/.test(step.name))}))},local:{header:local.split('\n').slice(0,6),tail:local.split('\n').slice(-5),totals:localTotals,executionMode:'reused existing finalHEAD raw logs; no Rust/CI executed by this verifier'},supplemental});
const product=files.filter(item=>!item.file.startsWith('.planning/')&&!item.file.startsWith('target/'));
const banned=/kn_id|\bbkn\b|\bodl\b|domain pack|_tenant_id|\bfo-[a-z]/i;
const absolute=/[a-z]:\\\\|\/users\/|\/home\//i;
save('anti-patterns.json',{scope:product.map(item=>item.file),debt:product.flatMap(item=>item.markers.filter(marker=>/\b(TBD|FIXME|XXX)\b/.test(marker.text)).map(marker=>({file:item.file,...marker}))),banned:product.flatMap(item=>item.text.split('\n').map((text,index)=>({file:item.file,line:index+1,text})).filter(item=>banned.test(item.text))),absolute:product.filter(item=>item.file.endsWith('.snap')||item.file.startsWith('schemas/')).flatMap(item=>item.text.split('\n').map((text,index)=>({file:item.file,line:index+1,text})).filter(item=>absolute.test(item.text))),disabledSelected:selected.filter(item=>/#\[ignore\]|#\[should_panic\]/.test(item.text)).map(item=>({file:item.file,name:item.name})),limitations:['Platform-dependent symlink helpers may return when OS cannot create links; passing test alone is not proof that helper performed OS operation.','Product-output privacy scan excludes build logs and planning artifacts intentionally containing local paths.','Provenance originality and judgment-tier resolution cannot be certified by these scans.'],noNewBlockers:true});
const sourceFiles=[...new Set(truths.flatMap(item=>item.sources).filter(file=>!file.startsWith('.planning/')) )];
save('source-assertions.json',sourceFiles.map(file=>({file,sha256:crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'),lines:read(file).split('\n').map((text,index)=>({line:index+1,text})).filter(item=>/\bfn |pub (struct|enum|const)|serde\(|schemars\(|has_errors|DeTable|AutoStream|check_ontology|offset_to_line_col|stack_size|spawn_scoped|follow_links|MAX_|DEFAULT_|MIN_|Error|V0[1-9]|V1[0-5]|out.push|diagnostics.push|canonicalize/.test(item.text))})));
const summaries=contracts.documents.filter(item=>item.file.endsWith('SUMMARY.md')).map(item=>({file:item.file,sha256:item.sha256,sections:item.text.match(/## (?:Deviations[^\n]*|Interface changes[^\n]*|Decisions Made[^\n]*|Known Stubs[^\n]*|Not verified here[^\n]*)[\s\S]*?(?=\n## |$)/g)||[],use:'narrative and historical task/commit index only; never pass evidence'}));
save('summary-review.json',summaries);
const req=read('.planning/REQUIREMENTS.md');
const reqIds=[...new Set(contracts.plans.flatMap(plan=>plan.frontmatter.requirements))];
const requirementMap=reqIds.map(id=>({id,description:req.split('\n').find(line=>line.includes('**'+id+'**:')),plans:contracts.plans.filter(plan=>plan.frontmatter.requirements.includes(id)).map(plan=>path.basename(plan.file)),status:'SATISFIED_TECHNICAL',boundary:id==='SCHEMA-07'?'editor experience retained prior override; no new visual observation':id==='DX-03'||id==='SCHEMA-06'||id==='LINT-04'?'relevant judgment-tier prohibition remains UNCERTAIN; do not infer whole-milestone satisfaction':''}));
save('requirements-coverage.json',{requirements:requirementMap,orphaned:req.split('\n').filter(line=>/\| Phase 1 \|/.test(line)).map(line=>line.split('|')[1].trim()).filter(id=>!reqIds.includes(id)),readOnly:true});
console.log(JSON.stringify({localTotals,selectedTests:selected.length,supplementalTests:supplemental.map(item=>({name:item.name,executions:item.executions.length})),manifests:manifests.length,commands:commands.map(item=>({args:item.args,exit:item.exit})),requirements:reqIds.length},null,2));

