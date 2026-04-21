import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..", "..");
const docsRoot = path.resolve(repoRoot, "docs");

const excludedDirectories = new Set(["archive", "plans", "node_modules", ".vitepress", "scripts"]);
const excludedFiles = new Set([
  "DOCUMENTATION_REORGANIZATION_PLAN.md",
  "REORGANIZATION_SUMMARY.md",
]);

function isExternalTarget(target) {
  return (
    target.startsWith("http://") ||
    target.startsWith("https://") ||
    target.startsWith("mailto:") ||
    target.startsWith("tel:") ||
    target.startsWith("data:") ||
    target.startsWith("javascript:")
  );
}

function collectMarkdownFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (excludedDirectories.has(entry.name)) {
        continue;
      }
      files.push(...collectMarkdownFiles(path.join(dir, entry.name)));
      continue;
    }
    if (!entry.name.endsWith(".md")) {
      continue;
    }
    if (excludedFiles.has(entry.name)) {
      continue;
    }
    files.push(path.join(dir, entry.name));
  }
  return files;
}

function pickLinkTarget(rawTarget) {
  const trimmed = rawTarget.trim();
  if (trimmed.startsWith("<") && trimmed.includes(">")) {
    return trimmed.slice(1, trimmed.indexOf(">"));
  }
  return trimmed.split(/\s+/)[0];
}

function checkResolvedPath(absPath, treatAsDirectoryHint) {
  if (fs.existsSync(absPath)) {
    const stats = fs.statSync(absPath);
    if (!stats.isDirectory()) {
      return true;
    }
    return (
      fs.existsSync(path.join(absPath, "README.md")) ||
      fs.existsSync(path.join(absPath, "index.md"))
    );
  }

  if (treatAsDirectoryHint) {
    return (
      fs.existsSync(path.join(absPath, "README.md")) ||
      fs.existsSync(path.join(absPath, "index.md"))
    );
  }

  if (!path.extname(absPath)) {
    return (
      fs.existsSync(`${absPath}.md`) ||
      fs.existsSync(path.join(absPath, "README.md")) ||
      fs.existsSync(path.join(absPath, "index.md"))
    );
  }

  return false;
}

function resolveLinkPath(sourceFile, linkTarget) {
  const noHash = linkTarget.split("#")[0];
  const noQuery = noHash.split("?")[0];
  if (!noQuery) {
    return null;
  }

  const decodedTarget = decodeURI(noQuery);
  const treatAsDirectoryHint = decodedTarget.endsWith("/");
  const relativeTarget = treatAsDirectoryHint ? decodedTarget.slice(0, -1) : decodedTarget;

  if (relativeTarget.startsWith("/")) {
    return {
      // VitePress absolute links are rooted at docs/
      absPath: path.resolve(docsRoot, relativeTarget.slice(1)),
      treatAsDirectoryHint,
    };
  }

  return {
    absPath: path.resolve(path.dirname(sourceFile), relativeTarget),
    treatAsDirectoryHint,
  };
}

const filesToCheck = [
  path.resolve(repoRoot, "README.md"),
  ...collectMarkdownFiles(docsRoot),
];

const markdownLinkRegex = /!?\[[^\]]*?\]\(([^)]+)\)/g;
const errors = [];

for (const filePath of filesToCheck) {
  const content = fs.readFileSync(filePath, "utf8");
  let match;
  while ((match = markdownLinkRegex.exec(content)) !== null) {
    const rawTarget = match[1];
    const target = pickLinkTarget(rawTarget);
    if (!target || target === "url" || target.startsWith("#") || isExternalTarget(target)) {
      continue;
    }

    const resolved = resolveLinkPath(filePath, target);
    if (!resolved) {
      continue;
    }

    if (!checkResolvedPath(resolved.absPath, resolved.treatAsDirectoryHint)) {
      errors.push({
        file: path.relative(repoRoot, filePath),
        target,
      });
    }
  }
}

if (errors.length > 0) {
  console.error("Broken internal markdown links detected:");
  for (const error of errors) {
    console.error(`- ${error.file} -> ${error.target}`);
  }
  process.exit(1);
}

console.log(`Internal markdown link check passed (${filesToCheck.length} files scanned).`);
