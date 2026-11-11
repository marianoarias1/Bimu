import {execFileSync} from 'node:child_process'
import {pathToFileURL} from 'node:url'
import {isDeepStrictEqual} from 'node:util'

const dependency = 'playwright-core'
const packagePath = 'node_modules/playwright-core'
const allowed = new Set([
  'package.json',
  'package-lock.json',
  '.github/workflows/verify-pull-request.yml',
  '.github/workflows/windows-release.yml',
  'scripts/browser-test-update.mjs',
  'tests/browser-test-update.test.mjs',
])

function removeBrowserTool(manifest) {
  const copy = structuredClone(manifest)
  delete copy.devDependencies[dependency]
  return copy
}

function removeBrowserLockEntry(lock) {
  const copy = structuredClone(lock)
  copy.packages[''] = removeBrowserTool(copy.packages[''])
  delete copy.packages[packagePath]
  return copy
}

export function isBrowserTestUpdate(files, before, after, oldLock, newLock) {
  if (!files.includes('package.json') || !files.includes('package-lock.json')
      || files.some(file => !allowed.has(file))) return false
  for (const [manifest, lock] of [[before, oldLock], [after, newLock]]) {
    const version = manifest.devDependencies?.[dependency]
    const entry = lock.packages?.[packagePath]
    if (!version || manifest.dependencies?.[dependency] || entry?.dev !== true
        || lock.packages?.['']?.devDependencies?.[dependency] !== version) return false
  }
  return before.devDependencies[dependency] !== after.devDependencies[dependency]
    && isDeepStrictEqual(removeBrowserTool(before), removeBrowserTool(after))
    && isDeepStrictEqual(removeBrowserLockEntry(oldLock), removeBrowserLockEntry(newLock))
}

const nativeManifests = new Set(['native-tests/Cargo.toml', 'native-tests/Cargo.lock'])
const nativeAllowed = new Set([
  ...nativeManifests,
  '.github/workflows/verify-pull-request.yml',
  'scripts/browser-test-update.mjs',
  'tests/browser-test-update.test.mjs',
])

export function isNativeTestUpdate(files) {
  return files.some(file => nativeManifests.has(file))
    && files.every(file => nativeAllowed.has(file))
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const base = process.argv[2]
  if (!base || !/^[\da-f]{40}$/i.test(base)) throw new Error('Expected the pull request base commit SHA')
  const git = (...args) => execFileSync('git', args, {encoding:'utf8'})
  const files = git('diff','--name-only','-z',base,'HEAD','--').split('\0').filter(Boolean)
  if (isNativeTestUpdate(files)) {
    console.log(true)
  } else if (!files.includes('package.json') || !files.includes('package-lock.json') || files.some(file => !allowed.has(file))) {
    console.log(false)
  } else {
    const read = (ref, file) => JSON.parse(git('show',ref+':'+file))
    console.log(isBrowserTestUpdate(files,read(base,'package.json'),read('HEAD','package.json'),
      read(base,'package-lock.json'),read('HEAD','package-lock.json')))
  }
}
