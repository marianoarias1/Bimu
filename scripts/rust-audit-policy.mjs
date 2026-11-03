import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath} from 'node:url'

// Only one reviewed, time-bounded, target-inapplicable advisory may be scoped out.
// Vulnerabilities and yanked packages remain blocking for the complete lockfiles.
export function checkRustAudit(report, metadata, exclusions, now = new Date()) {
  if (!(report?.database?.['advisory-count'] > 0) ||
      !/^[0-9a-f]{40}$/.test(report?.database?.['last-commit'] ?? '') ||
      !(report?.lockfile?.['dependency-count'] > 0) ||
      !Array.isArray(report?.settings?.ignore) || report.settings.ignore.length ||
      !Array.isArray(report.settings.target_arch) || report.settings.target_arch.length ||
      !Array.isArray(report.settings.target_os) || report.settings.target_os.length ||
      !Array.isArray(report?.vulnerabilities?.list) ||
      report.vulnerabilities.count !== report.vulnerabilities.list.length ||
      !report.warnings || typeof report.warnings !== 'object') throw Error('Incomplete RustSec report')
  const nodes = metadata?.resolve?.nodes
  if (!Array.isArray(nodes) || !Array.isArray(metadata.packages) ||
      !nodes.some(n => n.id === metadata.resolve.root) ||
      !metadata.packages.some(p => p.id === metadata.resolve.root && p.name === 'mascota-companero')) throw Error('Missing Windows dependency evidence')
  const ids = new Set(nodes.map(n => n.id))
  const applicable = new Set(metadata.packages.filter(p => ids.has(p.id)).map(p => `${p.name}@${p.version}`))
  const blockers = report.vulnerabilities.list.map(v => v.advisory.id)
  const scoped = []
  for (const kind of ['unsound', 'yanked']) {
    const entries = report.warnings[kind] ?? []
    if (!Array.isArray(entries)) throw Error('Malformed RustSec warnings')
    for (const entry of entries) {
      const pkg = entry.package
      if (!pkg?.name || !pkg.version) throw Error('Missing advisory package')
      const exception = exclusions.find(e => kind === 'unsound' &&
        e.id === entry.advisory?.id && e.package === pkg.name && e.version === pkg.version &&
        e.target === 'x86_64-pc-windows-msvc' && e.owner && e.reason &&
        Number.isFinite(Date.parse(e.until)) && now.getTime() < Date.parse(e.until) &&
        !applicable.has(`${pkg.name}@${pkg.version}`))
      if (exception) scoped.push(`${exception.id}: ${pkg.name}@${pkg.version}, absent from Windows; review before ${exception.until}`)
      else blockers.push(`${entry.advisory?.id ?? kind}: ${pkg.name}@${pkg.version}`)
    }
  }
  if (blockers.length) throw Error(`RustSec blockers: ${blockers.join('; ')}`)
  return scoped
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const read = path => JSON.parse(readFileSync(path, 'utf8'))
  const metadata = read('rust-metadata-windows.json')
  const exclusions = read('security/rust-target-exclusions.json')
  for (const path of ['rust-audit-app.json', 'rust-audit-tests.json']) {
    const scoped = checkRustAudit(read(path), metadata, exclusions)
    const status = Number(process.env[path.includes('-app.') ? 'RUST_AUDIT_APP_EXIT' : 'RUST_AUDIT_TESTS_EXIT'])
    if (![0, 1].includes(status) || (status !== 0 && scoped.length === 0)) throw Error(`${path}: scanner failed without an explained target exclusion`)
    console.log(`${path}: no blocking findings`)
    for (const note of scoped) console.log(`Target scope: ${note}`)
  }
}
