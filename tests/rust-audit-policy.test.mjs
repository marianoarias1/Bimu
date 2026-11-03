import test from 'node:test'
import assert from 'node:assert/strict'
import {checkRustAudit} from '../scripts/rust-audit-policy.mjs'
const metadata = {packages:[{id:'root',name:'mascota-companero',version:'1'}],resolve:{root:'root',nodes:[{id:'root'}]}}
const report = () => ({database:{'advisory-count':1,'last-commit':'a'.repeat(40)},lockfile:{'dependency-count':1},settings:{ignore:[],target_arch:[],target_os:[]},vulnerabilities:{count:0,list:[]},warnings:{}})
const entry = () => ({package:{name:'glib',version:'0.18.5'},advisory:{id:'RUSTSEC-2024-0429'}})
const exclusions = [{id:'RUSTSEC-2024-0429',package:'glib',version:'0.18.5',target:'x86_64-pc-windows-msvc',owner:'maintainer',reason:'not built',until:'2026-12-03T00:00:00Z'}]
const date = new Date('2026-11-03T00:00:00Z')
test('accepts clean audit and records only the reviewed absent-target advisory',()=>{
 assert.deepEqual(checkRustAudit(report(),metadata,exclusions,date),[])
 const r=report();r.warnings.unsound=[entry()];assert.equal(checkRustAudit(r,metadata,exclusions,date).length,1)
})
test('blocks the reviewed dependency if it enters Windows',()=>{
 const m=structuredClone(metadata);m.packages.push({id:'glib',name:'glib',version:'0.18.5'});m.resolve.nodes.push({id:'glib'})
 const r=report();r.warnings.unsound=[entry()];assert.throws(()=>checkRustAudit(r,m,exclusions,date),/blockers/)
})
test('blocks expired scope exclusions and unknown advisories or versions',()=>{
 const r=report();r.warnings.unsound=[entry()]
 assert.throws(()=>checkRustAudit(r,metadata,exclusions,new Date('2026-12-03T00:00:00Z')),/blockers/)
 r.warnings.unsound[0].advisory.id='RUSTSEC-NEW';assert.throws(()=>checkRustAudit(r,metadata,exclusions,date),/blockers/)
 r.warnings.unsound=[entry()];r.warnings.unsound[0].package.version='0.18.6';assert.throws(()=>checkRustAudit(r,metadata,exclusions,date),/blockers/)
})
test('never exempts vulnerabilities or yanked releases',()=>{
 const r=report();r.vulnerabilities={count:1,list:[entry()]};assert.throws(()=>checkRustAudit(r,metadata,exclusions,date),/blockers/)
 const y=report();y.warnings.yanked=[entry()];assert.throws(()=>checkRustAudit(y,metadata,exclusions,date),/blockers/)
})
test('fails closed without valid report and target evidence',()=>{
 assert.throws(()=>checkRustAudit({},metadata,exclusions,date),/Incomplete/)
 assert.throws(()=>checkRustAudit(report(),{},exclusions,date),/evidence/)
 const ignored=report();ignored.settings.ignore=['RUSTSEC-NEW'];assert.throws(()=>checkRustAudit(ignored,metadata,exclusions,date),/Incomplete/)
 const r=report();r.warnings.unsound={};assert.throws(()=>checkRustAudit(r,metadata,exclusions,date),/Malformed/)
})
