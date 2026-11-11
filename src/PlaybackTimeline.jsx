import React,{useRef,useState} from 'react'

const clock=seconds=>{
 const n=Math.max(0,Math.floor(Number(seconds)||0))
 return `${Math.floor(n/60)}:${String(n%60).padStart(2,'0')}`
}

export function PlaybackTimeline({media,onSeek}){
 const gesture=useRef(null)
 const [draft,setDraft]=useState(null)
 const [pending,setPending]=useState(false)
 const submitting=useRef(false)
 const duration=Number(media?.duration)||0
 const [min,max]=media?.seekRange??[0,duration]
 const enabled=duration>0&&max>min&&!!media?.seekRange
 const current=Math.max(0,Math.min(duration,Number(media?.position)||0))
 const shown=Math.max(0,Math.min(duration,draft??current))
 const percent=duration>0?shown/duration*100:0
 const commit=async value=>{
  if(!enabled||submitting.current)return
  submitting.current=true
  setDraft(value);setPending(true)
  try{await onSeek(value,media)}finally{submitting.current=false;setDraft(null);setPending(false)}
 }
 const clamp=value=>Math.max(min,Math.min(max,value))
 const pointerValue=event=>{
  const {left,width}=event.currentTarget.getBoundingClientRect()
  return clamp(width>0?(event.clientX-left)*duration/width:current)
 }
 const cancel=event=>{
  if(gesture.current!==event.pointerId)return
  gesture.current=null;setDraft(null)
 }
 return <div className="player-timeline">
  <span>{clock(shown)}</span>
  <div className={'timeline-scrubber'+(draft!==null?' is-scrubbing':'')}>
   <span aria-hidden="true" className={'player-progress player-wave'+(media?.playing?' is-playing':'')}><i style={{width:`${percent}%`}}/></span>
   {enabled&&<>
    <i className="timeline-thumb" aria-hidden="true" style={{left:`${percent}%`}}/>
    <input aria-label="Posición de reproducción" aria-valuetext={`${clock(shown)} de ${clock(duration)}`}
     type="range" min={0} max={duration} step={0.01} value={shown} aria-busy={pending}
     onKeyDown={event=>{
      const target={ArrowLeft:shown-5,ArrowDown:shown-5,ArrowRight:shown+5,ArrowUp:shown+5,
       PageDown:shown-duration*.1,PageUp:shown+duration*.1,Home:min,End:max}[event.key]
      if(typeof target!=='number')return
      event.preventDefault()
      if(!gesture.current)void commit(clamp(target))
     }}
     onPointerDown={event=>{
      if(event.button!==0||!event.isPrimary||submitting.current)return
      event.preventDefault()
      event.currentTarget.focus({preventScroll:true})
      event.currentTarget.setPointerCapture(event.pointerId)
      gesture.current=event.pointerId
      setDraft(pointerValue(event))
     }}
     onChange={event=>{if(!gesture.current)void commit(clamp(Number(event.currentTarget.value)))}}
     onPointerMove={event=>{if(gesture.current===event.pointerId)setDraft(pointerValue(event))}}
     onPointerUp={event=>{
      if(gesture.current!==event.pointerId)return
      gesture.current=null
      void commit(pointerValue(event))
     }}
     onPointerCancel={cancel} onLostPointerCapture={cancel}
     onBlur={()=>{if(gesture.current){gesture.current=null;setDraft(null)}}}/>
   </>}
  </div>
  <span>{clock(duration)}</span>
 </div>
}
