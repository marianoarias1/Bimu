// Real Chromium/Edge rendering of the production CSS. This checks intermediate
// frames, not merely the final layout, and deliberately needs no lyric service.
import {chromium} from 'playwright-core'
import {readFile,mkdir} from 'node:fs/promises'
import assert from 'node:assert/strict'
import {notchSize} from '../src/dock-geometry.js'
const css=await readFile(new URL('../src/desktop.css',import.meta.url),'utf8')
const browser=await chromium.launch({headless:true,...(process.platform==='win32'?{channel:'msedge'}:{}),args:['--no-sandbox']})
const page=await browser.newPage({viewport:{width:1000,height:700}})
await mkdir('test-results/notch-reveal',{recursive:true})
try{
 for(const reduced of [false,true]){
  await page.emulateMedia({reducedMotion:reduced?'reduce':'no-preference'})
  for(const edge of ['top','bottom','left','right'])for(const lyrics of [false,true]){
   const target=notchSize(edge,'peek','M',lyrics?'lyrics':'home')
   const compact=notchSize(edge,'idle','M',lyrics?'lyrics':'home')
   await page.setContent(`<html class="desktop-root" data-edge="${edge}" data-notch="idle" data-lyrics="${lyrics}" data-theme="dark"><head><style>${css}</style></head><body><div class="desktop compact"><div class="notch-surface"></div><section class="notch-player"><div class="notch-player-content"><div class="player-heading">Lun, 2 Nov 2026 <time>11:47</time></div><div class="notch-page"><div class="player-track"><span class="player-cover">♪</span><span><strong>Una canción con un título suficientemente largo</strong><small>Un artista</small></span></div><div class="player-timeline"><span>0:58</span><span class="player-progress"><i></i></span><span>2:47</span></div><div class="player-controls"><button>Anterior</button><button>Pausa</button><button>Siguiente</button></div>${lyrics?'<div class="lyrics-panel"><p>Una línea de canción sincronizada</p></div>':''}</div></div></section></div></body></html>`)
   await page.evaluate(({compact})=>{const s=document.documentElement.style;s.setProperty('--dock-duration','0ms');s.setProperty('--notch-width',compact[0]+'px');s.setProperty('--notch-height',compact[1]+'px');s.setProperty('--dock-left','30px');s.setProperty('--dock-top','30px');s.setProperty('--ui-opacity','1');s.setProperty('--notch-rgb','9,10,13')},{compact})
   await page.waitForTimeout(120)
   const frames=await page.evaluate(async({target,reduced})=>{
    const root=document.documentElement,viewport=document.querySelector('.notch-player'),content=document.querySelector('.notch-player-content'),track=document.querySelector('.player-track'),surface=document.querySelector('.notch-surface')
    root.style.setProperty('--dock-duration',reduced?'0ms':'260ms');root.dataset.notch='peek';root.style.setProperty('--notch-width',target[0]+'px');root.style.setProperty('--notch-height',target[1]+'px')
    const start=performance.now(),frames=[]
    await new Promise(resolve=>{function sample(now){const style=getComputedStyle(viewport),r=content.getBoundingClientRect();frames.push({ms:now-start,opacity:Number(style.opacity),visibility:style.visibility,width:r.width,track:track.getBoundingClientRect().height,delay:style.transitionDelay,overflow:style.overflowX,background:getComputedStyle(surface).backgroundSize});if(now-start<310)requestAnimationFrame(sample);else resolve()}requestAnimationFrame(sample)})
    return frames
   },{target,reduced})
   const label=`${edge} ${lyrics?'lyrics':'normal'} ${reduced?'reduced':'animated'}`
   const early=frames.filter(f=>f.ms>=35&&f.ms<=150)
   assert.ok(early.length,label+' sampled frames')
   assert.ok(early.every(f=>f.visibility==='visible'&&f.opacity>0),label+' must show content during opening, not after it')
   assert.ok(frames.every(f=>Math.abs(f.width-target[0])<1),label+' content width must stay stable throughout opening')
   assert.ok(frames.every(f=>f.delay==='0s'&&f.overflow==='hidden'),label+' no deferred reveal or escaping content')
   assert.ok(frames.every(f=>Math.abs(f.track-frames.at(-1).track)<1),label+' no text reflow during opening')
   if(lyrics)assert.ok(frames.every(f=>f.background==='100% 100%'),label+' body gradient always fills the current surface')
   // Reverse during a transition and reopen: no timer may leave it blank.
   await page.evaluate(()=>document.documentElement.dataset.notch='idle')
   await page.waitForTimeout(20)
   await page.evaluate(()=>document.documentElement.dataset.notch='peek')
   await page.waitForTimeout(65)
   assert.equal(await page.locator('.notch-player').evaluate(el=>getComputedStyle(el).visibility),'visible',label+' interrupted hover')
   assert.ok(await page.locator('.notch-player').evaluate(el=>Number(getComputedStyle(el).opacity)>0),label+' interrupted opacity')
   if(!reduced)await page.screenshot({path:`test-results/notch-reveal/${edge}-${lyrics?'lyrics':'normal'}.png`})
   console.log('PASS',label,frames.length,'frames')
  }
 }
}finally{await browser.close()}
