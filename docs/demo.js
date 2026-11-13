import {notchSize,pillSize,pillPosition} from './assets/bimu-dock.js'
import {freeContour,pillContour,cometSegments,loopCometSegments,cometHead} from './assets/bimu-contour.js'

// An isolated website simulation. No Tauri, native IPC, microphone, audio,
// location lookup, filesystem access, or access to the visitor's clipboard.
const root=document.documentElement, desktop=document.querySelector('.desktop')
const allowedViews=['hero','music','weather','clipboard','pill']
const requested=new URLSearchParams(location.search).get('view')
const view=allowedViews.includes(requested)?requested:'music'
const pill=view==='pill', scene=view==='hero'||pill
const reduced=matchMedia('(prefers-reduced-motion: reduce)')
let page=view==='weather'?'weather':view==='clipboard'?'clipboard':'music'
let open=view==='hero'||view==='weather'||view==='clipboard'
let lyrics=view==='hero', playing=true, motion=!reduced.matches, visible=true, dragging=null
let volumePanel=null, seeking=false
let position=58, duration=247, volume=65, historyPaused=false, city=0, feedbackTimer, hoverTimer
let anchor={x:.5,y:pill?0.48:0}, scale=1, width=150, height=27
let history=[
 {id:1,kind:'image',summary:'Imagen · 1600 × 1000',origin:'Este equipo'},
 {id:2,kind:'text',summary:'Revisar la propuesta del viernes',origin:'Este equipo'},
 {id:3,kind:'file',summary:'Presentación.pdf',origin:'Notebook'}
]
root.dataset.demo=view;root.dataset.scene=String(scene);root.dataset.mode=pill?'pill':'notch';root.dataset.centered='true'

