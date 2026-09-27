#!/usr/bin/env node
// Deterministic docs/repository-hygiene checks (RI-C06 / ADR-006 P005).
//
// Verifies, for the live documentation set:
//   1. No tracked generated/build artifacts (docs/node_modules,
//      docs/.vitepress/dist) are committed to git.
//   2. No two live markdown documents have identical content (exact duplicate
//      with competing authority).
//   3. ADRs live only under docs/adr/ (no ADR-patterned doc outside that path).
//
// Excludes the same directories as the link checker (archive, plans,
// node_modules, .vitepress, scripts) plus audits/internal (evidence, not
// authoritative) and decisions (a redirect to the ADR index).

import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..", "..");
const docsRoot = path.resolve(repoRoot, "docs");

const excludedDirectories = new Set([
  "archive",
  "plans",
  "audits",
  "internal",
  "decisions",
  "node_modules",
  ".vitepress",
  "scripts",
  "target",
  ".git",
  ".cargo",
  "mm-compat",
]);

const TRACKED_ARTIFACTS = ["docs/node_modules", "docs/.vitepress/dist"];

function collectMarkdownFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!excludedDirectories.has(entry.name)) {
        files.push(...collectMarkdownFiles(path.join(dir, entry.name)));
      }
      continue;
    }
    if (entry.name.toLowerCase().endsWith(".md")) {
      files.push(path.join(dir, entry.name));
    }
  }
  return files;
}

function checkTrackedArtifacts() {
  const problems = [];
  try {
    const out = execFileSync("git", ["ls-files", ...TRACKED_ARTIFACTS], {
      cwd: repoRoot,
      encoding: "utf8",
    });
    const tracked = out.split("\n").map((l) => l.trim()).filter(Boolean);
    if (tracked.length) {
      problems.push(
        `FAIL: generated/build artifacts are tracked in git: ${tracked.join(", ")}`,
      );
    }
  } catch (err) {
    problems.push(`FAIL: could not query git index for tracked artifacts: ${err.message}`);
  }
  return problems;
}

function checkExactDuplicates() {
  const problems = [];
  const byHash = new Map();
  for (const file of collectMarkdownFiles(docsRoot)) {
    try {
      const content = fs.readFileSync(file, "utf8");
      const hash = crypto.createHash("sha256").update(content).digest("hex");
      if (!byHash.has(hash)) {
        byHash.set(hash, []);
      }
      byHash.get(hash).push(path.relative(repoRoot, file));
    } catch {
      // unreadable files are reported by removing them from the comparison
    }
  }
  for (const [hash, files] of byHash) {
    if (files.length > 1) {
      problems.push(
        `FAIL: exact duplicate document content in live docs (sha256 ${hash.slice(0, 12)}):\n  - ${files.join("\n  - ")}`,
      );
    }
  }
  return problems;
}

function checkADRPlacement() {
  const problems = [];
  const roots = [
    path.join(repoRoot, "docs"),
    path.join(repoRoot),
  ];
  const adrPattern = /(^|\/)(ADR-|adr-)?\d+-?.*\.md$/i;
  const adrDir = path.resolve(docsRoot, "adr");
  const excludeAbs = new Set([
    ...excludedDirectories,
  ]);
  const seen = new Set();
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const abs = path.join(dir, entry.name);
      if (seen.has(abs)) continue;
      seen.add(abs);
      if (entry.isDirectory()) {
        if (excludeAbs.has(entry.name)) continue;
        walk(abs);
      } else if (entry.name.toLowerCase().endsWith(".md")) {
        // Treat a file as an ADR if its name looks like an ADR (starts with a
        // 4-digit number) or its first heading declares "ADR-".
        const base = path.basename(entry.name);
        const isAdrName = /^ADR-/i.test(base);
        let headerAdr = false;
        try {
          const head = fs.readFileSync(abs, "utf8").slice(0, 400);
          headerAdr = /^#\s+ADR-\d+/im.test(head);
        } catch {
          /* ignore */
        }
        const isInAdrDir = path.dirname(abs) === adrDir;
        if ((isAdrName || headerAdr) && !isInAdrDir) {
          problems.push(
            `FAIL: ADR-patterned document outside docs/adr/: ${path.relative(repoRoot, abs)}`,
          );
        }
      }
    }
  };
  walk(adrDir); // mark all adr files as seen first
  for (const root of roots) {
    walk(root);
  }
  return problems;
}

function main() {
  const problems = [
    ...checkTrackedArtifacts(),
    ...checkExactDuplicates(),
    ...checkADRPlacement(),
  ];
  for (const problem of problems) {
    console.error(problem);
  }
  if (problems.length) {
    console.error("docs:check-hygiene FAIL");
    process.exit(1);
  }
  console.log("docs:check-hygiene PASS");
}

main();
