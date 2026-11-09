export const EDGES=['top','bottom','left','right']
export const isLateral=edge=>edge==='left'||edge==='right'
export const clamp=(value,min,max)=>Math.max(min,Math.min(max,value))
export function notchSize(edge,state,size='M',tab='home',receiving=false) {
  const lateral=isLateral(edge)
  // Expanded pages retain the approved horizontal layout on every edge.
  if(state==='open')return tab==='quick-settings'?[480,360]:[680,tab==='home'?350:570]
  const factor=state==='peek'?1.15:state==='idle'||state==='anticipate'?({S:.85,M:1,L:1.15}[size]||1):1
  if(state==='peek'&&tab==='cleanup')return [483,152]
  if(state==='peek'&&tab==='cleanup-details')return [483,224]
  if(state==='peek'&&tab==='lyrics-volume')return [598,206]
  const lyric=tab==='lyrics'||tab==='lyrics-loading'||tab==='caps'
  const base=state==='retracted'?(lateral?[7,64]:[64,7]):state==='peek'?(lyric?[520,142]:[420,132]):state==='anticipate'?(lateral?(lyric?[188,110]:[42,132]):(lyric?[332,30]:[162,30])):(lateral?(lyric?[180,104]:[40,126]):(lyric?[320,27]:[150,27]))
  return base.map(n=>Math.round(n*factor))
}
// All values here are physical pixels, including negative monitor origins.
export function dockPosition(area,width,height,edge,fraction) {
  const offset=clamp(Number.isFinite(fraction)?fraction:.5,0,1)
  const x=clamp(area.x+area.width*offset-width/2,area.x,area.x+Math.max(0,area.width-width))
  const y=clamp(area.y+area.height*offset-height/2,area.y,area.y+Math.max(0,area.height-height))
  if(edge==='bottom')return {x:Math.round(x),y:Math.round(area.y+area.height-height)}
  if(edge==='left')return {x:Math.round(area.x),y:Math.round(y)}
  if(edge==='right')return {x:Math.round(area.x+area.width-width),y:Math.round(y)}
  return {x:Math.round(x),y:Math.round(area.y)}
}
export function slideOffset(start,delta,span,scale=1){
  const offset=clamp(start+delta/Math.max(1,span),0,1)
  return Math.abs(offset-.5)*span<=24*scale ? 0.5 : offset
}

export function transitionDuration(from,to,reduced=false){
  if(reduced||from===to)return 0
  return to==='open'?190:to==='peek'?260:230
}
export function unionRects(a,b){
  const x=Math.min(a.x,b.x),y=Math.min(a.y,b.y)
  return {x,y,width:Math.max(a.x+a.width,b.x+b.width)-x,height:Math.max(a.y+a.height,b.y+b.height)-y}
}

// Idle and hover share one native frame: resizing the OS window must not
// participate in the avatar's hover animation.
export function compactEnvelope(area,edge,size,scale,fraction){
  const rect=state=>{
    const [w,h]=notchSize(edge,state,size)
    const width=Math.min(Math.round(w*scale),area.width),height=Math.min(Math.round(h*scale),area.height)
    return {...dockPosition(area,width,height,edge,fraction),width,height}
  }
  return unionRects(rect('idle'),rect('peek'))
}
export function gazeTarget(x,y,rect){
  return [clamp((x-rect.left)/Math.max(1,rect.width)*2-1,-1,1),clamp((y-rect.top)/Math.max(1,rect.height)*2-1,-1,1)]
}

// Reserve transparent space beside the dock without moving its visible surface.
export function voiceEnvelope(area,frame,target,scale=1){
  const gap=Math.round(224*scale)
  const left=target.x-area.x>=gap
  const space=left?Math.min(gap,frame.x-area.x):Math.min(gap,area.x+area.width-frame.x-frame.width)
  return {side:left?'left':'right',frame:left?{...frame,x:frame.x-space,width:frame.width+space}:{...frame,width:frame.width+space}}
}

// One OS frame for all UI states. Opening/collapsing moves DOM elements only.
export function desktopEnvelope(area,edge,size,scale,fraction){
 const rect=(state,tab)=>{
  const [w,h]=notchSize(edge,state,size,tab)
  const width=Math.min(Math.round(w*scale),area.width),height=Math.min(Math.round(h*scale),area.height)
  return {...dockPosition(area,width,height,edge,fraction),width,height}
 }
 const frame=[rect('idle'),rect('peek'),rect('open','home'),rect('open','settings'),rect('idle','lyrics'),rect('peek','lyrics')].reduce(unionRects)
 const pad=Math.round(224*scale),x=Math.max(area.x,frame.x-pad),right=Math.min(area.x+area.width,frame.x+frame.width+pad)
 const vertical=isLateral(edge)?Math.round(12*scale):0,y=Math.max(area.y,frame.y-vertical),bottom=Math.min(area.y+area.height,frame.y+frame.height+vertical)
 return {...frame,x,y,width:right-x,height:bottom-y}
}