const paths={
 right:'<path d="m9 18 6-6-6-6"/>',
 wifi:'<path d="M12 20h.01"/><path d="M2 8.82a15 15 0 0 1 20 0"/><path d="M5 12.859a10 10 0 0 1 14 0"/><path d="M8.5 16.429a5 5 0 0 1 7 0"/>',
 settings:'<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
 captions:'<rect width="18" height="14" x="3" y="5" rx="2" ry="2"/><path d="M7 15h4M15 15h2M7 11h2M13 11h4"/>',
 grip:'<circle cx="12" cy="5" r="1"/><circle cx="19" cy="5" r="1"/><circle cx="5" cy="5" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/><circle cx="12" cy="19" r="1"/><circle cx="19" cy="19" r="1"/><circle cx="5" cy="19" r="1"/>',
 up:'<path d="M5 3h14"/><path d="m18 13-6-6-6 6"/><path d="M12 7v14"/>',
 left:'<path d="m15 18-6-6 6-6"/>',
 volume:'<path d="M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z"/><path d="M16 9a5 5 0 0 1 0 6"/><path d="M19.364 18.364a9 9 0 0 0 0-12.728"/>',
 sun:'<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l2 2m10 10 2 2M5 19l2-2M17 7l2-2"/>',
 cloud:'<path d="M6 18h12a4 4 0 0 0 0-8 6 6 0 0 0-11-2 5 5 0 0 0-1 10Z"/>',
 pin:'<path d="M20 10c0 6-8 12-8 12S4 16 4 10a8 8 0 0 1 16 0Z"/><circle cx="12" cy="10" r="2"/>',
 refresh:'<path d="M20 7v5h-5M4 17v-5h5"/><path d="M5 7a8 8 0 0 1 14 1M19 17A8 8 0 0 1 5 16"/>',
 trash:'<path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7"/>',
 clipboard:'<rect x="5" y="4" width="14" height="18" rx="2"/><rect x="9" y="2" width="6" height="4" rx="1"/>',
 file:'<path d="M14 2H6v20h12V6ZM14 2v5h5"/>',
 check:'<path d="m5 12 4 4L19 6"/>'
}
function icon(name,size=15) {return '<svg width="'+size+'" height="'+size+'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'+(paths[name]||paths.file)+'</svg>'}
function glyph(kind,size=17) {
 const shape=kind==='pause'?'<rect x="5.5" y="4" width="4.5" height="16" rx="2"/><rect x="14" y="4" width="4.5" height="16" rx="2"/>':kind==='play'?'<path d="M7.8 4.6c-.9-.5-2 .1-2 1.2v12.4c0 1.1 1.1 1.6 2 1.2l10-6.2c.9-.6.9-1.8 0-2.4z"/>':kind==='previous'?'<rect x="4" y="5" width="3" height="14" rx="1.5"/><path d="M18 5.2c.9-.5 2 .1 2 1.1v11.4c0 1-1.1 1.6-2 1.1l-8-5.7c-.8-.5-.8-1.7 0-2.2z"/>':'<rect x="17" y="5" width="3" height="14" rx="1.5"/><path d="M6 5.2c-.9-.5-2 .1-2 1.1v11.4c0 1 1.1 1.6 2 1.1l8-5.7c.8-.5.8-1.7 0-2.2z"/>'
 return '<svg width="'+size+'" height="'+size+'" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">'+shape+'</svg>'
}
if(scene)document.querySelector('.windows-scene').innerHTML=pill?'<div class="win-taskbar"><span class="win-logo"><i></i><i></i><i></i><i></i></span><i class="taskbar-icon folder"></i><i class="taskbar-icon bimu"></i><span class="taskbar-time">Bimu · Windows</span></div>':'<div class="explorer"><div class="explorer-title"><span>▰ &nbsp; Documentos</span><span>− &nbsp; □ &nbsp; ×</span></div><div class="explorer-tools">← &nbsp; → &nbsp; ↑ &nbsp;&nbsp; Este equipo &nbsp; › &nbsp; Documentos</div><div class="explorer-content"><div class="explorer-sidebar"><span>Inicio</span><span>Escritorio</span><span>Descargas</span><span>Documentos</span><span>Imágenes</span></div><div class="explorer-files"><span class="folder-file"><i class="folder-icon"></i>Proyectos</span><span class="folder-file"><i class="folder-icon"></i>Fotos</span><span class="folder-file"><i class="folder-icon"></i>Trabajo</span></div></div></div><div class="explorer explorer-secondary"><div class="explorer-title"><span>▰ &nbsp; Proyectos</span><span>− &nbsp; □ &nbsp; ×</span></div><div class="explorer-tools">Documentos &nbsp; › &nbsp; Proyectos</div><div class="explorer-files"><span class="folder-file"><i class="mini-document"></i>Propuesta</span><span class="folder-file"><i class="folder-icon"></i>Recursos</span></div></div><div class="win-taskbar"><span class="win-logo"><i></i><i></i><i></i><i></i></span><i class="taskbar-icon folder"></i><i class="taskbar-icon"></i><i class="taskbar-icon bimu"></i><span class="taskbar-time"></span></div>'

