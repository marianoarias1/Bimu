import {test} from 'node:test'
import assert from 'node:assert/strict'
import {parseSyncedLyrics,lyricIndex} from '../src/lyrics.js'
test('extracts and sorts multiple timestamps and ignores metadata',()=>{
 const lines=parseSyncedLyrics('[ar:Artist]\n[00:03.50][00:01.25]Hello\n[00:05.100]World\n[00:80.00]bad')
 assert.deepEqual(lines,[{time:1.25,text:'Hello'},{time:3.5,text:'Hello'},{time:5.1,text:'World'}])
 assert.equal(lyricIndex(lines,0),-1);assert.equal(lyricIndex(lines,3.7),1);assert.equal(lyricIndex(lines,9),2)
})

import {lyricsQuery,lyricsFailure,createLyricsLoader} from '../src/lyrics-request.js'
test('YouTube decoration is removed without hiding live/remix identity',()=>{
 assert.deepEqual(lyricsQuery('Milo J - Rara Vez (Official Video)','Milo J - Topic',180.4),{title:'Rara Vez',artist:'Milo J',duration:180})
 assert.equal(lyricsQuery('Song (Live at Wembley)','Artist',100).title,'Song (Live at Wembley)')
 assert.equal(lyricsQuery('Song (Remix)','Artist',100).title,'Song (Remix)')
 assert.equal(lyricsQuery('Artist – Song [Lyrics]','Artist',NaN).title,'Song')
})
test('lyrics requests deduplicate, cache successes and expire temporary failures',async()=>{
 let calls=0,time=0
 const query=lyricsQuery('Song','Artist',100)
 const loader=createLyricsLoader(async()=>{calls++;return {status:'synced',syncedLyrics:'[00:01]Hello'}},{now:()=>time})
 await Promise.all([loader(query),loader(query)]);assert.equal(calls,1)
 await loader(query);assert.equal(calls,1)
 time=3*60*60*1000;await loader(query);assert.equal(calls,2)
 let failures=0;time=0
 const retry=createLyricsLoader(async()=>{failures++;throw {kind:'temporary',retryAfterSeconds:15}},{now:()=>time})
 await assert.rejects(retry(query));await assert.rejects(retry(query));assert.equal(failures,1)
 time=16000;await assert.rejects(retry(query));assert.equal(failures,2)
})
test('cache stays bounded and user errors never reveal HTTP or provider details',async()=>{
 let calls=0
 const loader=createLyricsLoader(async()=>{calls++;return {status:'missing'}},{maxEntries:2})
 for(const title of ['one','two','three','one'])await loader(lyricsQuery(title,'Artist',100))
 assert.equal(calls,4)
 for(const error of ['HTTP 503 LRCLIB',{kind:'temporary'},{kind:'offline'},{kind:'timeout'}]){
  const text=lyricsFailure(error);assert.ok(text.length>20);assert.ok(!/503|HTTP|LRCLIB/.test(text))
 }
})

import {hasCompactLyrics} from '../src/lyrics-request.js'
import {notchSize} from '../src/dock-geometry.js'
test('idle lyrics occupy space only when a synchronized line can actually be displayed on every edge',()=>{
 for(const status of ['idle','loading','error','missing','instrumental','synced']){
  const lyrics={status,lines:status==='synced'?[{text:'Hello',time:1}]:[]}
  for(const index of [-1,0])for(const enabled of [false,true]){
   const ready=hasCompactLyrics(enabled,lyrics,index)
   assert.equal(ready,enabled&&status==='synced'&&index===0)
   for(const edge of ['top','bottom','left','right']){
    const size=notchSize(edge,'idle','M',ready?'lyrics':'home')
    const normal=notchSize(edge,'idle','M','home')
    if(ready)assert.ok(size[0]>normal[0])
    else assert.deepEqual(size,normal)
   }
  }
 }
})
