// Website only. Demos cannot call native commands, play audio, or read the clipboard.
const frames = [...document.querySelectorAll('.live-demo')]
const reduced = matchMedia('(prefers-reduced-motion: reduce)')
let paused = reduced.matches
function send(frame, command, value) {
  frame.contentWindow?.postMessage({source:'bimu-website',command,value}, location.origin)
}
function motion() {
  document.documentElement.dataset.motion = paused ? 'paused' : 'playing'
  const button = document.querySelector('[data-motion-toggle]')
  button.setAttribute('aria-pressed', String(paused))
  button.textContent = paused ? 'Activar animaciones' : 'Pausar animaciones'
  frames.forEach(frame => send(frame, 'motion', !paused))
}
const observer = new IntersectionObserver(entries => entries.forEach(entry => {
  entry.target.dataset.visible = String(entry.isIntersecting)
  send(entry.target, 'visible', entry.isIntersecting)
}), {rootMargin:'100px'})
frames.forEach(frame => {
  observer.observe(frame)
  frame.addEventListener('load', () => {
    send(frame,'motion',!paused)
    send(frame,'visible',frame.dataset.visible==='true')
  })
})
document.querySelector('[data-motion-toggle]').addEventListener('click', () => {paused=!paused;motion()})
reduced.addEventListener('change', event => {paused=event.matches;motion()})
document.querySelectorAll('[data-demo-command]').forEach(button => button.addEventListener('click', () => {
  const frame = frames.find(frame => new URL(frame.src).searchParams.get('view')===button.dataset.target)
  if(frame)send(frame,button.dataset.demoCommand)
}))
function tint(color) {
  const frame=frames.find(frame=>new URL(frame.src).searchParams.get('view')==='pill')
  if(frame)send(frame,'tint',color)
  document.querySelectorAll('[data-tint]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.tint===color)))
}
document.querySelectorAll('[data-tint]').forEach(button=>button.addEventListener('click',()=>tint(button.dataset.tint)))
document.querySelector('.custom-swatch input').addEventListener('input',event=>tint(event.target.value))
const eyes=document.querySelector('.brand-eyes')
let pointer=null,frame=0
document.addEventListener('pointermove',event=>{
  if(reduced.matches||paused)return
  pointer={x:event.clientX,y:event.clientY}
  if(frame)return
  frame=requestAnimationFrame(()=>{
    frame=0
    const r=eyes.getBoundingClientRect()
    eyes.style.setProperty('--eye-x',Math.max(-2,Math.min(2,(pointer.x-r.left-r.width/2)/140))+'px')
    eyes.style.setProperty('--eye-y',Math.max(-1.5,Math.min(1.5,(pointer.y-r.top-r.height/2)/140))+'px')
  })
},{passive:true})
document.addEventListener('pointerleave',()=>{eyes.style.setProperty('--eye-x','0px');eyes.style.setProperty('--eye-y','0px')})
motion()
window.addEventListener('message',event=>{
  if(event.origin!==location.origin||!frames.some(frame=>frame.contentWindow===event.source)||event.data?.source!=='bimu-demo')return
  if(event.data.command==='personalize'){document.getElementById('pill').scrollIntoView({behavior:reduced.matches?'instant':'smooth'});document.querySelector('.custom-swatch input').focus({preventScroll:true})}
})