desktop.innerHTML='<button type="button" class="notch-surface" aria-label="Desplegar Bimu" aria-expanded="'+open+'"><span class="notch-face" aria-hidden="true"><span class="notch-avatar-slot bimu-eyes bimu-listening"><i></i><i></i></span></span><span class="notch-copy"><strong>Tu compañero</strong><small>Acá, con vos</small></span><time class="notch-clock"></time><span class="system-status" aria-label="Conexión Wi-Fi">'+icon('wifi',15)+'</span><span class="notch-indicator"><span class="status-dot"></span></span><span class="notch-grip">'+icon('grip',12)+'</span></button><section class="notch-player" aria-label="Herramientas de Bimu"><div class="notch-player-content"><div class="player-heading"><button type="button" class="notch-move-button" aria-label="'+(pill?'Arrastrar pill':'Puntos de movimiento de Bimu')+'" title="'+(pill?'Mantené presionado para mover la pill':'Movimiento disponible en modo pill')+'">'+icon('grip',15)+'</button><span class="player-date"></span><time></time></div><div class="notch-page"></div><button type="button" class="player-menu-arrow player-menu-left" aria-label="Función anterior">'+icon('left',13)+'</button><button type="button" class="player-menu-arrow player-menu-right" aria-label="Función siguiente">'+icon('right',13)+'</button><div class="notch-pagination" role="group" aria-label="Funciones"></div></div></section><button type="button" class="notch-settings-button" aria-label="Probar personalización" title="Probar personalización">'+icon('settings',15)+'</button><button type="button" class="notch-retract-button" aria-label="Reducir Bimu" title="Reducir">'+icon('up',15)+'</button>'
const surface=desktop.querySelector('.notch-surface'), content=desktop.querySelector('.notch-page')
const nativeContent=desktop.querySelector('.notch-player-content')
nativeContent.append(desktop.querySelector('.notch-settings-button'))
if(pill)desktop.querySelector('.notch-retract-button').remove()
else nativeContent.append(desktop.querySelector('.notch-retract-button'))
const pages=['music','weather','clipboard'], labels=['Música','Clima','Portapapeles']
desktop.querySelector('.notch-pagination').innerHTML=pages.map((id,i)=>'<button type="button" data-page="'+id+'" aria-label="'+labels[i]+'"></button>').join('')
function renderPage() {
 root.dataset.lyrics=String(page==='music'&&lyrics)
 content.innerHTML=page==='music'?'<div class="player-track"><span class="player-cover"><img src="assets/bad-cover.webp" alt="Portada del álbum Bad"></span><span><strong>Bad - 2012 Remaster</strong><small>Michael Jackson</small></span></div><div class="player-timeline"><span class="elapsed"></span><div class="timeline-scrubber"><span class="player-progress player-wave is-playing" aria-hidden="true"><i></i></span><i class="timeline-thumb" aria-hidden="true"></i><input type="range" aria-label="Posición de reproducción simulada" min="0" max="247" step=".1" value="'+position+'"></div><span>4:07</span></div><div class="player-controls"><button type="button" class="lyrics-toggle" aria-label="Alternar letra" aria-pressed="'+lyrics+'">'+icon('captions')+'</button><button type="button" class="player-action" data-media="previous" aria-label="Retroceder en la demo">'+glyph('previous')+'</button><button type="button" class="player-action player-toggle" aria-label="'+(playing?'Pausar':'Reproducir')+' la demo">'+glyph(playing?'pause':'play',19)+'</button><button type="button" class="player-action" data-media="next" aria-label="Avanzar en la demo">'+glyph('next')+'</button><div class="volume-control"><button type="button" class="volume-toggle" aria-label="Mostrar volumen de la demo" aria-expanded="false">'+icon('volume',17)+'</button><div class="volume-popover" hidden><strong>Volumen de Windows</strong><div class="volume-row">'+icon('volume',13)+'<input type="range" min="0" max="100" value="'+volume+'" style="--volume-fill:'+volume+'%" aria-label="Volumen simulado"><span>'+volume+'%</span></div></div></div></div>'+(lyrics?'<div class="lyrics-panel" aria-label="Letra sincronizada"><div class="demo-lyrics"><p data-line="0">You know I’m bad</p><p data-line="1">Really, really bad</p></div></div>':''):page==='weather'?weatherMarkup():clipboardMarkup()
 desktop.querySelectorAll('[data-page]').forEach(button=>button.setAttribute('aria-current',button.dataset.page===page?'page':'false'))
 const popup=content.querySelector('.volume-popover');if(popup){volumePanel=popup;popup.remove()}
 updatePlayback()
 layout()
}
function weatherMarkup() {
 const c=city?{name:'Córdoba',temperature:26,high:29,low:15}:{name:'Buenos Aires',temperature:24,high:27,low:12}
 return '<section class="weather-view" aria-label="Clima de demostración"><div class="weather-heading"><strong>Clima</strong><button type="button" data-weather>'+icon('pin',11)+' '+c.name+'</button></div><div class="weather-now"><span class="weather-icon">'+icon('sun',29)+'</span><strong>'+c.temperature+'°</strong><div><b>Mayormente despejado</b><small>Sensación '+(c.temperature-1)+'° · Máx. '+c.high+'° / Mín. '+c.low+'°</small></div><button type="button" class="weather-refresh" aria-label="Actualizar clima de demostración">'+icon('refresh',13)+'</button></div><div class="weather-hours">'+[0,1,2].map((i)=>'<div><span>'+String(15+i*2).padStart(2,'0')+':00</span>'+icon(i===2?'cloud':'sun',15)+'<b>'+(c.temperature+i)+'°</b><small>☂ '+(i===2?10:0)+'%</small></div>').join('')+'</div></section>'
}
function clipboardMarkup() {
 return '<section class="clipboard-view" aria-label="Historial de demostración"><div class="clipboard-heading"><strong>Portapapeles</strong><div><button type="button" data-history="pause">'+glyph(historyPaused?'play':'pause',12)+' '+(historyPaused?'Activar':'Pausar')+'</button>'+(history.length?'<button type="button" data-history="clear">Vaciar</button>':'')+'</div></div>'+(history.length?'<div class="clipboard-list">'+history.map(item=>'<div class="clipboard-item"><button type="button" class="clipboard-copy" data-copy="'+item.id+'" aria-label="Simular volver a copiar '+item.summary+'"><span class="clipboard-icon file-preview">'+(item.kind==='image'?'<img src="assets/windows-bloom.webp" alt="Miniatura de la imagen de Windows" width="30" height="22">':icon(item.kind==='text'?'clipboard':'file',15))+'</span><span><strong>'+item.summary+'</strong><small>'+item.origin+'</small></span></button><button type="button" class="clipboard-delete" data-delete="'+item.id+'" aria-label="Eliminar '+item.summary+' de la demo">'+icon('trash',13)+'</button></div>').join('')+'</div>':'<div class="clipboard-empty">'+icon('clipboard',19)+'<strong>'+(historyPaused?'Activá tu historial':'Lo que copies aparecerá acá')+'</strong><small>Texto, imágenes y archivos copiados</small></div>')+'</section>'
}
function layout() {
 const state=open&&!dragging?'peek':'idle'
 root.dataset.notch=state;root.dataset.dragging=String(!!dragging)
 surface.setAttribute('aria-expanded',String(open&&!dragging))
 const tab=page==='music'&&lyrics?(content.querySelector('.volume-popover')?'lyrics-volume':'lyrics'):'home'
 const size=pill?pillSize(state,'M',tab):notchSize('top',state,'M',tab)
 width=size[0];height=size[1]
 const maxWidth=page==='music'&&lyrics?598:483
 scale=Math.min(view==='hero'?1.15:1,(innerWidth-24)/maxWidth)
 if(!Number.isFinite(scale)||scale<=0)scale=.5
 root.style.setProperty('--demo-scale',scale)
 root.style.setProperty('--notch-width',width+'px');root.style.setProperty('--notch-height',height+'px')
 let x=innerWidth/2-width*scale/2,y=scene&&!pill?0:Math.max(8,(innerHeight-height*scale)/2-10)
 if(pill) {
   const area={x:0,y:0,width:innerWidth,height:innerHeight}
   const p=pillPosition(area,width*scale,height*scale,anchor,1);x=p.x;y=p.y
 }
 root.style.setProperty('--dock-left',x+'px');root.style.setProperty('--dock-top',y+'px')
 surface.querySelector('.system-status').hidden=pill&&state==='idle'
 resizeComet()
}
function setOpen(value) {if(dragging)return;open=value;layout()}
function switchPage(next) {page=next;setOpen(true);renderPage()}
function notice(text) {const el=document.querySelector('.demo-feedback');clearTimeout(feedbackTimer);el.textContent=text;el.classList.add('visible');feedbackTimer=setTimeout(()=>el.classList.remove('visible'),1800)}
const clock=n=>Math.floor(n/60)+':'+String(Math.floor(n%60)).padStart(2,'0')
function updatePlayback() {
 const input=content.querySelector('.timeline-scrubber input')
 if(input) {
  const percent=position/duration*100;input.value=position;input.setAttribute('aria-valuetext',clock(position)+' de 4:07')
  content.querySelector('.elapsed').textContent=clock(position)
  content.querySelector('.player-progress i').style.width=percent+'%'
  content.querySelector('.timeline-thumb').style.left=percent+'%'
  content.querySelector('.player-wave').classList.toggle('is-playing',playing)
  content.querySelectorAll('.demo-lyrics p').forEach((p,i)=>p.classList.toggle('active',i===Math.floor(position/7)%2))
 }
 desktop.querySelector('.bimu-eyes').classList.toggle('bimu-listening',playing)
 svg?.classList.toggle('is-playing',playing)
}
desktop.addEventListener('pointerenter',()=>{clearTimeout(hoverTimer);if(!open)hoverTimer=setTimeout(()=>setOpen(true),250)})
desktop.addEventListener('pointerleave',()=>{clearTimeout(hoverTimer);if(!dragging&&['music','pill'].includes(view))hoverTimer=setTimeout(()=>setOpen(false),250)})
desktop.addEventListener('focusin',()=>{if(!dragging)setOpen(true)})
desktop.addEventListener('focusout',()=>{if(['music','pill'].includes(view)&&!desktop.contains(document.activeElement)&&!desktop.matches(':hover'))setOpen(false)})
surface.addEventListener('click',()=>{if(!surface.dataset.dragged)setOpen(!open);delete surface.dataset.dragged})
desktop.addEventListener('click',event=>{
 const button=event.target.closest('button')
 if(!button)return
 if(button.matches('[data-page]'))switchPage(button.dataset.page)
 if(button.matches('.player-menu-arrow'))switchPage(pages[(pages.indexOf(page)+(button.classList.contains('player-menu-left')?-1:1)+pages.length)%pages.length])
 if(button.matches('.notch-retract-button')){button.blur();setOpen(false)}
 if(button.matches('.notch-settings-button'))parent.postMessage({source:'bimu-demo',command:'personalize'},location.origin)
 if(button.matches('.player-toggle')){playing=!playing;button.innerHTML=glyph(playing?'pause':'play',19);button.setAttribute('aria-label',(playing?'Pausar':'Reproducir')+' la demo');updatePlayback();animate()}
 if(button.matches('[data-media]')){position=(position+(button.dataset.media==='next'?20:-20)+duration)%duration;updatePlayback()}
 if(button.matches('.lyrics-toggle')){lyrics=!lyrics;renderPage()}
 if(button.matches('.volume-toggle')){const popup=content.querySelector('.volume-popover');if(popup)popup.remove();else{volumePanel.hidden=false;volumePanel.querySelector('input').value=volume;volumePanel.querySelector('input').style.setProperty('--volume-fill',volume+'%');button.parentElement.append(volumePanel)}button.setAttribute('aria-expanded',String(!popup));layout()}
 if(button.matches('[data-weather],.weather-refresh')){city=1-city;renderPage();notice('Pronóstico de demostración')}
 if(button.matches('[data-history]')){if(button.dataset.history==='clear')history=[];else historyPaused=!historyPaused;renderPage()}
 if(button.matches('[data-copy]')){button.querySelector('small').textContent='Copiado';notice('Elemento seleccionado en la demo')}
 if(button.matches('[data-delete]')){history=history.filter(item=>item.id!==Number(button.dataset.delete));renderPage()}
})

