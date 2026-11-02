import React,{useEffect,useId,useRef} from 'react'
import {freeContour,cometSegments,cometHead} from './music-edge'

const COUNT=24
// One fixed SVG path, a single filtered glow copy and 24 short light segments.
// No audio polling, native resizing, React renders per frame, or shape deformation.
export function MusicEdge({active,edge}){
 const svg=useRef(null),oriented=useRef(null),trail=useRef(null),filterNode=useRef(null)
 const id=useId().replace(/[^a-zA-Z0-9_-]/g,''),filter=`music-glow-${id}`,group=`music-trail-${id}`
 useEffect(()=>{
  if(!active)return
  let alive=true,frame,previous=0,drawn=0,cycle=0,duration=4200
  const node=svg.current,parent=node.parentElement,reduced=matchMedia('(prefers-reduced-motion: reduce)')
  const paths=Array.from(trail.current.children)
  const draw=head=>{
   cometSegments(head,.36,COUNT).forEach((segment,i)=>{
    const path=paths[i]
    path.setAttribute('stroke-dasharray',`${Math.max(0,segment.end-segment.start)*1000} 2000`)
    path.setAttribute('stroke-dashoffset',String(-segment.start*1000))
    path.setAttribute('opacity',String(segment.opacity))
   })
  }
  const resize=()=>{
   const style=getComputedStyle(parent)
   const radius=parseFloat(edge==='bottom'?style.borderTopLeftRadius:edge==='left'?style.borderTopRightRadius:style.borderBottomLeftRadius)||0
   const width=parent.clientWidth,height=parent.clientHeight,contour=freeContour(width,height,edge,radius)
   if(!contour)return
   node.setAttribute('viewBox',`0 0 ${width} ${height}`)
   oriented.current.setAttribute('transform',contour.transform)
   filterNode.current.setAttribute('width',String(contour.span+40))
   filterNode.current.setAttribute('height',String(contour.depth+40))
   paths.forEach(path=>path.setAttribute('d',contour.d))
   duration=Math.max(4200,Math.min(10000,contour.length/65*1000))
   draw(reduced.matches ? .62 : cometHead(cycle,1))
  }
  const tick=now=>{
   if(!alive)return
   if(now-drawn>=32){const dt=previous?Math.min(100,now-previous):0;previous=now;drawn=now;cycle+=dt/duration;draw(cometHead(cycle,1))}
   frame=requestAnimationFrame(tick)
  }
  const animate=()=>{cancelAnimationFrame(frame);previous=0;if(document.hidden)return;if(reduced.matches)draw(.62);else frame=requestAnimationFrame(tick)}
  const observer=new ResizeObserver(resize);observer.observe(parent);resize();animate()
  reduced.addEventListener('change',animate);document.addEventListener('visibilitychange',animate)
  return()=>{alive=false;cancelAnimationFrame(frame);observer.disconnect();reduced.removeEventListener('change',animate);document.removeEventListener('visibilitychange',animate)}
 },[active,edge])
 // Keep the tiny SVG shell mounted to fade the last frame out on pause.
 // Inactive: no observer, animation frame, meter or other continuous work.
 return <svg ref={svg} className={'bimu-music-edge'+(active?' is-playing':'')} aria-hidden="true" focusable="false" preserveAspectRatio="none">
  <defs><filter ref={filterNode} id={filter} filterUnits="userSpaceOnUse" x="-20" y="-20" width="40" height="40" colorInterpolationFilters="sRGB"><feGaussianBlur stdDeviation="1.8"/></filter></defs>
  <g ref={oriented}>
   <use href={`#${group}`} filter={`url(#${filter})`} className="music-comet-glow"/>
   <g ref={trail} id={group} fill="none" strokeLinecap="round" strokeLinejoin="round">
    {Array.from({length:COUNT},(_,i)=><path key={i} pathLength="1000" strokeWidth={.25+1.85*((i+1)/COUNT)**2} stroke={i>=COUNT-5?'var(--comet-head-color)':'var(--audio-accent)'} opacity="0"/>)}
   </g>
  </g>
 </svg>
}
