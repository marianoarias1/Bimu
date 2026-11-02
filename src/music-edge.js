// A shallow, outward-only contour. The endpoints meet the original edge with
// zero slope, so neither the black silhouette nor its light has a hard seam.
export function musicEdgePaths(length,phase=0,level=0,lateral=false){
 if(!Number.isFinite(length)||length<=0)return {fill:'',line:''}
 phase=Number.isFinite(phase)?phase:0
 level=Number.isFinite(level)?Math.max(0,Math.min(1,level)):0
 const start=length*.18,span=length*.64,amplitude=.65+level*.95
 const point=(along,out)=>lateral?`${out.toFixed(3)},${along.toFixed(3)}`:`${along.toFixed(3)},${out.toFixed(3)}`
 const points=Array.from({length:65},(_,i)=>{const t=i/64,envelope=Math.sin(Math.PI*t)**2
  const out=amplitude*envelope*(.5+.5*Math.sin(t*Math.PI*4+phase))
  return point(start+span*t,out)
 })
 const line='M'+points.join(' L')
 return {line,fill:line+' Z'}
}
