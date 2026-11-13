// Connect to the actual packaged WebView2: no Tauri mocks and no Vite server.
import {chromium} from 'playwright-core'
import {mkdir} from 'node:fs/promises'
import assert from 'node:assert/strict'
const endpoint='http://127.0.0.1:9225';let browser,page
await mkdir('test-results/native-windows',{recursive:true})
try{
 for(let i=0;i<100;i++){try{if((await fetch(endpoint+'/json/version',{signal:AbortSignal.timeout(1000)})).ok)break}catch{}await new Promise(r=>setTimeout(r,200));if(i===99)throw Error('Native WebView2 debugger did not start')}
 browser=await chromium.connectOverCDP(endpoint)
 for(let i=0;i<100;i++){page=browser.contexts().flatMap(context=>context.pages()).find(page=>!page.url().startsWith('devtools:'));if(page)break;await new Promise(r=>setTimeout(r,100))}
 assert.ok(page,'native WebView2 page');page.setDefaultTimeout(15000)
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
   await page.mouse.move(handle.x+handle.width/2,handle.y+handle.height/2);await page.mouse.down()
   await page.waitForFunction(()=>document.documentElement.dataset.dragging==='true'&&document.documentElement.dataset.notch==='idle')
   await page.waitForTimeout(350)
   const compact=await page.locator('.desktop').boundingBox();assert.ok(compact.width<=100&&compact.height<=36)
   await page.locator('.desktop').screenshot({path:'test-results/native-windows/pill-drag.png'})
   await page.mouse.up();await page.waitForFunction(()=>document.documentElement.dataset.dragging==='false'&&document.documentElement.dataset.notch==='peek')
  }
  await page.mouse.move(1,1);await page.waitForFunction(()=>document.documentElement.dataset.notch==='idle')
  console.log('PASS actual Windows/WebView2',mode)
 }
}catch(error){if(page)await page.screenshot({path:'test-results/native-windows/failure.png'}).catch(()=>{});throw error}
finally{await browser?.close()}
