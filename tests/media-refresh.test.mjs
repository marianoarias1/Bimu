import {test} from 'node:test'
import assert from 'node:assert/strict'
import {singleFlight,sameTrack} from '../src/media-refresh.js'
test('overlapping intervals share a slow sample instead of discarding it forever',async()=>{
 let finish,calls=0;const refresh=singleFlight(()=>{calls++;return new Promise(resolve=>{finish=resolve})})
 const first=refresh();await Promise.resolve();const second=refresh(),third=refresh();assert.equal(calls,1);assert.equal(first,second);assert.equal(first,third)
 finish({title:'Spotify'});assert.deepEqual(await second,{title:'Spotify'});const next=refresh();await Promise.resolve();assert.equal(calls,2);finish(null);await next
})
test('a failed poll releases its lock and subsequent polls can recover',async()=>{
 let calls=0;const refresh=singleFlight(()=>{if(++calls===1)throw Error('temporary');return 'recovered'})
 await assert.rejects(refresh());assert.equal(await refresh(),'recovered');const track={source:'spotify',title:'Song',artist:'Artist'}
 assert.ok(sameTrack(track,{...track,position:3}));assert.ok(!sameTrack(track,{...track,source:'youtube'}))
})
