// Lightweight configuration/secret-pattern guard; not a replacement for a
// dedicated secret scanner, SAST, dependency audits or an independent pentest.
import {readFileSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import assert from 'node:assert/strict'
const config=JSON.parse(readFileSync('src-tauri/tauri.conf.json','utf8'))
const csp=new Map(config.app.security.csp.split(';').map(v=>v.trim().split(/\s+/)).filter(v=>v[0]).map(([name,...values])=>[name,values]))
for(const name of ['object-src','base-uri','form-action','frame-ancestors'])assert.deepEqual(csp.get(name),["'none'"],name+' must be disabled')
assert.deepEqual(csp.get('script-src'),["'self'"],'only bundled scripts')
assert.ok(!config.app.security.dangerousDisableAssetCspModification,'keep Tauri CSP injection')
assert.ok(!config.app.security.dangerousRemoteDomainIpcAccess?.length,'no remote IPC')
const capability=JSON.parse(readFileSync('src-tauri/capabilities/default.json','utf8'))
assert.deepEqual(capability.windows,['main']);assert.ok(!capability.remote,'local webview only')
for(const permission of capability.permissions)assert.ok(typeof permission==='string'&&!/^(shell|fs|http|opener):/.test(permission),'no generic filesystem, HTTP or shell IPC permissions')
const files=execFileSync('git',['ls-files','-z'],{encoding:'utf8'}).split('\0').filter(Boolean)
let scanned=0
for(const file of files){
 const bytes=readFileSync(file)
 if(bytes.length>2_000_000||bytes.subarray(0,8000).includes(0))continue
 const text=bytes.toString('utf8');scanned++
 for(const [kind,pattern]of [
  ['private signing key',/-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
  ['GitHub credential',/\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{60,})\b/],
  ['AWS access identifier',/\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/]
 ])assert.ok(!pattern.test(text),`${kind} pattern in ${file}; inspect locally without logging the value`)
 if(file.startsWith('.github/workflows/')&&/\.ya?ml$/.test(file)){
  assert.ok(!/^\s*pull_request_target:/m.test(text),'avoid privileged untrusted PR execution')
  for(const match of text.matchAll(/^\s*- uses:\s*(\S+)/gm))assert.ok(/^[-\w]+\/[-\w./]+@[0-9a-f]{40}$/.test(match[1]),'pin third-party action: '+file)
 }
}
console.log(`Security policy OK; ${scanned} bounded text files checked. Secret values are never printed.`)
