import assert from 'node:assert/strict'
import {notchSize,dockPosition,slideOffset,transitionDuration,unionRects,compactEnvelope,gazeTarget} from '../src/dock-geometry.js'
for(const area of [{x:0,y:0,width:1920,height:1040},{x:-2560,y:-200,width:2560,height:1440},{x:1920,y:0,width:3840,height:2080}]){
 for(const edge of ['top','bottom','left','right'])for(const scale of [1,1.25,2])for(const state of ['idle','peek','open'])for(const ratio of [0,.2,.5,.8,1]){
  const [w,h]=notchSize(edge,state,'M','settings');const width=Math.min(Math.round(w*scale),area.width),height=Math.min(Math.round(h*scale),area.height);const p=dockPosition(area,width,height,edge,ratio)
  assert.ok(p.x>=area.x&&p.x+width<=area.x+area.width+1);assert.ok(p.y>=area.y&&p.y+height<=area.y+area.height+1)
  if(edge==='top')assert.equal(p.y,area.y)
  if(edge==='bottom')assert.equal(p.y,area.y+area.height-height)
  if(edge==='left')assert.equal(p.x,area.x)
  if(edge==='right')assert.equal(p.x,area.x+area.width-width)
  if(ratio===.5){if(edge==='left'||edge==='right')assert.ok(Math.abs(p.y+height/2-(area.y+area.height/2))<=.5);else assert.ok(Math.abs(p.x+width/2-(area.x+area.width/2))<=.5)}
 }
}
assert.equal(slideOffset(.45,85,1920),.5);assert.equal(slideOffset(.45,180,3840,2),.5);assert.notEqual(slideOffset(.45,200,1920),.5)
assert.equal(slideOffset(.5,-9999,1920),0);assert.equal(slideOffset(.5,9999,1920),1)
assert.ok(notchSize('top','peek')[0]>notchSize('top','idle')[0])
assert.ok(notchSize('top','peek','M','lyrics')[1]>notchSize('top','peek','M','home')[1])
assert.ok(notchSize('top','idle','M','lyrics')[0]>notchSize('top','idle','M','home')[0])
for(const edge of ['left','right']){
 const idle=notchSize(edge,'idle'),peek=notchSize(edge,'peek')
 assert.ok(idle[0]<=40&&idle[1]<=126,'a small side tab at rest')
 assert.ok(peek[1]>=idle[1],'hover expands inward without a tall empty column')
 assert.ok(peek[0]-64>=220,'quick chat has enough horizontal reading space')
 const area={x:-1920,y:0,width:1920,height:1040},a=dockPosition(area,...idle,edge,.5),b=dockPosition(area,...peek,edge,.5)
 assert.ok(Math.abs((a.y+idle[1]/2)-(b.y+peek[1]/2))<=1)
 if(edge==='left')assert.equal(a.x,b.x);else assert.equal(a.x+idle[0],b.x+peek[0])
 assert.deepEqual(notchSize(edge,'peek','M','home',true),notchSize(edge,'peek','M','home'),'file reception uses the horizontal inward surface')
}
console.log('OK: 540 combinaciones de posición, límites de monitor, centro, DPI y magnetismo.')

assert.ok(transitionDuration('peek','open')<transitionDuration('idle','peek'))
assert.equal(transitionDuration('idle','peek',true),0)
assert.equal(transitionDuration('peek','peek'),0)
assert.deepEqual(unionRects({x:-20,y:20,width:100,height:40},{x:-50,y:0,width:180,height:90}),{x:-50,y:0,width:180,height:90})
console.log('OK: aperturas parcial/completa y contenedor de animación.')

