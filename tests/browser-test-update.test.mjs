import {test} from 'node:test'
import assert from 'node:assert/strict'
import {isBrowserTestUpdate,isNativeTestUpdate,isWorkflowOnlyUpdate} from '../scripts/browser-test-update.mjs'

const files=['package.json','package-lock.json']
const version=number=>({
  version:'0.55.12',dependencies:{react:'19.0.0'},scripts:{build:'vite build'},
  devDependencies:{'playwright-core':number,vite:'6.4.5'},
})
const lock=number=>({
  lockfileVersion:3,
  packages:{
    '':version(number),
    'node_modules/playwright-core':{version:number,dev:true},
    'node_modules/react':{version:'19.0.0'},
    'node_modules/vite':{version:'6.4.5',dev:true},
  },
})
const check=(changed=files, next=version('1.65.0'), nextLock=lock('1.65.0'))=>
  isBrowserTestUpdate(changed,version('1.58.2'),next,lock('1.58.2'),nextLock)

test('Playwright updates can verify the Windows build without a release bump',()=>{
  assert.equal(check(),true)
  assert.equal(check([...files,'.github/workflows/verify-pull-request.yml']),true)
})

test('the exception does not cover application files, scripts, version changes or other dependencies',()=>{
  assert.equal(check([...files,'src/desktop.jsx']),false)
  assert.equal(check(files,{...version('1.65.0'),version:'0.55.13'}),false)
  assert.equal(check(files,{...version('1.65.0'),scripts:{build:'node other.mjs'}}),false)
  for (const name of ['react','vite','playwright-core']) {
    const modified=lock('1.65.0')
    if(name==='playwright-core') modified.packages['node_modules/'+name].dev=false
    else modified.packages['node_modules/'+name].version='99.0.0'
    assert.equal(check(files,version('1.65.0'),modified),false)
  }
})

test('native test dependency changes are separate from application dependencies',()=>{
  const native=['native-tests/Cargo.toml','native-tests/Cargo.lock']
  assert.equal(isNativeTestUpdate(native),true)
  assert.equal(check(native),false)
  assert.equal(isNativeTestUpdate([...native,'scripts/browser-test-update.mjs','.github/workflows/verify-pull-request.yml']),true)
  assert.equal(isNativeTestUpdate(['native-tests/Cargo.lock']),true)
  for(const production of ['src-tauri/Cargo.toml','src-tauri/Cargo.lock','src-tauri/src/clipboard_protocol.rs','package.json','store-msix/AppxManifest.xml']){
    assert.equal(isNativeTestUpdate([...native,production]),false)
  }
  assert.equal(isNativeTestUpdate([]),false)
  assert.equal(isNativeTestUpdate(['scripts/browser-test-update.mjs']),false)
  assert.equal(isNativeTestUpdate(['native-tests/Cargo.lock','native-tests/../src-tauri/Cargo.toml']),false)
})

test('workflow maintenance still builds installers without releasing a new app version',()=>{
  const workflows=['.github/workflows/verify-pull-request.yml','.github/workflows/security.yml','.github/workflows/msix-experimental.yml','.github/workflows/windows-release.yml','.github/workflows/website.yml']
  assert.equal(isWorkflowOnlyUpdate(workflows),true)
  assert.equal(isWorkflowOnlyUpdate([...workflows,'scripts/browser-test-update.mjs','tests/browser-test-update.test.mjs']),true)
  for(const file of ['src/desktop.jsx','src-tauri/Cargo.toml','src-tauri/Cargo.lock','src-tauri/tauri.conf.json','package.json','package-lock.json','store-msix/AppxManifest.xml','scripts/security-policy.mjs']){
    assert.equal(isWorkflowOnlyUpdate([...workflows,file]),false,file)
  }
  assert.equal(isWorkflowOnlyUpdate([]),false)
  assert.equal(isWorkflowOnlyUpdate(['scripts/browser-test-update.mjs']),false)
})