// Match the native timeline: freeze playback while scrubbing and render wave/thumb together.
function seekPointer(event){const input=event.target;if(!input.matches('.timeline-scrubber input'))return;const r=input.getBoundingClientRect();position=Math.max(0,Math.min(duration,(event.clientX-r.left)/r.width*duration));updatePlayback()}
desktop.addEventListener('pointerdown',event=>{if(!event.target.matches('.timeline-scrubber input')||event.button!==0)return;event.preventDefault();seeking=true;event.target.focus({preventScroll:true});event.target.setPointerCapture(event.pointerId);event.target.parentElement.classList.add('is-scrubbing');seekPointer(event)})
desktop.addEventListener('pointermove',event=>{if(seeking)seekPointer(event)})
function endSeek(event){if(!seeking)return;if(event.type==='pointerup')seekPointer(event);seeking=false;content.querySelector('.timeline-scrubber')?.classList.remove('is-scrubbing');progressAt=performance.now()}
desktop.addEventListener('pointerup',endSeek);desktop.addEventListener('pointercancel',endSeek)

desktop.addEventListener('input',event=>{
 if(event.target.matches('.timeline-scrubber input')){position=Number(event.target.value);updatePlayback()}
 if(event.target.matches('.volume-row input')){volume=Number(event.target.value);event.target.parentElement.querySelector('span').textContent=volume+'%';event.target.style.setProperty('--volume-fill',volume+'%')}
})
desktop.querySelector('.notch-move-button').addEventListener('pointerdown',event=>{
 if(!pill){notice('Probá el arrastre en la sección Modo pill');return}
 if(event.button!==0)return
 event.preventDefault();desktop.setPointerCapture(event.pointerId)
 dragging={id:event.pointerId};anchor={x:event.clientX/innerWidth,y:event.clientY/innerHeight};layout()
})
desktop.addEventListener('pointermove',event=>{
 if(dragging&&event.pointerId===dragging.id){anchor={x:event.clientX/innerWidth,y:event.clientY/innerHeight};layout();return}
 if(!motion||reduced.matches)return
 const eyes=desktop.querySelector('.bimu-eyes'),r=eyes.getBoundingClientRect()
 eyes.style.setProperty('--eye-x',Math.max(-2,Math.min(2,(event.clientX-r.left-r.width/2)/140))+'px')
 eyes.style.setProperty('--eye-y',Math.max(-1.5,Math.min(1.5,(event.clientY-r.top-r.height/2)/140))+'px')
})
function release(event){if(dragging&&event.pointerId===dragging.id){dragging=null;open=true;layout()}}
desktop.addEventListener('pointerup',release);desktop.addEventListener('pointercancel',release)