for(const edge of ['top','bottom','left','right'])for(const scale of [1,1.25,2])for(const offset of [0,.5,1]){
 const area={x:-1920,y:0,width:1920,height:1040};const frame=compactEnvelope(area,edge,'M',scale,offset)
 for(const state of ['idle','peek']){
  const [w,h]=notchSize(edge,state);const width=Math.round(w*scale),height=Math.round(h*scale);const p=dockPosition(area,width,height,edge,offset)
  assert.ok(p.x>=frame.x&&p.y>=frame.y&&p.x+width<=frame.x+frame.width&&p.y+height<=frame.y+frame.height)
 }
}
assert.deepEqual(gazeTarget(-9999,9999,{left:20,top:20,width:64,height:54}),[-1,1])
assert.deepEqual(gazeTarget(52,47,{left:20,top:20,width:64,height:54}),[0,0])
console.log('OK: marco compacto estable en cuatro bordes y mirada acotada.')

// Voice feedback keeps the dock in place and stays within monitor bounds.
const {voiceEnvelope}=await import('../src/dock-geometry.js')
for(const scale of [1,1.5,2])for(const x of [-1920,-1200,-410]){
 const area={x:-1920,y:0,width:1920,height:1080}
 const target={x,y:0,width:330,height:86}
 const {frame,side}=voiceEnvelope(area,target,target,scale)
 assert.ok(frame.x>=area.x && frame.x+frame.width<=area.x+area.width)
 assert.ok(frame.x<=target.x && frame.x+frame.width>=target.x+target.width)
 assert.equal(side,x-area.x>=224*scale?'left':'right')
}

const {desktopEnvelope}=await import('../src/dock-geometry.js')
for(const area of [{x:0,y:0,width:1920,height:1040},{x:-1920,y:-200,width:1920,height:1080},{x:0,y:0,width:800,height:600}])for(const edge of ['top','bottom','left','right'])for(const scale of [1,1.25,2])for(const fraction of [0,.2,.5,.8,1]){
 const frame=desktopEnvelope(area,edge,'M',scale,fraction)
 assert.ok(frame.x>=area.x&&frame.y>=area.y&&frame.x+frame.width<=area.x+area.width&&frame.y+frame.height<=area.y+area.height)
 for(const [state,tab]of [['idle','home'],['idle','lyrics'],['retracted','home'],['retracted','lyrics'],['anticipate','home'],['anticipate','lyrics'],['peek','home'],['peek','lyrics'],['open','home'],['open','quick-settings'],['open','settings']]){
  const [w,h]=notchSize(edge,state,'M',tab),width=Math.min(Math.round(w*scale),area.width),height=Math.min(Math.round(h*scale),area.height)
  const target=dockPosition(area,width,height,edge,fraction)
  assert.ok(target.x>=frame.x&&target.y>=frame.y&&target.x+width<=frame.x+frame.width&&target.y+height<=frame.y+frame.height)
 }
 if(edge==='left'||edge==='right'){
  const [w,h]=notchSize(edge,'peek','M','home',true),width=Math.min(Math.round(w*scale),area.width),height=Math.min(Math.round(h*scale),area.height),target=dockPosition(area,width,height,edge,fraction)
  assert.ok(target.x>=frame.x&&target.y>=frame.y&&target.x+width<=frame.x+frame.width&&target.y+height<=frame.y+frame.height,'receiving files fits in the stable native frame')
 }
}
console.log('OK: marco nativo unico en reposo, hover, inicio y ajustes, con DPI y bordes de monitor.')

for (const state of ['peek','open']) for(const tab of ['home','lyrics','quick-settings','settings']) {
 assert.deepEqual(notchSize('top',state,'S',tab),notchSize('top',state,'L',tab))
 assert.deepEqual(notchSize('top',state,'M',tab),notchSize('top',state,'L',tab))
}
assert.notDeepEqual(notchSize('top','idle','S'),notchSize('top','idle','L'))

for(const edge of ['top','bottom','left','right'])for(const size of ['S','M','L']){
 assert.deepEqual(notchSize(edge,'peek',size,'home'),notchSize('top','peek','L','home'))
 assert.deepEqual(notchSize(edge,'peek',size,'lyrics'),notchSize('top','peek','L','lyrics'))
 assert.deepEqual(notchSize(edge,'open',size,'settings'),[680,570])
 assert.deepEqual(notchSize(edge,'open',size,'quick-settings'),[480,360])
}
