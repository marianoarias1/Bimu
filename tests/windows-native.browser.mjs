// Connect to the actual packaged WebView2: no Tauri mocks and no Vite server.
import {chromium} from 'playwright-core'
import {mkdir} from 'node:fs/promises'
import {execFile} from 'node:child_process'
import {promisify} from 'node:util'
import assert from 'node:assert/strict'
const endpoint='http://127.0.0.1:9225';let browser,page,pointerHeld=false
const run=promisify(execFile)
// CDP pointer events do not set GetAsyncKeyState. Dragging must use the real
// Windows input state, which the transparent-window hit test relies on.
async function nativePointer(action,point){
 const move=point?`if(![BimuTestPointer]::SetCursorPos(${Math.round(point.x)},${Math.round(point.y)})){throw 'Could not move the Windows cursor'};Start-Sleep -Milliseconds 120;`:''
 const button=action==='down'?'[BimuTestPointer]::mouse_event(2,0,0,0,[UIntPtr]::Zero)':action==='up'?'[BimuTestPointer]::mouse_event(4,0,0,0,[UIntPtr]::Zero)':''
 await run('powershell.exe',['-NoProfile','-NonInteractive','-Command',`Add-Type @'
using System;using System.Runtime.InteropServices;
public static class BimuTestPointer{
 [DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")]public static extern void mouse_event(uint flags,uint dx,uint dy,uint data,UIntPtr extra);
}
'@
${move}${button}`],{timeout:10000,windowsHide:true})
}
await mkdir('test-results/native-windows',{recursive:true})
try{
 for(let i=0;i<100;i++){try{if((await fetch(endpoint+'/json/version',{signal:AbortSignal.timeout(1000)})).ok)break}catch{}await new Promise(r=>setTimeout(r,200));if(i===99)throw Error('Native WebView2 debugger did not start')}
 browser=await chromium.connectOverCDP(endpoint)
 for(let i=0;i<100;i++){page=browser.contexts().flatMap(context=>context.pages()).find(page=>!page.url().startsWith('devtools:'));if(page)break;await new Promise(r=>setTimeout(r,100))}
 assert.ok(page,'native WebView2 page');page.setDefaultTimeout(15000)
 // Keep foreground CI consoles from activating the unrelated auto-hide option.
 await page.waitForFunction(()=>document.querySelector('.desktop'))
 await page.evaluate(()=>localStorage.setItem('bimu-avoid-overlap','false'));await page.reload()
 // Hosted desktops can start with the physical cursor already over Bimu.
 await nativePointer('move',{x:0,y:0});await page.mouse.move(1,1)
 await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle')
 for(const mode of ['notch','pill']){
  if(mode==='pill'){
   await page.getByRole('button',{name:'Abrir compañero',exact:true}).click()
   await page.getByRole('button',{name:'Ajustes avanzados',exact:true}).click()
   await page.getByLabel('Diseño',{exact:true}).selectOption('pill')
   await page.getByRole('button',{name:'Volver al notch',exact:true}).click()
   await page.mouse.move(1,1);await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle')
  }
  const result=await page.evaluate(async()=>{
   const invoke=window.__TAURI_INTERNALS__.invoke
   const [media,status]=await Promise.all([invoke('media_state',{includeCover:false}),invoke('system_status')])
   return {mediaValid:media===null||typeof media.title==='string',statusValid:status&&Object.hasOwn(status,'wifiSignal')}
  })
  assert.ok(result.mediaValid&&result.statusValid,'real WinRT and system IPC must return successfully')
  await page.locator('.desktop').screenshot({path:`test-results/native-windows/${mode}-idle.png`})
  await page.getByRole('button',{name:'Abrir compañero',exact:true}).hover()
  await page.waitForFunction(()=>document.documentElement.dataset.notch==='peek')
  assert.equal(await page.getByText('No se pudo consultar la reproducción.',{exact:true}).count(),0)
  await page.locator('.desktop').screenshot({path:`test-results/native-windows/${mode}-open.png`})
  if(mode==='pill'){
   const handle=await page.locator('.notch-move-button').boundingBox()
   const frame=await page.evaluate(async()=>({position:await window.__TAURI_INTERNALS__.invoke('plugin:window|outer_position',{label:'main'}),scale:await window.__TAURI_INTERNALS__.invoke('plugin:window|scale_factor',{label:'main'})}))
   const cursor={x:frame.position.x+(handle.x+handle.width/2)*frame.scale,y:frame.position.y+(handle.y+handle.height/2)*frame.scale}
   pointerHeld=true;await nativePointer('down',cursor)
   await page.waitForFunction(()=>document.documentElement.dataset.dragging==='true'&&document.documentElement.dataset.notch==='idle')
   await page.waitForTimeout(350)
   const compact=await page.locator('.desktop').boundingBox();assert.ok(compact.width<=100&&compact.height<=36,JSON.stringify({compact,state:await page.evaluate(()=>({dragging:document.documentElement.dataset.dragging,notch:document.documentElement.dataset.notch}))}))
   const movedCursor={x:cursor.x+30,y:cursor.y+30};await nativePointer('move',movedCursor);await page.waitForTimeout(150)
   const actual=await page.evaluate(async()=>{const position=await window.__TAURI_INTERNALS__.invoke('plugin:window|outer_position',{label:'main'}),scale=await window.__TAURI_INTERNALS__.invoke('plugin:window|scale_factor',{label:'main'}),rect=document.querySelector('.desktop').getBoundingClientRect();return {x:position.x+(rect.left+rect.width/2)*scale,y:position.y+(rect.top+rect.height/2)*scale}})
   assert.ok(Math.abs(actual.x-movedCursor.x)<=2&&Math.abs(actual.y-movedCursor.y)<=2,`Actual pill center ${JSON.stringify(actual)} must follow Windows cursor ${JSON.stringify(movedCursor)}`)
   console.log('PASS actual Windows cursor anchor')
   await page.locator('.desktop').screenshot({path:'test-results/native-windows/pill-drag.png'})
   await nativePointer('up');pointerHeld=false;await page.waitForFunction(()=>document.documentElement.dataset.dragging==='false'&&document.documentElement.dataset.notch==='peek')
  }
  if(mode==='pill')await nativePointer('move',{x:0,y:0})
  await page.mouse.move(1,1);await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle')
  console.log('PASS actual Windows/WebView2',mode)
 }
}catch(error){if(page){console.log('Native failure state:',await page.evaluate(()=>({state:{...document.documentElement.dataset},text:document.body.innerText})).catch(()=>null));await page.screenshot({path:'test-results/native-windows/failure.png'}).catch(()=>{})}throw error}
finally{if(pointerHeld)await nativePointer('up').catch(()=>{});await browser?.close()}
