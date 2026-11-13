import {chromium} from 'playwright-core'
import {createServer} from 'node:http'
import {readFile,mkdir} from 'node:fs/promises'
import {resolve,extname} from 'node:path'
import assert from 'node:assert/strict'
const root=resolve('docs'),shots=resolve('test-results/website')
const types={'.html':'text/html;charset=utf-8','.js':'text/javascript;charset=utf-8','.css':'text/css;charset=utf-8','.webp':'image/webp','.woff2':'font/woff2','.woff':'font/woff'}
const server=createServer(async(req,res)=>{
 try{
  const pathname=new URL(req.url,'http://localhost').pathname
  const path=resolve(root,'.'+decodeURIComponent(pathname==='/'?'/index.html':pathname))
  if(path!==root&&!path.startsWith(root+'/')){res.writeHead(403).end();return}
  const data=await readFile(path);res.writeHead(200,{'Content-Type':types[extname(path)]||'application/octet-stream'}).end(data)
 }catch{res.writeHead(404).end('Not found')}
})
await new Promise(done=>server.listen(0,'127.0.0.1',done))
const url='http://127.0.0.1:'+server.address().port
let browser,page
const errors=[]
try{
 await mkdir(shots,{recursive:true})
 browser=await chromium.launch({headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],...(process.env.BIMU_BROWSER_PATH?{executablePath:process.env.BIMU_BROWSER_PATH}:{})})
 page=await browser.newPage({viewport:{width:1440,height:1000}})
 page.on('pageerror',error=>errors.push(String(error)))
 page.on('response',response=>{if(response.status()>=400)errors.push(response.status()+' '+response.url())})
 await page.goto(url,{waitUntil:'networkidle'})
 await page.evaluate(()=>document.fonts.ready)
 const hero=page.frameLocator('iframe[src$="view=hero"]')
 await hero.getByRole('button',{name:'Pausar la demo',exact:true}).waitFor()
 for(const frameEl of await page.locator('.live-demo').all()){
  await frameEl.scrollIntoViewIfNeeded();await frameEl.contentFrame().locator('.desktop').waitFor()
 }
 await page.evaluate(()=>scrollTo(0,0))
 await page.screenshot({path:shots+'/desktop.png',fullPage:true})
 const eyes=page.locator('.brand-eyes')
 await page.mouse.move(1400,800);await page.waitForTimeout(150)
 assert.notEqual(await eyes.evaluate(el=>getComputedStyle(el).getPropertyValue('--eye-x').trim()),'0px')
 await page.getByRole('navigation',{name:'Navegación principal'}).getByRole('link',{name:'Funciones',exact:true}).hover()
 assert.notEqual(await eyes.locator('i').first().evaluate(el=>getComputedStyle(el).clipPath),'none','header and nav hover use Bimu happy eyes')
 const seek=hero.getByRole('slider',{name:'Posición de reproducción simulada'})
 await hero.getByRole('button',{name:'Pausar la demo',exact:true}).click()
 const paused=Number(await seek.inputValue());await page.waitForTimeout(400)
 assert.equal(Number(await seek.inputValue()),paused)
 await seek.focus();await seek.press('Home');await seek.press('ArrowRight');assert.ok(Math.abs(Number(await seek.inputValue())-.1)<.02)
 await hero.getByRole('button',{name:'Reproducir la demo',exact:true}).click()
 await page.waitForTimeout(400);assert.ok(Number(await seek.inputValue())>.1)
 await hero.getByRole('button',{name:'Mostrar volumen de la demo'}).click()
 assert.equal(await hero.locator('html').evaluate(el=>el.style.getPropertyValue('--notch-height')),'206px')
 assert.equal(await hero.locator('.volume-row input').evaluate(el=>el.style.getPropertyValue('--volume-fill')),'65%')
 await page.waitForTimeout(350)
 const popupBox=await hero.locator('.volume-popover').boundingBox(), dockBox=await hero.locator('.desktop').boundingBox()
 assert.ok(popupBox.y+popupBox.height<=dockBox.y+dockBox.height+1,'lyrics volume fits in native expanded height')
 assert.equal(await hero.locator('.notch-move-button circle').count(),9)
 const volume=hero.getByRole('slider',{name:'Volumen simulado'});await volume.focus();await volume.press('End');await volume.press('ArrowLeft')
 assert.ok((await hero.locator('.volume-row').innerText()).includes('99%'))
 await hero.getByRole('button',{name:'Mostrar volumen de la demo'}).click()
 const clipboardEl=page.locator('iframe[src$="view=clipboard"]')
 await clipboardEl.scrollIntoViewIfNeeded()
 const clipboard=page.frameLocator('iframe[src$="view=clipboard"]')
 await clipboard.locator('.clipboard-item').first().waitFor()
 assert.equal(await clipboard.locator('.clipboard-item').count(),3)
 assert.ok(await clipboard.locator('.clipboard-icon img').evaluate(el=>el.complete&&el.naturalWidth>0))
 await clipboard.locator('[data-copy]').first().click()
 assert.ok((await clipboard.locator('.clipboard-item').first().innerText()).includes('Copiado'))
 await clipboard.getByRole('button',{name:'Vaciar',exact:true}).click()
 assert.equal(await clipboard.locator('.clipboard-item').count(),0)
 await page.getByRole('button',{name:'Reiniciar el historial de la demo'}).click()
 await clipboard.locator('.clipboard-item').first().waitFor()
 assert.equal(await clipboard.locator('.clipboard-item').count(),3)
 await page.locator('iframe[src$="view=weather"]').scrollIntoViewIfNeeded()
 const weather=page.frameLocator('iframe[src$="view=weather"]')
 await weather.getByRole('button',{name:'Buenos Aires',exact:true}).waitFor()
 await page.getByRole('button',{name:'Cambiar ciudad en la demo'}).click()
 await weather.getByRole('button',{name:'Córdoba',exact:true}).waitFor()
 const pillEl=page.locator('iframe[src$="view=pill"]')
 await pillEl.scrollIntoViewIfNeeded()
 const pill=page.frameLocator('iframe[src$="view=pill"]')
 assert.equal(await pill.locator('.notch-settings-button').isVisible(),false,'idle pill hides settings')
 assert.equal(await pill.locator('.notch-retract-button').count(),0,'pill has no notch retract action')
 await page.getByRole('button',{name:'Probar la pill',exact:true}).click()
 await pill.locator('.notch-move-button').waitFor({state:'visible'})
 await page.getByRole('button',{name:'Color azul',exact:true}).click()
 assert.equal(await pill.locator('html').evaluate(el=>el.style.getPropertyValue('--notch-rgb')),'48,76,99')
 await page.locator('.custom-swatch input').evaluate(el=>{el.value='#f6edde';el.dispatchEvent(new Event('input',{bubbles:true}))})
 assert.equal(await pill.locator('html').getAttribute('data-theme'),'light')
 const handle=await pill.locator('.notch-move-button').boundingBox()
 await page.mouse.move(handle.x+handle.width/2,handle.y+handle.height/2)
 await page.mouse.down();await page.waitForTimeout(300)
 assert.equal(await pill.locator('html').getAttribute('data-notch'),'idle')
 const frameBox=await pillEl.boundingBox()
 await page.mouse.move(frameBox.x+frameBox.width-8,frameBox.y+frameBox.height-8,{steps:5})
 await page.mouse.up();await page.waitForTimeout(400)
 assert.equal(await pill.locator('html').getAttribute('data-dragging'),'false')
 const coords=await pill.locator('.desktop').boundingBox()
 assert.ok(coords.x>=frameBox.x-1&&coords.y>=frameBox.y-1&&coords.x+coords.width<=frameBox.x+frameBox.width+1)
 await pillEl.screenshot({path:shots+'/pill-interactive.png'})
 for(const width of [1440,1024,768,390,320]){
  await page.setViewportSize({width,height:900})
  await page.goto(url,{waitUntil:'networkidle'})
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no page overflow at '+width)
  for(const frameEl of await page.locator('.live-demo').all()){
   await frameEl.scrollIntoViewIfNeeded()
   const child=frameEl.contentFrame()
   await child.locator('.desktop').waitFor()
   assert.ok(await child.locator('.desktop').evaluate(el=>{const r=el.getBoundingClientRect();return r.width>20&&r.height>10&&r.left>=-1&&r.right<=innerWidth+1}),'Bimu fits at '+width)
  }
  await page.screenshot({path:shots+'/width-'+width+'.png',fullPage:true})
 }
 const reducedPage=await browser.newPage({viewport:{width:390,height:844},reducedMotion:'reduce'})
 await reducedPage.goto(url,{waitUntil:'networkidle'})
 assert.equal(await reducedPage.getByRole('button',{name:'Activar animaciones',exact:true}).getAttribute('aria-pressed'),'true')
 assert.ok(await reducedPage.locator('h1').isVisible())
 await reducedPage.close()
 assert.deepEqual(errors,[])
 console.log('PASS website: assets, header eyes, playback/pause/seek/volume, weather, clipboard, pill drag/colors, five widths and reduced motion')
}catch(error){
 if(page)await page.screenshot({path:shots+'/failure.png',fullPage:true}).catch(()=>{})
 throw error
}finally{
 await browser?.close();await new Promise(done=>server.close(done))
}
