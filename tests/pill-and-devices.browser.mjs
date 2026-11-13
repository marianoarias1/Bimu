// Production React/CSS integration with deterministic native IPC fixtures.
// Real Windows APIs (including Shell thumbnail extraction) are covered by Rust.
import {chromium} from 'playwright-core'
import {spawn} from 'node:child_process'
import {mkdir} from 'node:fs/promises'
import assert from 'node:assert/strict'
const port=5179,url=`http://127.0.0.1:${port}`
const server=spawn(process.execPath,['node_modules/vite/bin/vite.js','preview','--host','127.0.0.1','--port',String(port),'--strictPort'],{stdio:'pipe'})
let logs='';server.stdout.on('data',b=>logs=(logs+b).slice(-8000));server.stderr.on('data',b=>logs=(logs+b).slice(-8000))
let browser
try{
 for(let i=0;i<100;i++){try{if((await fetch(url)).ok)break}catch{}await new Promise(r=>setTimeout(r,100));if(i===99)throw Error(logs)}
 browser=await chromium.launch({headless:true,...(process.platform==='win32'?{channel:'msedge'}:{}),args:['--no-sandbox']})
 await mkdir('test-results/pill-devices',{recursive:true})
 async function pageFor(mode,theme,selected){
  const page=await browser.newPage({viewport:{width:1280,height:900},reducedMotion:'reduce'}),errors=[]
  page.on('pageerror',e=>errors.push(String(e)))
  await page.addInitScript(({mode,theme,selected})=>{
   const png='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVR4nGPo2RfwH4QZYAwAW8AKZWZiO2wAAAAASUVORK5CYII='
   localStorage.setItem('companion-dock',JSON.stringify({mode,edge:'top',monitorId:'Main:0:0',pillPositions:{'Main:0:0':{x:.5,y:.12}}}))
   localStorage.setItem('companion-theme',theme);localStorage.setItem('bimu-notch-color',theme==='light'?'#e9eddb':'#304c63')
   const ids=['music','shelf','modes','weather','clipboard','cleanup'];localStorage.setItem('bimu-pages-v1',JSON.stringify([{id:selected,enabled:true},...ids.filter(id=>id!==selected).map(id=>({id,enabled:false}))]))
   const monitor={name:'Main',position:{x:0,y:0},size:{width:1440,height:900},scaleFactor:1,workArea:{position:{x:0,y:0},size:{width:1440,height:860}}}
   let count=0;const callbacks={},devices=[{id:'a',name:'PC principal',favorite:true,connected:true,available:true,lastConnected:1700000000,management:true,pendingForget:false},{id:'b',name:'PC de trabajo',favorite:false,connected:false,available:true,lastConnected:1700000000,management:true,pendingForget:false}]
   const status={phase:'connected',peer:'PC principal',activeId:'a',devices,automatic:true,linked:true,localName:'Notebook',transferLimitMb:30,effectiveLimitMb:30,limitConfigured:false,isMain:false,usesMainLimit:false}
   window.__bimuCalls=[]
   window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}}
   window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(cb){callbacks[++count]=cb;return count},unregisterCallback(id){delete callbacks[id]},async invoke(command,args={}){
    window.__bimuCalls.push({command,args})
    if(command==='plugin:event|listen')return ++count
    if(command.includes('|current_monitor')||command.includes('|primary_monitor'))return monitor
    if(command.includes('|available_monitors'))return [monitor]
    if(command.includes('|cursor_position')||command.includes('|outer_position')||command.includes('|inner_position'))return {x:0,y:0}
    if(command.includes('|scale_factor'))return 1
    if(command==='apps_list'||command==='shortcuts_list'||command==='chrome_profiles')return []
    if(command==='startup_sync')return {enabled:false,supported:true,automaticLaunch:false}
    if(command==='system_status')return {batteryPercent:null,wifiSignal:80,charging:false}
    if(command==='media_state')return {title:'Canción de prueba',artist:'Artista',source:'spotify',playing:true,position:30,duration:180,seekRange:[0,180],sampledAtMs:Date.now(),cover:png}
    if(command==='lyrics_lookup')return {status:'synced',syncedLyrics:'[00:00.00] Una primera línea\n[00:15.00] La música sigue acá\n[00:45.00] Una tercera línea'}
    if(command==='system_volume')return {level:50,muted:false}
    if(command==='shelf_list')return [{id:'a'.repeat(64),name:'flowers.png',path:'C:\\flowers.png',kind:'file',available:true,remote:false,online:true,preview_version:'1:1'},{id:'peer:'+'b'.repeat(64),name:'playa.png',path:'',kind:'file',available:true,remote:true,online:true,origin:'PC principal',peer_id:'peer-a',preview_version:'1:1'}]
    if(command==='shelf_thumbnail'||command==='clipboard_history_thumbnail')return png
    if(command==='clipboard_history_list')return {paused:false,items:[{id:1,kind:'image',summary:'Imagen · 524 × 173',preview:png,origin:'Este equipo',available:true},{id:2,kind:'files',summary:'flowers.png',preview:null,origin:'Este equipo',available:true}]}
    if(command==='device_pairing_status')return {...status,devices:devices.map(d=>({...d}))}
    if(command==='device_pairing_favorite'){for(const d of devices)d.favorite=d.id===args.id;return null}
    if(command==='device_pairing_disconnect'){status.phase='idle';status.automatic=false;for(const d of devices)d.connected=false;return null}
    if(command==='device_pairing_forget'){const d=devices.find(d=>d.id===args.id);if(d){d.pendingForget=true;d.connected=false;d.favorite=false;}status.phase='idle';return null}
    if(command==='device_pairing_connect'){for(const d of devices)d.connected=d.id===args.id;status.phase='connected';status.activeId=args.id;return null}
    return null
   }}
  },{mode,theme,selected})
  await page.goto(url);await page.waitForFunction(()=>document.documentElement.dataset.mode)
  return {page,errors}
 }
 for(const mode of ['notch','pill'])for(const theme of ['dark','light'])for(const selected of ['music','shelf','clipboard','cleanup']){
  const {page,errors}=await pageFor(mode,theme,selected)
  const surface=page.getByRole('button',{name:'Abrir compañero',exact:true})
  await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle')
  if(mode==='pill'){const rect=await page.locator('.desktop').boundingBox();assert.ok(rect.width<=140&&rect.height<=30);assert.equal(await surface.locator('.lucide-music-2').count(),0);assert.ok((await surface.locator('time').innerText()).length>0)}
  await surface.hover();await page.waitForFunction(()=>document.documentElement.dataset.notch==='peek');await page.waitForTimeout(180)
  if(selected==='shelf'||selected==='clipboard'){const selector=selected==='shelf'?'.shelf-open .file-preview img':'.clipboard-copy .file-preview img';await page.waitForFunction(selector=>Array.from(document.querySelectorAll(selector)).filter(img=>img.complete&&img.naturalWidth>0).length===2,selector)}
  assert.ok((await page.locator('.notch-page').innerText()).trim().length>0)
  assert.deepEqual(errors,[],`${mode} ${theme} ${selected}`)
  await page.locator('.desktop').screenshot({path:`test-results/pill-devices/${mode}-${theme}-${selected}.png`})
  if(selected==='music'){
   await page.getByRole('button',{name:'Abrir ajustes',exact:true}).click();await page.getByRole('button',{name:'Ajustes avanzados',exact:true}).click()
   await page.getByRole('navigation',{name:'Secciones de ajustes'}).getByRole('button',{name:'General',exact:true}).click()
   await page.getByLabel('Diseño',{exact:true}).selectOption(mode==='pill'?'notch':'pill');await page.waitForFunction(target=>document.documentElement.dataset.mode===target,mode==='pill'?'notch':'pill')
   await page.getByRole('navigation',{name:'Secciones de ajustes'}).getByRole('button',{name:'Dispositivos',exact:true}).click()
   await page.getByRole('button',{name:'Marcar como favorito PC de trabajo',exact:true}).click();await page.getByRole('button',{name:'Quitar de favoritos PC de trabajo',exact:true}).waitFor()
   await page.locator('.pair-device-menu').last().locator('summary').click();await page.getByRole('button',{name:'Olvidar equipo',exact:true}).click();await page.getByRole('alertdialog').waitFor();await page.getByRole('button',{name:'Cancelar',exact:true}).click()
   await page.locator('.desktop').screenshot({path:`test-results/pill-devices/${mode}-${theme}-devices.png`});assert.deepEqual(errors,[])
  }
  console.log('PASS',mode,theme,selected);await page.close()
 }
}finally{await browser?.close();server.kill()}
