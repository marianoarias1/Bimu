import React,{useEffect,useId,useRef} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'
import {musicEdgePaths} from './music-edge'

// Only the moving edge is redrawn; no React renders, native resizes, or canvas.
export function MusicEdge({active,edge,native}){
 const svg=useRef(null),fill=useRef(null),line=useRef(null),glow=useRef(null),filterNode=useRef(null)
 const id=useId().replace(/[^a-zA-Z0-9_-]/g,''),filter=`music-glow-${id}`,gradient=`music-fade-${id}`
 const lateral=edge==='left'||edge==='right'
 useEffect(()=>{
  if(!active)return
  let alive=true,frame,off,previous=0,drawn=0,phase=0,target=.25,level=.25,length=0
  const node=svg.current,parent=node.parentElement,reduced=matchMedia('(prefers-reduced-motion: reduce)')
  const draw=()=>{const paths=musicEdgePaths(length,phase,level,lateral);fill.current?.setAttribute('d',paths.fill);line.current?.setAttribute('d',paths.line);glow.current?.setAttribute('d',paths.line)}
  const resize=()=>{const r=parent.getBoundingClientRect();length=lateral?r.height:r.width;node.setAttribute('viewBox',lateral?`0 0 8 ${length}`:`0 0 ${length} 8`);filterNode.current?.setAttribute('width',lateral?24:length+16);filterNode.current?.setAttribute('height',lateral?length+16:24);draw()}
  const observer=new ResizeObserver(resize);observer.observe(parent);resize()
  const tick=now=>{
   if(!alive)return
   // Limit path updates to ~30 fps; advance by elapsed time, not frame count.
   if(!reduced.matches&&now-drawn>=32){const dt=previous?Math.min(100,now-previous):32;previous=now;drawn=now;phase+=dt/2600*Math.PI*2;level+=(target-level)*(1-Math.exp(-dt/220));draw()}
   frame=requestAnimationFrame(tick)
  }
  // Reduced motion is a static lit contour, without continuous redraws.
  const animate=()=>{cancelAnimationFrame(frame);previous=0;if(native)invoke('audio_meter_enabled',{enabled:!document.hidden}).catch(()=>{});if(document.hidden)return;if(reduced.matches)draw();else frame=requestAnimationFrame(tick)}
  reduced.addEventListener('change',animate);document.addEventListener('visibilitychange',animate);animate()
  if(native){listen('audio-output-peak',({payload})=>{if(alive)target=Math.min(1,Math.sqrt(Math.max(0,Number(payload)||0))*1.5)}).then(fn=>{if(alive)off=fn;else fn()}).catch(()=>{})}
  return()=>{alive=false;cancelAnimationFrame(frame);observer.disconnect();reduced.removeEventListener('change',animate);document.removeEventListener('visibilitychange',animate);off?.();if(native)invoke('audio_meter_enabled',{enabled:false}).catch(()=>{})}
 },[active,edge,native,lateral])
 if(!active)return null
 return <svg ref={svg} className={`bimu-music-edge edge-${edge}`} aria-hidden="true" focusable="false" preserveAspectRatio="none">
  <defs><filter ref={filterNode} id={filter} filterUnits="userSpaceOnUse" x="-8" y="-8" width="24" height="24" colorInterpolationFilters="sRGB"><feGaussianBlur stdDeviation="1.6"/></filter><linearGradient id={gradient} x1="0" y1="0" x2={lateral?'0':'1'} y2={lateral?'1':'0'}><stop offset="15%" stopColor="var(--audio-accent)" stopOpacity="0"/><stop offset="35%" stopColor="var(--audio-accent)" stopOpacity=".5"/><stop offset="65%" stopColor="var(--audio-accent)" stopOpacity=".5"/><stop offset="85%" stopColor="var(--audio-accent)" stopOpacity="0"/></linearGradient></defs>
  <path ref={fill} className="music-edge-fill"/>
  <path ref={glow} fill="none" stroke={`url(#${gradient})`} strokeWidth="3" filter={`url(#${filter})`} className="music-edge-glow"/>
  <path ref={line} fill="none" stroke={`url(#${gradient})`} strokeWidth=".6" className="music-edge-line"/>
 </svg>
}
