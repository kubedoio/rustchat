import fs from 'node:fs'
import path from 'node:path'
import { execFileSync } from 'node:child_process'

const rootDir = process.cwd()
const manifestPath = path.join(rootDir, 'dependency-patches.json')

function fail(message) {
  console.error(`frontend dependency patch apply failed: ${message}`)
  process.exit(1)
}

if (!fs.existsSync(manifestPath)) {
  fail('dependency-patches.json is missing')
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'))
const patches = manifest.patches || []

if (!Array.isArray(patches)) {
  fail('dependency-patches.json must contain a patches array')
}

if (patches.length === 0) {
  console.log('no frontend dependency patches configured')
  process.exit(0)
}

for (const patch of patches) {
  if (!patch.package || !patch.patchFile) {
    fail('every patch entry must include package and patchFile')
  }

  const patchPath = path.join(rootDir, patch.patchFile)
  const packageDir = path.join('node_modules', patch.package)
  const packageDirAbsolute = path.join(rootDir, packageDir)

  if (!fs.existsSync(patchPath)) {
    fail(`patch file does not exist: ${patch.patchFile}`)
  }

  if (!fs.existsSync(packageDirAbsolute)) {
    fail(`target package directory does not exist: node_modules/${patch.package}`)
  }

  console.log(`applying dependency patch for ${patch.package} from ${patch.patchFile}`)
  execFileSync('git', ['apply', '--directory', packageDir, '--whitespace=nowarn', patchPath], {
    cwd: rootDir,
    stdio: 'inherit',
  })
}

console.log('frontend dependency patches applied successfully')
