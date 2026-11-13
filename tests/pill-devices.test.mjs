import {test} from 'node:test'
import assert from 'node:assert/strict'
import {pillSize,pillPosition,pillEnvelope,cleanPillPositions} from '../src/dock-geometry.js'
import {pillContour,loopCometSegments} from '../src/music-edge.js'
import {pairingView} from '../src/device-pairing-ui.js'
test('floating frame contains compact, hover and settings on negative/DPI monitors',()=>{
 for(const scale of [1,1.25,2])for(const area of [{x:-1920,y:-180,width:1920,height:1080},{x:2560,y:0,width:2560,height:1440}])for(const point of [{x:0,y:0},{x:1,y:1},{x:.5,y:.5},{x:.2,y:.85}]){
  const frame=pillEnvelope(area,'M',scale,point)
  for(const [state,tab]of [['idle','home'],['peek','lyrics-volume'],['peek','cleanup-details'],['open','settings']]){const [w,h]=pillSize(state,'M',tab),width=Math.min(w*scale,area.width-12*scale),height=Math.min(h*scale,area.height-12*scale),p=pillPosition(area,width,height,point,scale);assert.ok(p.x>=frame.x-1&&p.y>=frame.y-1&&p.x+width<=frame.x+frame.width+1&&p.y+height<=frame.y+frame.height+1)}
 }
})
test('saved pill coordinates are bounded and malformed positions do not enter layout',()=>{assert.deepEqual(cleanPillPositions({a:{x:-5,y:2},b:{x:'bad',y:0}}),{a:{x:0,y:1}});assert.deepEqual(cleanPillPositions(null),{})})
test('pill comet closes the contour and keeps a thicker leading head across the seam',()=>{
 assert.ok(pillContour(134,28).d.endsWith('Z'));assert.equal(pillContour(0,28),null)
 for(const head of [0,.01,.5,.99,1]){const segments=loopCometSegments(head);assert.equal(segments.length,24);assert.ok(segments.every(s=>s.start>=0&&s.start<1&&Math.abs(s.end-s.start-.36/24)<1e-9));assert.ok(segments.slice(1).every((s,i)=>s.width>segments[i].width&&s.opacity>segments[i].opacity));}
})
test('adding another saved PC requires explicit navigation intent',()=>{assert.equal(pairingView({phase:'idle',linked:true},'join'),'reconnecting');assert.equal(pairingView({phase:'idle',linked:true},'join',true),'join');assert.equal(pairingView({phase:'idle',linked:true},'choose',true),'start');assert.equal(pairingView({phase:'connected',linked:true},'join',true),'connected')})
