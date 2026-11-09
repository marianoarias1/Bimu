import {test} from 'node:test'
import assert from 'node:assert/strict'
import {isBrowserTestUpdate} from '../scripts/browser-test-update.mjs'

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
