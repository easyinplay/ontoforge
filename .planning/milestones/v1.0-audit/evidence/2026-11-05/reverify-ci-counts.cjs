const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const dir = __dirname;
const head = '060019fe52005c6b85ae5c5f59da9868f464e579';
const reports = [];
for (const file of fs.readdirSync(dir).filter(file => /^ci-41994479295-.*\.log$/.test(file))) {
  const bytes = fs.readFileSync(path.join(dir,file));
  const lines = bytes.toString('utf8').replace(/^\uFEFF/,'').split(/\r?\n/);
  const steps = [];
  const discovery = [];
  const bench = [];
  const errors = [];
  const python = [];
  let step;
  let pending=[];
  for (let index=0;index<lines.length;index++) {
    const line=lines[index].replace(/\x1b\[[0-?]*[ -/]*[@-~]/g,'').replace(/^\d{4}-\d\d-\d\dT\S+\s+/,'');
    if(line.startsWith('##[group]Run ')) {step={command:line.slice('##[group]Run '.length),line:index+1,targets:[],node:[]};steps.push(step);pending=[];}
    if (/^\s*Running (unittests |tests\/|tests\\)/.test(line)||/^\s*Doc-tests /.test(line)) {
      const target=line.trim();
      const last=pending.at(-1);
      if(!last||last.target!==target||last.announced!==null)pending.push({target,announced:null});
    }
    if (/^running \d+ tests?$/.test(line)&&pending.length) pending[0].announced=+line.match(/\d+/)[0];
    const result=line.match(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/);
    if(result&&step){const target=pending.shift()||{target:null,announced:null};step.targets.push({...target,line:index+1,status:result[1],passed:+result[2],failed:+result[3],ignored:+result[4],measured:+result[5],filtered:+result[6]});}
    if (/^[\w-]+: \d+ (?:ignored )?tests discovered(?: at [a-f0-9]+)?$/.test(line)) discovery.push({line:index+1,text:line,count:+line.match(/: (\d+)/)[1]});
    if (/^[\w_]+: p95=/.test(line)) bench.push({line:index+1,text:line});
    if (/^##\[error\]/.test(line)||/test result: FAILED\./.test(line)) errors.push({line:index+1,text:line});
    if (/^Ran \d+ tests? in /.test(line)) python.push({line:index+1,text:line});
    if (/^(?:# |ℹ )?(tests|pass|fail|cancelled|skipped|todo) \d+$/.test(line)&&step)step.node.push({line:index+1,text:line});
  }
  for(const entry of steps)entry.totals=entry.targets.reduce((total,result)=>{for(const key of ['passed','failed','ignored','filtered'])total[key]+=result[key];return total;},{passed:0,failed:0,ignored:0,filtered:0});
  reports.push({file,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length,raw_ansi_preserved:bytes.includes(Buffer.from([27])),head_present:bytes.toString('utf8').includes(head),lines:lines.length,steps:steps.filter(step=>step.targets.length||step.node.length||/TLS|test_tls|check-tls/.test(step.command)),discovery,bench,python,errors});
}
const report={generated_at:new Date().toISOString(),run:41994479295,head,note:'Per-invocation counts, not unique tests. Workspace, later target reruns, integration and platforms must not be summed as a unique total. Original logs are read-only; timestamps and ANSI removed only in parser memory.',jobs:reports};
fs.writeFileSync(path.join(dir,'reverify-ci-counts.json'),JSON.stringify(report,null,2)+'\n');
for(const job of reports){console.log(`${job.file}: sha256=${job.sha256}; head=${job.head_present}; errors=${job.errors.length}`);for(const step of job.steps)console.log(JSON.stringify({command:step.command,line:step.line,targets:step.targets.length,totals:step.totals,node:step.node}));console.log(JSON.stringify({discovery:job.discovery,bench:job.bench,python:job.python}));}
