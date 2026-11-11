import {test} from 'node:test'
import assert from 'node:assert/strict'
import {normalizePages,togglePage,movePage} from '../src/page-preferences.js'
test('restores saved order, ignores unknown and duplicate pages and adds new pages',()=>{
 const pages=normalizePages([{id:'weather',enabled:true},{id:'music',enabled:false},{id:'weather',enabled:true},{id:'invalid',enabled:true}])
 assert.deepEqual(pages.map(p=>p.id),['weather','music','shelf','modes','clipboard','cleanup']);assert.equal(pages[1].enabled,false)
})
test('never disables the last page and corrects invalid all-hidden preferences',()=>{
 let pages=normalizePages(null);for(const p of pages.slice(1))pages=togglePage(pages,p.id)
 assert.equal(togglePage(pages,pages[0].id).filter(p=>p.enabled).length,1)
 assert.ok(normalizePages(pages.map(p=>({...p,enabled:false}))).some(p=>p.enabled))
})
test('reorders without losing preferences',()=>{
 const p=normalizePages(null),next=movePage(p,'weather','music');assert.equal(next[0].id,'weather');assert.equal(next.length,p.length);assert.deepEqual(movePage(p,'missing','music'),p)
})
test('moves pages down and up to the selected position',()=>{
 const pages=normalizePages(null)
 assert.deepEqual(movePage(pages,'music','shelf').map(p=>p.id),['shelf','music','modes','weather','clipboard','cleanup'])
 assert.deepEqual(movePage(pages,'music','weather').map(p=>p.id),['shelf','modes','weather','music','clipboard','cleanup'])
 assert.deepEqual(movePage(pages,'weather','shelf').map(p=>p.id),['music','weather','shelf','modes','clipboard','cleanup'])
})
