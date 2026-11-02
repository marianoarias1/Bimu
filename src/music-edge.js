// Fixed silhouette: only light moves along this OPEN contour. The edge against
// the monitor is never part of the path. Coordinates are canonical top-edge
// coordinates and are reflected for the other three placements.
export function freeContour(width,height,edge='top',radius=13){
 if(!Number.isFinite(width)||!Number.isFinite(height)||width<=0||height<=0)return null
 const lateral=edge==='left'||edge==='right',span=lateral?height:width,depth=lateral?width:height
 const r=Math.min(Math.max(0,Number.isFinite(radius)?radius:13),span/2,depth)
 const shoulder=Math.min(12,Math.max(0,depth-r))
 const f=n=>Number(n.toFixed(3))
 const d=`M ${f(-shoulder)} 0 A ${f(shoulder)} ${f(shoulder)} 0 0 1 0 ${f(shoulder)} L 0 ${f(depth-r)} A ${f(r)} ${f(r)} 0 0 0 ${f(r)} ${f(depth)} L ${f(span-r)} ${f(depth)} A ${f(r)} ${f(r)} 0 0 0 ${f(span)} ${f(depth-r)} L ${f(span)} ${f(shoulder)} A ${f(shoulder)} ${f(shoulder)} 0 0 1 ${f(span+shoulder)} 0`
 const transform=edge==='bottom'?`translate(0 ${height}) scale(1 -1)`:edge==='left'?'matrix(0 1 1 0 0 0)':edge==='right'?`matrix(0 1 -1 0 ${width} 0)`:''
 const length=Math.PI*(shoulder+r)+2*(depth-r-shoulder)+span-2*r
 return {d,transform,length,span,depth,shoulder,radius:r}
}
// The LAST segment is always the leading head. Width and light grow strictly
// toward that end; the trailing end fades out, including around rounded corners.
// Clip rather than wrap: a comet cannot reappear on the other end of this open
// path until its entire tail has left it.
export function cometSegments(head,tail=.36,count=24){
 if(!Number.isFinite(head))return []
 tail=Number.isFinite(tail)?Math.max(.01,Math.min(.8,tail)):.36
 count=Number.isFinite(count)?Math.max(2,Math.min(48,Math.round(count))):24
 return Array.from({length:count},(_,i)=>{
  const t=(i+1)/count,start=Math.max(0,head-tail+i*tail/count),end=Math.min(1,head-tail+(i+1)*tail/count)
  return {start,end,width:.25+1.85*t*t,opacity:end>start?Math.pow(t,1.65):0}
 })
}
export function cometHead(elapsed,duration=4200){
 if(!Number.isFinite(elapsed)||!Number.isFinite(duration)||duration<=0)return -.02
 return ((Math.max(0,elapsed)%duration)/duration)*1.4-.02
}
