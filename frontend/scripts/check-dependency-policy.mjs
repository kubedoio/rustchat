import fs from 'node:fs'
import path from 'node:path'

const rootDir = process.cwd()

const APPROVED_CATEGORIES = new Set([
  'framework-critical',
  'build-tool-critical',
  'security-critical',
  'editor-format-tooling-critical',
  'impossible-to-replace-economically',
])

function readJson(relativePath) {
  const filePath = path.join(rootDir, relativePath)
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function fail(message) {
  console.error(`frontend dependency policy check failed: ${message}`)
  process.exit(1)
}

function assert(condition, message) {
  if (!condition) {
    fail(message)
  }
}

function sortedKeys(record = {}) {
  return Object.keys(record).sort()
}

function assertSameKeys(actual, expected, label) {
  const actualKeys = sortedKeys(actual)
  const expectedKeys = sortedKeys(expected)

  if (actualKeys.length !== expectedKeys.length) {
    fail(
      `${label} key count mismatch.\nactual: ${actualKeys.join(', ')}\nexpected: ${expectedKeys.join(', ')}`
    )
  }

  for (let index = 0; index < actualKeys.length; index += 1) {
    if (actualKeys[index] !== expectedKeys[index]) {
      fail(
        `${label} key mismatch.\nactual: ${actualKeys.join(', ')}\nexpected: ${expectedKeys.join(', ')}`
      )
    }
  }
}

function assertSameEntries(actual, expected, label) {
  assertSameKeys(actual, expected, label)

  for (const key of sortedKeys(actual)) {
    if (actual[key] !== expected[key]) {
      fail(`${label} version mismatch for ${key}. actual=${actual[key]} expected=${expected[key]}`)
    }
  }
}

const packageJson = readJson('package.json')
const packageLock = readJson('package-lock.json')
const policy = readJson('dependency-policy.json')
const patchManifest = readJson('dependency-patches.json')

assert(policy.packageManager === 'npm', 'packageManager must be "npm" in dependency-policy.json')
assert(
  policy.lockfile === 'package-lock.json',
  'lockfile must be package-lock.json in dependency-policy.json'
)

const forbiddenLockfiles = ['pnpm-lock.yaml', 'yarn.lock']
for (const lockfile of forbiddenLockfiles) {
  assert(!fs.existsSync(path.join(rootDir, lockfile)), `${lockfile} must not exist in frontend/`)
}

const lockRoot = packageLock?.packages?.['']
assert(lockRoot, 'package-lock.json must contain packages[""] metadata')

const packageDependencies = packageJson.dependencies || {}
const packageDevDependencies = packageJson.devDependencies || {}
const lockDependencies = lockRoot.dependencies || {}
const lockDevDependencies = lockRoot.devDependencies || {}

assertSameKeys(policy.directDependencies, packageDependencies, 'direct dependency metadata')
assertSameKeys(
  policy.directDevDependencies,
  packageDevDependencies,
  'direct devDependency metadata'
)
assertSameEntries(lockDependencies, packageDependencies, 'package-lock dependency root')
assertSameEntries(lockDevDependencies, packageDevDependencies, 'package-lock devDependency root')
assertSameKeys(policy.overrides || {}, packageJson.overrides || {}, 'override metadata')

for (const [name, metadata] of Object.entries(policy.directDependencies)) {
  assert(
    APPROVED_CATEGORIES.has(metadata.category),
    `dependency ${name} has invalid category ${metadata.category}`
  )
  assert(
    typeof metadata.reason === 'string' && metadata.reason.trim().length > 0,
    `dependency ${name} must include a non-empty reason`
  )
}

for (const [name, metadata] of Object.entries(policy.directDevDependencies)) {
  assert(
    APPROVED_CATEGORIES.has(metadata.category),
    `devDependency ${name} has invalid category ${metadata.category}`
  )
  assert(
    typeof metadata.reason === 'string' && metadata.reason.trim().length > 0,
    `devDependency ${name} must include a non-empty reason`
  )
}

for (const [name, metadata] of Object.entries(policy.overrides || {})) {
  assert(
    typeof metadata.reason === 'string' && metadata.reason.trim().length > 0,
    `override ${name} must include a non-empty reason`
  )
  assert(
    typeof metadata.removeWhen === 'string' && metadata.removeWhen.trim().length > 0,
    `override ${name} must include a removal condition`
  )
}

const trustedPostInstallSteps = policy.trustedPostInstallSteps || []
assert(Array.isArray(trustedPostInstallSteps), 'trustedPostInstallSteps must be an array')
for (const step of trustedPostInstallSteps) {
  assert(
    typeof step.name === 'string' && step.name.trim().length > 0,
    'trusted post-install steps must include a name'
  )
  assert(
    typeof step.command === 'string' && step.command.trim().length > 0,
    `trusted post-install step ${step.name} must include a command`
  )
  assert(
    typeof step.reason === 'string' && step.reason.trim().length > 0,
    `trusted post-install step ${step.name} must include a reason`
  )
}

assert(Array.isArray(patchManifest.patches), 'dependency-patches.json must expose a patches array')
for (const patch of patchManifest.patches) {
  assert(
    typeof patch.package === 'string' && patch.package.trim().length > 0,
    'every patch entry must include a package name'
  )
  assert(
    typeof patch.patchFile === 'string' && patch.patchFile.trim().length > 0,
    `patch ${patch.package} must include a patchFile`
  )
  assert(
    typeof patch.reason === 'string' && patch.reason.trim().length > 0,
    `patch ${patch.package} must include a reason`
  )
  assert(
    typeof patch.removeWhen === 'string' && patch.removeWhen.trim().length > 0,
    `patch ${patch.package} must include a removal condition`
  )

  const patchPath = path.join(rootDir, patch.patchFile)
  assert(fs.existsSync(patchPath), `patch file does not exist: ${patch.patchFile}`)
}

console.log('frontend dependency policy check passed')
