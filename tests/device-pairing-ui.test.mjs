import {test} from 'node:test'
import assert from 'node:assert/strict'
import {pairingView,canPair,remainingCodeTime,codeTimeLabel,limitSummary,validTransferLimit} from '../src/device-pairing-ui.js'

test('each pairing state has one screen and a saved link never becomes a new-pairing form',()=>{
 assert.equal(pairingView({phase:'idle'},'start'),'start')
 assert.equal(pairingView({phase:'idle'},'join'),'join')
 assert.equal(pairingView({phase:'waiting'},'join'),'code')
 assert.equal(pairingView({phase:'connecting'},'join'),'connecting')
 assert.equal(pairingView({phase:'connected',linked:true},'join'),'connected')
 for(const phase of ['idle','error','connecting'])for(const route of ['start','join','host'])assert.equal(pairingView({phase,linked:true},route),'reconnecting')
 assert.equal(pairingView({phase:'error'},'host'),'create-error')
})
test('joining requires an available chosen device or explicit manual address, and exactly six digits',()=>{
 const peers=[{name:'Desktop',address:'192.168.1.10:12345'}]
 assert.ok(canPair(peers[0].address,'000123',peers))
 assert.ok(!canPair('192.168.1.11:12345','000123',peers))
 assert.ok(canPair('192.168.1.11:12345','000123',peers,true))
 for(const code of ['12345','1234567','abcdef','12 345'])assert.ok(!canPair(peers[0].address,code,peers,true))
 assert.ok(!canPair('  ','123456',peers,true))
})
test('code expiry uses native deadline, never restarts after rerender, and cannot go negative',()=>{
 assert.equal(remainingCodeTime(120,18000),102)
 assert.equal(codeTimeLabel(remainingCodeTime(120,18000)),'Vence en 1:42')
 assert.equal(remainingCodeTime(120,119500),1)
 assert.equal(remainingCodeTime(120,120001),0)
 assert.equal(codeTimeLabel(0),'Código vencido')
 assert.equal(remainingCodeTime(undefined),null)
})
test('secondary shows inherited active 250 without replacing its saved 30',()=>{
 const status={phase:'connected',usesMainLimit:true,isMain:false,peer:'Desktop',effectiveLimitMb:250,transferLimitMb:30}
 const summary=limitSummary(status)
 assert.equal(summary.value,250);assert.equal(summary.editLabel,'Mi límite')
 assert.equal(summary.source,'Definido por Desktop');assert.ok(summary.inherited)
 assert.equal(status.transferLimitMb,30)
})
test('main override, independent limits and disconnected preferences remain distinct',()=>{
 assert.equal(limitSummary({phase:'connected',usesMainLimit:true,isMain:true,effectiveLimitMb:250,transferLimitMb:250}).source,'Se aplica a ambos equipos.')
 assert.equal(limitSummary({phase:'connected',usesMainLimit:false,effectiveLimitMb:70,transferLimitMb:70}).value,70)
 assert.equal(limitSummary({phase:'idle',effectiveLimitMb:250,transferLimitMb:30}).value,30)
 assert.equal(limitSummary({phase:'idle'}).value,30)
 for(const value of ['1','30','250'])assert.ok(validTransferLimit(value))
 for(const value of ['0','251','2.5','NaN',''])assert.ok(!validTransferLimit(value))
})
