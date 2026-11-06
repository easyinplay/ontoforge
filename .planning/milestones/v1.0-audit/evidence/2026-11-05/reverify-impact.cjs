const fs = require('node:fs');
const cp = require('node:child_process');
const git = (...args) => cp.execFileSync('git', args, {encoding: 'utf8'}).trim();
const head = git('rev-parse', 'HEAD');
const phases = [];
for (const dir of fs.readdirSync('.planning/phases')) {
  if (!/^0[1-7]-/.test(dir)) continue;
  const report = `.planning/phases/${dir}/${dir.slice(0, 2)}-VERIFICATION.md`;
  const text = fs.readFileSync(report, 'utf8');
  const header = text.split('---')[1];
  const covered = JSON.parse(header.match(/^covered_files: (.+)$/m)[1]);
  const identityFields = Object.fromEntries([...header.matchAll(/^([\w]*(?:commit|head|sha)[\w]*):\s*["']?([a-f0-9]{7,40})["']?\s*$/gm)].map(match => [match[1], match[2]]));
  const baseline = identityFields.verified_source_commit || identityFields.verified_head || identityFields.source_head || identityFields.delivery_head || identityFields.source_commit || identityFields.ci_head_sha || (text.includes('f8851ca073fed34e80e605f0ac91880259df7e12') ? 'f8851ca073fed34e80e605f0ac91880259df7e12' : null);
  if (!baseline) throw Error(`No verified baseline for ${report}`);
  const changes = git('diff', '--name-status', baseline, head, '--', ...covered).split('\n').filter(Boolean);
  const implementationChanges = changes.filter(line => !line.split('\t').at(-1).startsWith('.planning/'));
  phases.push({phase: dir, report, previous_status: header.match(/^status: (.+)$/m)[1], identity_fields: identityFields, baseline, baseline_evidence: Object.keys(identityFields).length ? 'canonical frontmatter (delivery/source/CI fields preferred over implementation origin)' : 'canonical body explicitly identifies verified HEAD', covered_files: covered, covered_changes: changes, implementation_changes: implementationChanges});
}
const result = {created_at: new Date().toISOString(), head, comparison_8d9d8c6: git('diff', '--name-status', '8d9d8c6', head, '--', 'crates', 'ci', 'clients', 'Cargo.toml', 'Cargo.lock', '.github'), phases};
fs.writeFileSync(`${__dirname}/reverify-impact.json`, JSON.stringify(result, null, 2) + '\n');
for (const phase of phases) console.log(`${phase.phase}: ${phase.baseline}; ${phase.implementation_changes.length} covered implementation changes\n${phase.implementation_changes.join('\n')}`);
