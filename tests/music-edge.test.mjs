import {test} from 'node:test'
import assert from 'node:assert/strict'
import {freeContour,cometSegments,cometHead} from '../src/music-edge.js'

test('fixed contour follows shoulders and free corners on all four edges, never closing across monitor',()=>{
 for(const [w,h,r] of [[150,27,13],[128,23,13],[40,126,15],[46,145,15],[483,152,29],[598,163,29]])for(const edge of ['top','bottom','left','right']){
  const c=freeContour(w,h,edge,r),lateral=edge==='left'||edge==='right'
  assert.equal(c.span,lateral?h:w);assert.equal(c.depth,lateral?w:h)
  assert.ok(c.length>0);assert.equal((c.d.match(/ A /g)||[]).length,4)
  assert.ok(!/NaN|Infinity| Z| C /.test(c.d),'fixed arcs, no waveform or closed monitor edge')
  assert.ok(c.d.startsWith(`M ${-c.shoulder} 0`))
  assert.ok(c.d.endsWith(`${c.span+c.shoulder} 0`))
  assert.equal(c.transform,edge==='top'?'':edge==='bottom'?`translate(0 ${h}) scale(1 -1)`:edge==='left'?'matrix(0 1 1 0 0 0)':`matrix(0 1 -1 0 ${w} 0)`)
 }
 assert.equal(freeContour(0,27),null);assert.equal(freeContour(150,NaN),null)
 assert.ok(!freeContour(150,27,'top',Infinity).d.includes('NaN'))
})
test('thickest and brightest segment is the leading end, not the middle',()=>{
 const s=cometSegments(.7)
 assert.equal(s.length,24)
 for(let i=1;i<s.length;i++){assert.ok(s[i].width>s[i-1].width);assert.ok(s[i].opacity>s[i-1].opacity);assert.ok(s[i].end>s[i-1].end);assert.ok(Math.abs(s[i].start-s[i-1].end)<1e-9)}
 assert.ok(Math.abs(s.at(-1).end-.7)<1e-9);assert.equal(s.at(-1).width,2.1)
 assert.ok(s[0].opacity<.01);assert.equal(s.at(-1).opacity,1)
})
test('tail clips at open ends instead of wrapping onto a second head',()=>{
 for(const head of [-.02,0,.01,.5,1,1.2,1.38]){
  const visible=cometSegments(head).filter(s=>s.opacity>0)
  assert.ok(visible.every(s=>s.start>=0&&s.end<=1&&s.start<s.end))
 }
 assert.equal(cometSegments(-.02).filter(s=>s.opacity>0).length,0)
 assert.equal(cometSegments(1.38).filter(s=>s.opacity>0).length,0)
 assert.deepEqual(cometSegments(NaN),[])
})
test('motion is continuous in time and resets only after the whole tail has left',()=>{
 const duration=4200
 assert.equal(cometHead(0,duration),-.02);assert.equal(cometHead(duration,duration),-.02)
 assert.ok(Math.abs(cometHead(1001,duration)-cometHead(1000,duration))<.001)
 assert.equal(cometSegments(cometHead(duration-1,duration)).filter(s=>s.opacity>0).length,0)
 assert.equal(cometHead(Infinity),-.02);assert.equal(cometHead(10,0),-.02)
})
