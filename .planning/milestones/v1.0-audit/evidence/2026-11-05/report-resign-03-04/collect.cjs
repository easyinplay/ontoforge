const fs = require("node:fs");
const path = require("node:path");
const cp = require("node:child_process");
const crypto = require("node:crypto");
const root = process.cwd();
const output = __dirname;
const tool = "C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs";
function query(args) {
  const result = cp.spawnSync(process.execPath, [tool, "query", ...args], { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  let data;
  try { data = JSON.parse(result.stdout); } catch { data = result.stdout; }
  return { args, exit: result.status, data, stderr: result.stderr };
}
function save(name, data) { fs.writeFileSync(path.join(output, name), JSON.stringify(data, null, 2) + "\n"); }
const result = [];
for (const phase of ["03-edit-store-scope-model", "04-scoped-l0-read"]) {
  const dir = ".planning/phases/" + phase;
  const report = dir + "/" + phase.slice(0, 2) + "-VERIFICATION.md";
  const snapshot = path.join(output, phase.slice(0, 2) + "-VERIFICATION.before.md");
  if (!fs.existsSync(snapshot)) fs.copyFileSync(report, snapshot);
  const old = query(["frontmatter.get", snapshot]);
  const plans = fs.readdirSync(dir).filter(name => /^\d.*-PLAN.md$/.test(name)).map(name => {
    const file = dir + "/" + name;
    return { file, frontmatter: query(["frontmatter.get", file]), artifacts: query(["verify.artifacts", file]), links: query(["verify.key-links", file]) };
  });
  const item = { phase, report, old, plans, roadmap: query(["roadmap.get-phase", phase.slice(0, 2)]), before: query(["verification.status", dir]), decisions: query(["check.decision-coverage-verify", dir, dir + "/" + phase.slice(0, 2) + "-CONTEXT.md"]) };
  save(phase.slice(0, 2) + "-contracts.json", item);
  result.push({ phase, plans: plans.length, truths: plans.reduce((sum, plan) => sum + (plan.frontmatter.data.must_haves?.truths?.length || 0), 0), prohibitions: plans.reduce((sum, plan) => sum + (plan.frontmatter.data.must_haves?.prohibitions?.length || 0), 0), covered: old.data.covered_files?.length, before: item.before.data });
}
const evidenceDir = path.dirname(output);
const files = fs.readdirSync(evidenceDir).filter(name => /^ci-41994479295-.*\.(json|log)$/.test(name));
save("evidence-inputs.json", files.map(name => { const bytes = fs.readFileSync(path.join(evidenceDir, name)); return { path: path.relative(root, path.join(evidenceDir, name)).replaceAll("\\", "/"), bytes: bytes.length, sha256: crypto.createHash("sha256").update(bytes).digest("hex") }; }));
save("collection-summary.json", result);
console.log(JSON.stringify(result, null, 2));
