// One slow native sample must finish before another poll can invalidate it.
export function singleFlight(task){
 let pending=null
 return (...args)=>{if(pending)return pending;pending=Promise.resolve().then(()=>task(...args)).finally(()=>{pending=null});return pending}
}
export function sameTrack(a,b){return !!a&&!!b&&a.source===b.source&&a.title===b.title&&a.artist===b.artist}
export function retainMissingMedia(previous,lastSeen,now,grace=6000){return previous&&now-lastSeen<grace?previous:null}