// Same fixed contour, fading segments and rounded head as src/MusicEdge.jsx.
const ns='http://www.w3.org/2000/svg'
const svg=document.createElementNS(ns,'svg')
svg.classList.add('bimu-music-edge','is-playing');svg.setAttribute('aria-hidden','true');svg.setAttribute('preserveAspectRatio','none')
svg.innerHTML='<defs><filter id="demo-glow" x="-20" y="-20" width="1000" height="1000" filterUnits="userSpaceOnUse"><feGaussianBlur stdDeviation="1.8"/></filter></defs><g class="orientation"><use href="#demo-trail" filter="url(#demo-glow)" class="music-comet-glow"/><g id="demo-trail" fill="none" stroke-linecap="round" stroke-linejoin="round"></g></g>'
surface.append(svg)
const trail=svg.querySelector('#demo-trail'),orientation=svg.querySelector('.orientation'),segments=[]
for(let i=0;i<24;i++){const path=document.createElementNS(ns,'path');path.setAttribute('pathLength','1000');path.setAttribute('stroke-width',.25+1.85*((i+1)/24)**2);path.setAttribute('stroke',i>=19?'var(--comet-head-color)':'var(--audio-accent)');trail.append(path);segments.push(path)}
let contour=null,frame=0,last=0,cycle=0,lastDraw=0,progressAt=0
function draw(head) {
 if(!contour)return
 const parts=(pill?loopCometSegments:cometSegments)(head,.36,24)
 parts.forEach((part,i)=>{const length=Math.max(0,part.end-part.start)*1000;segments[i].setAttribute('stroke-dasharray',length+' '+(pill?1000-length:2000));segments[i].setAttribute('stroke-dashoffset',-part.start*1000);segments[i].setAttribute('opacity',part.opacity)})
}
function resizeComet() {
 if(!svg)return
 const w=surface.clientWidth,h=surface.clientHeight,r=parseFloat(getComputedStyle(surface).borderBottomLeftRadius)||0
 contour=pill?pillContour(w,h,r):freeContour(w,h,'top',r)
 if(!contour)return
 svg.setAttribute('viewBox','0 0 '+w+' '+h);orientation.setAttribute('transform',contour.transform||'')
 segments.forEach(path=>path.setAttribute('d',contour.d));draw(reduced.matches?0.62:pill?cycle%1:cometHead(cycle,1))
}
function tick(now) {
 if(now-lastDraw>=32) {
  const dt=last?Math.min(100,now-last):0;last=now;lastDraw=now
  cycle+=dt/Math.max(4200,Math.min(10000,(contour?.length||300)/65*1000))
  draw(pill?cycle%1:cometHead(cycle,1))
  if(!seeking&&now-progressAt>250){position=(position+dt/1000+Math.max(0,(now-progressAt-dt))/1000)%duration;progressAt=now;updatePlayback()}
 }
 frame=requestAnimationFrame(tick)
}
function animate() {
 cancelAnimationFrame(frame);last=0;progressAt=performance.now()
 root.dataset.motion=motion?'playing':'paused'
 if(!visible||document.hidden||!motion||!playing||reduced.matches){draw(.62);return}
 frame=requestAnimationFrame(tick)
}
new ResizeObserver(resizeComet).observe(surface)
document.addEventListener('visibilitychange',animate)
reduced.addEventListener('change',()=>{motion=!reduced.matches;animate()})
window.addEventListener('resize',layout)
window.addEventListener('message',event=>{
 if(event.source!==parent||event.origin!==location.origin||event.data?.source!=='bimu-website')return
 const {command,value}=event.data
 if(command==='expand')setOpen(true)
 if(command==='weather'){city=1-city;switchPage('weather')}
 if(command==='reset'){history=[{id:1,kind:'image',summary:'Imagen · 1600 × 1000',origin:'Este equipo'},{id:2,kind:'text',summary:'Revisar la propuesta del viernes',origin:'Este equipo'},{id:3,kind:'file',summary:'Presentación.pdf',origin:'Notebook'}];switchPage('clipboard')}
 if(command==='visible'){visible=!!value;animate()}
 if(command==='motion'){motion=!!value;animate()}
 if(command==='tint'&&typeof value==='string'&&/^#[0-9a-f]{6}$/i.test(value)){const rgb=[1,3,5].map(n=>parseInt(value.slice(n,n+2),16));root.style.setProperty('--notch-rgb',rgb.join(','));root.dataset.theme=(rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722)>155?'light':'dark'}
})
function updateClock() {
 const now=new Date(),time=now.toLocaleTimeString('es-AR',{hour:'2-digit',minute:'2-digit',hour12:false})
 desktop.querySelectorAll('time').forEach(node=>node.textContent=time)
 desktop.querySelector('.player-date').textContent=now.toLocaleDateString('es-AR',{weekday:'short',day:'numeric',month:'short',year:'numeric'}).replaceAll('.','')
 const taskbar=document.querySelector('.taskbar-time');if(taskbar)taskbar.textContent=time+' · Windows 11'
}
renderPage();updateClock();setInterval(updateClock,60000);animate()
