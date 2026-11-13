import {useEffect,useLayoutEffect,useRef,useState} from 'react'
import {getCurrentWindow,LogicalSize,PhysicalPosition,currentMonitor,primaryMonitor,cursorPosition,availableMonitors} from '@tauri-apps/api/window'
import {EDGES,isLateral,notchSize,dockPosition,slideOffset,transitionDuration,unionRects,desktopEnvelope,pillSize,pillPosition,pillEnvelope,pillDragPosition,clamp,cleanPillPositions} from './dock-geometry'
const initial=()=>{try{const d=JSON.parse(localStorage.getItem('companion-dock')||'{}');return {mode:d.mode==='pill'?'pill':'notch',pillPositions:cleanPillPositions(d.pillPositions),monitorId:typeof d.monitorId==='string'&&d.monitorId.length<=256?d.monitorId:null,edge:EDGES.includes(d.edge)?d.edge:'top',offsets:Object.fromEntries(EDGES.map(e=>[e,Number.isFinite(d.offsets?.[e])?Math.max(0,Math.min(1,d.offsets[e])):.5]))}}catch{return {mode:'notch',pillPositions:{},monitorId:null,edge:'top',offsets:{top:.5,bottom:.5,left:.5,right:.5}}}}
export function useDock({native,expanded,hovered,preview=false,retracted=false,tab,size,receiving=false,voiceVisible,onError,onDragEnd}){
  const [dock,setDock]=useState(initial)
  const [dragging,setDragging]=useState(false)
  const [refresh,setRefresh]=useState(0)
  const shown=useRef(false),lastState=useRef(null),lastTab=useRef(null),nativeFrame=useRef(null)
  const monitorRef=useRef(null),queue=useRef(Promise.resolve()),revision=useRef(0),dragRef=useRef(null),suppressClick=useRef(false),moveRevision=useRef(0)
  const mode=dock.mode||'notch',pill=mode==='pill',edge=pill?'top':dock.edge, lateral=isLateral(edge),state=pill&&dragging?'idle':expanded?'open':dragging?(dragRef.current?.visualState||'peek'):hovered?'peek':preview?'anticipate':retracted&&!pill?'retracted':'idle'
  const fraction=dock.offsets[edge]
  const monitorsRef=useRef([]),floatedFrame=useRef(false)
  const monitorKey=m=>`${m.name||"monitor"}:${m.position.x}:${m.position.y}`
  const point=dock.pillPositions?.[dock.monitorId]||{x:.5,y:.12}
  useEffect(()=>{localStorage.setItem('companion-dock',JSON.stringify(dock))},[dock])
  useLayoutEffect(()=>{
    const root=document.documentElement
    const previous=lastState.current
    const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches
    const duration=pill&&dragging?transitionDuration(dragRef.current?.visualState,'idle',reduced):previous===state&&state==='peek'&&lastTab.current!==tab&&!reduced?220:transitionDuration(previous,state,reduced)
    lastState.current=state
    lastTab.current=tab
    root.dataset.mode=mode;root.dataset.edge=edge;root.dataset.notch=state;root.dataset.dragging=String(dragging);root.dataset.centered=String(pill?Math.abs(point.x-.5)<.0001&&Math.abs(point.y-.5)<.0001:Math.abs(fraction-.5)<.0001)
    root.style.setProperty('--dock-duration',duration+'ms')
    root.style.setProperty('--dock-ease',state==='peek'&&previous!=='peek'&&!reduced?'cubic-bezier(.2,1.12,.3,1)':'cubic-bezier(.22,1,.36,1)')
    const [wantedW,wantedH]=(pill?pillSize(state,size,tab):notchSize(edge,state,size,tab,receiving))
    const setFrame=(x,y,w,h)=>{
      root.style.setProperty('--dock-left',x+'px');root.style.setProperty('--dock-top',y+'px')
      root.style.setProperty('--notch-width',w+'px');root.style.setProperty('--notch-height',h+'px')
    }
    if(native&&pill&&dragRef.current?.nativeMove){
      // Collapse toward the grab point inside the same HWND.
      // Pointer deltas keep moving that HWND without resizing or rebasing it.
      const d=dragRef.current
      setFrame(d.center.x-wantedW/2,d.center.y-wantedH/2,wantedW,wantedH)
      return
    }
    if(!native){const p=pill?pillPosition({x:0,y:0,width:window.innerWidth,height:window.innerHeight},wantedW,wantedH,point):{x:0,y:0};setFrame(p.x,p.y,wantedW,wantedH);return}
    // Update animation endpoints before paint. The OS frame is already fixed,
    // so opening/closing must not wait for an asynchronous monitor lookup.
    const cached=monitorRef.current
    if(cached&&nativeFrame.current){
      const scale=cached.scaleFactor,a=cached.workArea,area={x:a.position.x,y:a.position.y,width:a.size.width,height:a.size.height}
      const frame=(pill?pillEnvelope(area,size,scale,point):desktopEnvelope(area,edge,size,scale,fraction))
      if(['x','y','width','height'].every(k=>nativeFrame.current[k]===frame[k])){
        const w=Math.min(wantedW,(area.width-(pill?12*scale:0))/scale),h=Math.min(wantedH,(area.height-(pill?12*scale:0))/scale)
        const target=(pill?pillPosition(area,Math.round(w*scale),Math.round(h*scale),point,scale):dockPosition(area,Math.round(w*scale),Math.round(h*scale),edge,fraction))
        setFrame((target.x-frame.x)/scale,(target.y-frame.y)/scale,w,h)
      }
    }
    const ticket=++revision.current
    let finishTimer,frame1,frame2
    const valid=()=>ticket===revision.current
    queue.current=queue.current.catch(()=>{}).then(async()=>{
      if(!valid())return
      const win=getCurrentWindow(),monitors=pill?await availableMonitors():[];monitorsRef.current=monitors
      const monitor=(pill&&monitors.find(m=>monitorKey(m)===dock.monitorId))||await currentMonitor()||await primaryMonitor()
      if(!monitor)throw new Error('No se pudo detectar el monitor.')
      if(!valid())return
      monitorRef.current=monitor
      if(pill&&monitorKey(monitor)!==dock.monitorId){const id=monitorKey(monitor);setDock(d=>({...d,monitorId:id,pillPositions:{...d.pillPositions,[id]:d.pillPositions[id]||point}}))}
      const scale=monitor.scaleFactor,area=monitor.workArea
      const width=Math.max(1,Math.min(wantedW,(area.size.width-(pill?12*scale:0))/scale)),height=Math.max(1,Math.min(wantedH,(area.size.height-(pill?12*scale:0))/scale))
      const bounds={x:area.position.x,y:area.position.y,width:area.size.width,height:area.size.height}
      const pos=pill?pillPosition(bounds,Math.round(width*scale),Math.round(height*scale),point,scale):dockPosition(bounds,Math.round(width*scale),Math.round(height*scale),edge,fraction)
      const target={...pos,width:Math.round(width*scale),height:Math.round(height*scale)}
      const wantedFrame=pill?pillEnvelope(bounds,size,scale,point):desktopEnvelope(bounds,edge,size,scale,fraction)
      const previousFrame=nativeFrame.current
      const keepDraggedFrame=pill&&floatedFrame.current&&previousFrame&&target.x>=previousFrame.x&&target.y>=previousFrame.y&&target.x+target.width<=previousFrame.x+previousFrame.width&&target.y+target.height<=previousFrame.y+previousFrame.height
      const frame=keepDraggedFrame?previousFrame:wantedFrame
      if(!keepDraggedFrame)floatedFrame.current=false
      root.dataset.voiceSide=target.x-area.position.x>=224*scale?'left':'right'
      const localX=(target.x-frame.x)/scale,localY=(target.y-frame.y)/scale
      const sameFrame=nativeFrame.current&&['x','y','width','height'].every(k=>nativeFrame.current[k]===frame[k])
      if(shown.current&&sameFrame){
        // Pure CSS idle/peek animation; no native resize, move or final rebase.
        root.style.setProperty('--dock-duration',duration+'ms')
        setFrame(localX,localY,width,height)
        return
      }
      if(!shown.current||!duration||dragging){
        root.style.setProperty('--dock-duration','0ms')
        await win.setSize(new LogicalSize(frame.width/scale,frame.height/scale));await win.setPosition(new PhysicalPosition(frame.x,frame.y))
        nativeFrame.current=frame
        if(!valid())return
        setFrame(localX,localY,width,height)
        if(!shown.current){await win.show();shown.current=true;document.dispatchEvent(new Event('bimu-surface-ready'))}
        return
      }
      // Animate a DOM surface inside a temporary transparent native envelope.
      // Windows resizes only at the endpoints instead of on every animation frame.
      const origin=await win.outerPosition()
      if(!valid())return
      const rect=document.querySelector('.desktop').getBoundingClientRect()
      const old={x:origin.x+rect.left*scale,y:origin.y+rect.top*scale,width:rect.width*scale,height:rect.height*scale}
      const envelope=unionRects(old,frame)
      await win.setSize(new LogicalSize(envelope.width/scale,envelope.height/scale))
      await win.setPosition(new PhysicalPosition(Math.round(envelope.x),Math.round(envelope.y)))
      nativeFrame.current=envelope
      if(!valid())return
      root.style.setProperty('--dock-duration','0ms')
      setFrame((old.x-envelope.x)/scale,(old.y-envelope.y)/scale,old.width/scale,old.height/scale)
      frame1=requestAnimationFrame(()=>{frame2=requestAnimationFrame(()=>{
        if(!valid())return
        root.style.setProperty('--dock-duration',duration+'ms')
        setFrame((target.x-envelope.x)/scale,(target.y-envelope.y)/scale,width,height)
        finishTimer=setTimeout(()=>{
          queue.current=queue.current.catch(()=>{}).then(async()=>{
            if(!valid())return
            root.style.setProperty('--dock-duration','0ms')
            await win.setPosition(new PhysicalPosition(frame.x,frame.y))
            if(!valid())return
            setFrame(localX,localY,width,height)
            await win.setSize(new LogicalSize(frame.width/scale,frame.height/scale))
            nativeFrame.current=frame
          }).catch(e=>onError('No se pudo terminar la transición: '+String(e)))
        },duration+35)
      })})
    }).catch(e=>onError('No se pudo anclar el notch: '+String(e)))
    return()=>{revision.current++;clearTimeout(finishTimer);cancelAnimationFrame(frame1);cancelAnimationFrame(frame2)}
  },[native,edge,state,size,tab,fraction,refresh,dragging,receiving,mode,point.x,point.y,dock.monitorId])
  useEffect(()=>{
    if(!native)return
    let gone=false,off
    getCurrentWindow().onScaleChanged(()=>setRefresh(n=>n+1)).then(fn=>{if(gone)fn();else off=fn}).catch(e=>onError(String(e)))
    return()=>{gone=true;off?.()}
  },[native])
  const center=()=>setDock(d=>pill?{...d,pillPositions:{...d.pillPositions,[d.monitorId]:{x:.5,y:.5}}}:{...d,offsets:{...d.offsets,[d.edge]:.5}})
  const reanchor=()=>setRefresh(n=>n+1)
  const moveFloating=async d=>{
    if(d.moving)return
    d.moving=true
    try{
      while(d.target){const target=d.target;d.target=null;await getCurrentWindow().setPosition(new PhysicalPosition(target.x,target.y));nativeFrame.current={...d.frame,x:target.x,y:target.y}}
    }catch(error){onError('No se pudo mover Bimu: '+String(error))}
    finally{d.moving=false}
  }
  const placeFloating=(d,cursor)=>{
    const m=monitorsRef.current.find(m=>cursor.x>=m.position.x&&cursor.y>=m.position.y&&cursor.x<m.position.x+m.size.width&&cursor.y<m.position.y+m.size.height)||monitorRef.current
    const a=m.workArea,area={x:a.position.x,y:a.position.y,width:a.size.width,height:a.size.height}
    const placement=pillDragPosition(area,size,d.scale,cursor,d.grabOffset)
    d.placement={...placement,monitor:m}
    d.target={x:Math.round(placement.center.x-d.center.x*d.scale),y:Math.round(placement.center.y-d.center.y*d.scale)}
    floatedFrame.current=true;moveFloating(d)
  }
  const finishDrag=async e=>{
    const d=dragRef.current;if(!d||e.pointerId!==d.pointer)return
    d.ending=true
    // Flush the last coalesced move before measuring the real desktop location.
    while(d.moving)await new Promise(resolve=>setTimeout(resolve,8))
    try{
      if(pill&&native&&(d.moved||d.reanchored)){
        // Save the intended compact center, even if released before its animation ends.
        const center=d.placement.center,m=d.placement.monitor
        const a=m.workArea,id=monitorKey(m),x=Math.max(0,Math.min(1,(center.x-a.position.x)/a.size.width)),y=Math.max(0,Math.min(1,(center.y-a.position.y)/a.size.height))
        monitorRef.current=m;setDock(current=>({...current,monitorId:id,pillPositions:{...current.pillPositions,[id]:{x,y}}}))
      }
    }catch(error){onError(String(error))}
    finally{dragRef.current=null;moveRevision.current++;setDragging(false);if(pill&&d.visualState==='peek')onDragEnd?.();if(e.currentTarget?.hasPointerCapture?.(d.pointer))e.currentTarget.releasePointerCapture(d.pointer)}
  }
  const handlers={
    onPointerDown:async e=>{
      if(e.button!==0)return
      e.preventDefault();e.currentTarget.setPointerCapture?.(e.pointerId)
      const pressed={x:e.clientX,y:e.clientY,handle:e.currentTarget.classList.contains('notch-move-button')}
      const d={pointer:e.pointerId,start:fraction,x:native?e.screenX:e.clientX,y:native?e.screenY:e.clientY,moved:false,ready:false,visualState:state,point:{...point}};dragRef.current=d;suppressClick.current=false
      try{
        const cursor=native?await cursorPosition():{x:e.clientX,y:e.clientY}
        if(dragRef.current!==d)return
        d.x=cursor.x;d.y=cursor.y
        if(pill){
          const rect=document.querySelector('.desktop').getBoundingClientRect(),frame=nativeFrame.current,scale=native?monitorRef.current.scaleFactor:1
          const [w,h]=pillSize('idle',size)
          d.scale=scale;d.frame=frame?{...frame}:null;d.nativeMove=native;d.reanchored=pressed.handle
          d.center=pressed.handle?{x:clamp(pressed.x,w/2+6,(frame?.width/scale||window.innerWidth)-w/2-6),y:clamp(pressed.y,h/2+6,(frame?.height/scale||window.innerHeight)-h/2-6)}:{x:rect.left+rect.width/2,y:rect.top+rect.height/2}
          d.grabOffset=pressed.handle?{x:0,y:0}:{x:(frame?.x||0)+d.center.x*scale-cursor.x,y:(frame?.y||0)+d.center.y*scale-cursor.y}
          if(native)placeFloating(d,cursor)
          else if(pressed.handle)setDock(current=>({...current,monitorId:'preview',pillPositions:{...current.pillPositions,preview:{x:cursor.x/window.innerWidth,y:cursor.y/window.innerHeight}}}))
          setDragging(true)
        }
        d.ready=true
      }catch(error){dragRef.current=null;setDragging(false);onError(String(error))}
    },
    onPointerMove:async e=>{
      const d=dragRef.current;if(!d?.ready||d.ending||e.pointerId!==d.pointer)return
      const ticket=++moveRevision.current
      try{
        const cursor=native?await cursorPosition():{x:e.clientX,y:e.clientY}
        if(dragRef.current!==d||d.ending||ticket!==moveRevision.current)return
        const monitor=monitorRef.current,scale=native?monitor?.scaleFactor||1:1
        const dx=cursor.x-d.x,dy=cursor.y-d.y,delta=pill?Math.hypot(dx,dy):lateral?dy:dx
        if(Math.abs(delta)<4*scale&&!d.moved)return
        d.moved=true;suppressClick.current=true;setDragging(true)
        if(pill&&native){placeFloating(d,cursor);return}
        if(pill){const p=pillDragPosition({x:0,y:0,width:window.innerWidth,height:window.innerHeight},size,1,cursor,d.grabOffset);setDock(current=>({...current,monitorId:'preview',pillPositions:{...current.pillPositions,preview:{x:p.center.x/window.innerWidth,y:p.center.y/window.innerHeight}}}));return}
        const span=native?(lateral?monitor?.workArea.size.height:monitor?.workArea.size.width):(lateral?window.innerHeight:window.innerWidth)
        const next=slideOffset(d.start,delta,span||1000,scale)
        setDock(current=>({...current,offsets:{...current.offsets,[edge]:next}}))
      }catch(error){onError(String(error))}
    },
    onPointerUp:finishDrag,
    onPointerCancel:finishDrag,
    onLostPointerCapture:e=>{if(dragRef.current&&!dragRef.current.ending)finishDrag(e)}
  }

  return {mode,setMode:mode=>setDock(d=>({...d,mode:mode==='pill'?'pill':'notch'})),edge,lateral,state,dragging,fraction,center,reanchor,setEdge:edge=>{if(EDGES.includes(edge))setDock(d=>({...d,edge}))},centered:pill?Math.abs(point.x-.5)<.0001&&Math.abs(point.y-.5)<.0001:Math.abs(fraction-.5)<.0001,handlers,consumeDrag:()=>{const moved=suppressClick.current;suppressClick.current=false;return moved}}
}
