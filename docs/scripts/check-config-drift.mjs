import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..", "..");
const docsRoot = path.resolve(repoRoot, "docs");

const excludedDirectories = new Set([
  "archive",
  "audits",
  "internal",
  "node_modules",
  ".vitepress",
  "plans",
  "scripts",
]);

const publicMarkdownFiles = [
  path.resolve(repoRoot, "README.md"),
  ...collectMarkdownFiles(docsRoot),
];

const deploymentFiles = [
  path.resolve(repoRoot, ".env.example"),
  ...fs
    .readdirSync(repoRoot)
    .filter((name) => /^docker-compose.*\.ya?ml$/.test(name))
    .map((name) => path.resolve(repoRoot, name)),
];

const rules = [
  {
    pattern: /\bRUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN\b/,
    message:
      "Query-token WebSocket authentication has been removed; do not document or template RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN.",
  },
  {
    pattern: /\bWS_ALLOW_QUERY_TOKEN\b/,
    message:
      "Query-token WebSocket authentication has been removed; avoid shorthand references that imply it is configurable.",
  },
  {
    pattern: /\bRUSTCHAT_S3_PUBLIC_URL\b/,
    message: "Use RUSTCHAT_S3_PUBLIC_ENDPOINT instead of the retired S3 public URL name.",
  },
  {
    pattern: /\bRUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY\s*=\s*(query|header)\b/i,
    message: "OAuth token delivery only supports cookie exchange mode.",
  },
];

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

    if (entry.name.endsWith(".md")) {
      files.push(path.join(dir, entry.name));
    }
  }
  return files;
}

function existingFiles(files) {
  return files.filter((filePath) => fs.existsSync(filePath));
}

const filesToCheck = existingFiles([...publicMarkdownFiles, ...deploymentFiles]);
const errors = [];

for (const filePath of filesToCheck) {
  const content = fs.readFileSync(filePath, "utf8");
  for (const rule of rules) {
    const match = rule.pattern.exec(content);
    if (!match) {
      continue;
    }

    const beforeMatch = content.slice(0, match.index);
    const line = beforeMatch.split("\n").length;
    errors.push({
      file: path.relative(repoRoot, filePath),
      line,
      message: rule.message,
      value: match[0],
    });
  }
}

if (errors.length > 0) {
  console.error("Configuration documentation drift detected:");
  for (const error of errors) {
    console.error(`- ${error.file}:${error.line}: ${error.message} (${error.value})`);
  }
  process.exit(1);
}

console.log(`Configuration drift check passed (${filesToCheck.length} files scanned).`);
