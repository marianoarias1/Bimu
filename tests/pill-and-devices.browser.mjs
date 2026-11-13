// Production React/CSS integration with deterministic native IPC fixtures.
// Real Windows APIs (including Shell thumbnail extraction) are covered by Rust.
import {chromium} from 'playwright-core'
import {spawn} from 'node:child_process'
import {mkdir} from 'node:fs/promises'
import assert from 'node:assert/strict'
const port=5179,url=`http://127.0.0.1:${port}`
const server=spawn(process.execPath,['node_modules/vite/bin/vite.js','preview','--host','127.0.0.1','--port',String(port),'--strictPort'],{stdio:'pipe'})
let logs='';server.stdout.on('data',b=>logs=(logs+b).slice(-8000));server.stderr.on('data',b=>logs=(logs+b).slice(-8000))
let browser,activePage
try{
 for(let i=0;i<100;i++){try{if((await fetch(url)).ok)break}catch{}await new Promise(r=>setTimeout(r,100));if(i===99)throw Error(logs)}
 browser=await chromium.launch({headless:true,...(process.platform==='win32'?{channel:'msedge'}:{}),args:['--no-sandbox']})
 await mkdir('test-results/pill-devices',{recursive:true})
 async function pageFor(mode,theme,selected){
  const page=await browser.newPage({viewport:{width:1280,height:900},reducedMotion:'reduce'}),errors=[]
  activePage=page;page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(String(e)))
  await page.addInitScript(({mode,theme,selected})=>{
   const png='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVR4nGPo2RfwH4QZYAwAW8AKZWZiO2wAAAAASUVORK5CYII='
   localStorage.setItem('companion-dock',JSON.stringify({mode,edge:'top',monitorId:'Main:0:0',pillPositions:{'Main:0:0':{x:.5,y:.12}}}))
   localStorage.setItem('companion-theme',theme);localStorage.setItem('bimu-notch-color',theme==='light'?'#e9eddb':'#304c63')
   const ids=['music','shelf','modes','weather','clipboard','cleanup'];localStorage.setItem('bimu-pages-v1',JSON.stringify([{id:selected,enabled:true},...ids.filter(id=>id!==selected).map(id=>({id,enabled:false}))]))
   const monitor={name:'Main',position:{x:0,y:0},size:{width:1440,height:900},scaleFactor:1,workArea:{position:{x:0,y:0},size:{width:1440,height:860}}}
   let count=0;const callbacks={},devices=[{id:'a',name:'PC principal',favorite:true,connected:true,available:true,lastConnected:1700000000,management:true,pendingForget:false},{id:'b',name:'PC de trabajo',favorite:false,connected:false,available:true,lastConnected:1700000000,management:true,pendingForget:false}]
   const status={phase:'connected',peer:'PC principal',activeId:'a',devices,automatic:true,linked:true,localName:'Notebook',transferLimitMb:30,effectiveLimitMb:30,limitConfigured:false,isMain:false,usesMainLimit:false}
   window.isTauri=true;window.__bimuCalls=[];window.__bimuCursor={x:100,y:100};window.__bimuWindow={x:0,y:0};window.__mediaConcurrent=0;window.__mediaMax=0;window.__mediaMissing=false;window.__mediaEmptySamples=0;window.__mediaFailure=false;const thumbnails={}
   window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}}
   window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(cb){callbacks[++count]=cb;return count},unregisterCallback(id){delete callbacks[id]},async invoke(command,args={}){
    window.__bimuCalls.push({command,args})
    if(command==='plugin:event|listen')return ++count
    if(command.includes('|current_monitor')||command.includes('|primary_monitor'))return monitor
    if(command.includes('|available_monitors'))return [monitor]
    if(command.includes('|cursor_position'))return window.__bimuCursor
    if(command.includes('|outer_position')||command.includes('|inner_position'))return {...window.__bimuWindow}
    if(command.includes('|set_position')){const p=args.value.position||args.value.Physical||args.value.Logical;window.__bimuWindow={x:p.x,y:p.y};return null}
    if(command.includes('|scale_factor'))return 1
    if(command==='apps_list'||command==='shortcuts_list'||command==='chrome_profiles')return []
    if(command==='startup_sync')return {enabled:false,supported:true,automaticLaunch:false}
    if(command==='system_status')return {batteryPercent:null,wifiSignal:80,charging:false}
    if(command==='media_state'){if(!args.includeCover){window.__mediaConcurrent++;window.__mediaMax=Math.max(window.__mediaMax,window.__mediaConcurrent);if(selected==='music'&&theme==='dark')await new Promise(r=>setTimeout(r,1800));window.__mediaConcurrent--;if(window.__mediaFailure)throw Error('native service temporarily unavailable');if(window.__mediaMissing){window.__mediaEmptySamples++;return null}}return {title:'Canción de prueba',artist:'Artista',source:'spotify',playing:true,position:30,duration:180,seekRange:[0,180],sampledAtMs:Date.now(),cover:png}}
    if(command==='lyrics_lookup')return {status:'synced',syncedLyrics:'[00:00.00] Una primera línea\n[00:15.00] La música sigue acá\n[00:45.00] Una tercera línea'}
    if(command==='system_volume')return {level:50,muted:false}
    if(command==='shelf_list')return [{id:'a'.repeat(64),name:'flowers.png',path:'C:\\flowers.png',kind:'file',available:true,remote:false,online:true,preview_version:'1:1'},{id:'peer:'+'b'.repeat(64),name:'playa.png',path:'',kind:'file',available:true,remote:true,online:true,origin:'PC principal',peer_id:'peer-a',preview_version:'1:1'}]
    if(command==='shelf_thumbnail'||command==='clipboard_history_thumbnail'){const key=command+args.id;thumbnails[key]=(thumbnails[key]||0)+1;return thumbnails[key]===1?null:png}
    if(command==='clipboard_history_list')return {paused:false,items:[{id:1,kind:'image',summary:'Imagen · 524 × 173',preview:png,origin:'Este equipo',available:true},{id:2,kind:'files',summary:'flowers.png',preview:null,origin:'Este equipo',available:true}]}
    if(command==='device_pairing_status')return {...status,devices:devices.map(d=>({...d}))}
    if(command==='device_pairing_favorite'){for(const d of devices)d.favorite=d.id===args.id;return null}
    if(command==='device_pairing_disconnect'){status.phase='idle';status.automatic=false;for(const d of devices)d.connected=false;return null}
    if(command==='device_pairing_forget'){const d=devices.find(d=>d.id===args.id);if(d){d.pendingForget=true;d.connected=false;d.favorite=false;}status.phase='idle';return null}
    if(command==='device_pairing_connect'){for(const d of devices)d.connected=d.id===args.id;status.phase='connected';status.activeId=args.id;return null}
    return null
   }}
  },{mode,theme,selected})
  await page.goto(url);await page.waitForFunction(()=>document.documentElement.dataset.mode);await page.waitForFunction(()=>window.__bimuCalls.some(call=>call.command==='media_state'))
  return {page,errors}
 }
 for(const mode of ['notch','pill'])for(const theme of ['dark','light'])for(const selected of ['music','shelf','clipboard','cleanup']){
  const {page,errors}=await pageFor(mode,theme,selected)
  const surface=page.getByRole('button',{name:'Abrir compañero',exact:true})
  await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle')
  if(mode==='pill'){const rect=await page.locator('.desktop').boundingBox();assert.ok(rect.width<=100&&rect.height<=36);assert.equal(await surface.locator('.lucide-music-2').count(),0);assert.ok((await surface.locator('time').innerText()).length>0);assert.equal(await surface.locator('.system-status').count(),0)}
  if(mode==='pill')await page.locator('.desktop').screenshot({path:`test-results/pill-devices/${mode}-${theme}-${selected}-idle.png`})
  await surface.hover();await page.waitForFunction(()=>document.documentElement.dataset.notch==='peek');await page.waitForTimeout(180)
  if(selected==='music')await page.locator('.player-track strong').filter({hasText:'Canción de prueba'}).waitFor()
  if(selected==='shelf'||selected==='clipboard'){const selector=selected==='shelf'?'.shelf-open .file-preview img':'.clipboard-copy .file-preview img';await page.waitForFunction(selector=>Array.from(document.querySelectorAll(selector)).filter(img=>img.complete&&img.naturalWidth>0).length===2,selector)}
  assert.ok((await page.locator('.notch-page').innerText()).trim().length>0)
  assert.deepEqual(errors,[],`${mode} ${theme} ${selected}`)
  await page.locator('.desktop').screenshot({path:`test-results/pill-devices/${mode}-${theme}-${selected}.png`})
  if(selected==='music'){
   if(mode==='pill'){
    await page.emulateMedia({reducedMotion:'no-preference'});
    const rect=await page.locator('.notch-move-button').boundingBox()
    const cursorStart=await page.evaluate(rect=>({x:window.__bimuWindow.x+rect.x+rect.width/2,y:window.__bimuWindow.y+rect.y+rect.height/2}),rect)
    await page.evaluate(cursor=>{window.__bimuCursor=cursor},cursorStart);await page.mouse.move(rect.x+rect.width/2,rect.y+rect.height/2);await page.mouse.down();await page.waitForTimeout(350)
    assert.equal(await page.evaluate(()=>document.documentElement.dataset.notch),'idle','pressing the pill handle must collapse before movement')
    const compact=await page.locator('.desktop').boundingBox();assert.ok(compact.width<=100&&compact.height<=36)
    const placed=await page.evaluate(()=>{const r=document.querySelector('.desktop').getBoundingClientRect();return {x:window.__bimuWindow.x+r.x+r.width/2,y:window.__bimuWindow.y+r.y+r.height/2}})
    assert.ok(Math.abs(placed.x-cursorStart.x)<2&&Math.abs(placed.y-cursorStart.y)<2,'collapse must bring the compact pill under the cursor, not retain the expanded center')
    assert.ok(parseFloat(await page.locator('.desktop').evaluate(el=>getComputedStyle(el).transitionDuration))>0,'drag collapse retains the normal animation')
    const sizeCalls=await page.evaluate(()=>window.__bimuCalls.filter(c=>c.command.includes('|set_size')).length)
    let move=0
    for(const cursor of [{x:cursorStart.x+20,y:cursorStart.y+8},{x:0,y:0},{x:1439,y:0},{x:0,y:859},{x:1439,y:859},{x:1000,y:450}]){
     await page.evaluate(cursor=>{window.__bimuCursor=cursor},cursor);await page.mouse.move(rect.x+rect.width/2+(++move%2),rect.y+rect.height/2+(move%2));await page.waitForTimeout(80)
     const moved=await page.evaluate(()=>{const r=document.querySelector('.desktop').getBoundingClientRect();return {x:window.__bimuWindow.x+r.x,y:window.__bimuWindow.y+r.y,width:r.width,height:r.height}})
     const x=Math.max(6,Math.min(1440-6-moved.width,cursor.x-moved.width/2)),y=Math.max(6,Math.min(860-6-moved.height,cursor.y-moved.height/2))
     assert.ok(Math.abs(moved.x-x)<2&&Math.abs(moved.y-y)<2,'compact surface follows the cursor and reaches every screen edge')
    }
    assert.equal(await page.evaluate(()=>window.__bimuCalls.filter(c=>c.command.includes('|set_size')).length),sizeCalls,'movement must not resize the native frame')
    await page.mouse.up();await page.waitForTimeout(450)
    assert.equal(await page.evaluate(()=>document.documentElement.dataset.notch),'peek','release restores the expanded player')
    const point=await page.evaluate(()=>{const d=JSON.parse(localStorage.getItem('companion-dock'));return d.pillPositions[d.monitorId]});assert.ok(Math.abs(point.x-1000/1440)<.01);assert.ok(Math.abs(point.y-450/860)<.01)
    await page.mouse.move(1,1);await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle');await page.waitForTimeout(350)
    const saved=await page.evaluate(()=>{const r=document.querySelector('.desktop').getBoundingClientRect();return {x:window.__bimuWindow.x+r.x+r.width/2,y:window.__bimuWindow.y+r.y+r.height/2}})
    assert.ok(Math.abs(saved.x-1000)<2&&Math.abs(saved.y-450)<2,'the compact pill must keep the drop location after reopening and collapsing')


   }
   if(await page.evaluate(()=>document.documentElement.dataset.notch)!=='peek')await surface.hover();await page.waitForFunction(()=>document.documentElement.dataset.notch==='peek')
   if(theme==='dark'){
    assert.equal(await page.evaluate(()=>window.__mediaMax),1,'slow polls must not overlap')
    await page.evaluate(()=>{window.__mediaMissing=true});await page.waitForFunction(()=>window.__mediaEmptySamples>0);await page.waitForFunction(()=>!document.querySelector('.player-track strong')?.textContent.includes('Canción de prueba'));assert.ok(!(await page.locator('.player-track strong').innerText()).includes('Canción de prueba'),'an empty Windows sample must clear playback without a grace period');await page.evaluate(()=>{window.__mediaMissing=false})
    await page.evaluate(()=>{window.__mediaFailure=true});await page.waitForTimeout(2200);assert.equal(await page.getByText('No se pudo consultar la reproducción.',{exact:true}).count(),0);await page.evaluate(()=>{window.__mediaFailure=false})
   }
   await page.getByRole('button',{name:'Abrir ajustes',exact:true}).click();await page.getByRole('button',{name:'Ajustes avanzados',exact:true}).click()
   await page.getByRole('navigation',{name:'Secciones de ajustes'}).getByRole('button',{name:'General',exact:true}).click()
   await page.getByLabel('Diseño',{exact:true}).selectOption(mode==='pill'?'notch':'pill');await page.waitForFunction(target=>document.documentElement.dataset.mode===target,mode==='pill'?'notch':'pill')
   await page.getByRole('navigation',{name:'Secciones de ajustes'}).getByRole('button',{name:'Dispositivos',exact:true}).click()
   await page.getByRole('button',{name:'Marcar como favorito PC de trabajo',exact:true}).click();await page.getByRole('button',{name:'Quitar de favoritos PC de trabajo',exact:true}).waitFor()
   await page.locator('.pair-device-menu').last().locator('summary').click();
   const menu=page.locator('.pair-device-menu[open]'),trigger=await menu.locator('summary').boundingBox(),action=await menu.getByRole('button',{name:'Olvidar equipo',exact:true}).boundingBox();assert.ok(action.y-trigger.y-trigger.height>=6);assert.ok(action.height>=38);
   assert.ok(!['transparent','rgba(0, 0, 0, 0)'].includes(await menu.locator('button').evaluate(el=>getComputedStyle(el).backgroundColor)))
   await page.locator('.desktop').screenshot({path:`test-results/pill-devices/${mode}-${theme}-forget-menu.png`})
   await page.getByRole('button',{name:'Olvidar equipo',exact:true}).click();await page.getByRole('alertdialog').waitFor();await page.getByRole('button',{name:'Cancelar',exact:true}).click()
   await page.locator('.desktop').screenshot({path:`test-results/pill-devices/${mode}-${theme}-devices.png`});assert.deepEqual(errors,[])
   await page.getByRole('button',{name:'+ Vincular otra PC',exact:true}).click();await page.getByRole('button',{name:'Generar código',exact:true}).waitFor();await page.getByRole('button',{name:'Volver',exact:true}).click();await page.getByLabel('Equipos guardados',{exact:true}).waitFor()
  }
  console.log('PASS',mode,theme,selected);await page.close()
 }
}catch(error){if(activePage&&!activePage.isClosed()){await activePage.screenshot({path:'test-results/pill-devices/failure.png',fullPage:true}).catch(()=>{});console.log('UI failure state:',await activePage.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();server.kill()}
