// Keep display metadata intact; remove only explicit video/channel decoration.
const decoration=/\s*[([]\s*(?:official\s+(?:(?:music|lyric)\s+)?(?:video|audio)|lyrics?|lyric\s+video|video\s+oficial|audio\s+oficial|visuali[sz]er)\s*[)\]]/gi
export function lyricsQuery(title,artist,duration=0){
 let track=String(title||'').replace(decoration,'').trim()
 let performer=String(artist||'').replace(/\s*-\s*Topic\s*$/i,'').trim()
 for(const separator of [' - ',' – ',' — ']){
  const split=track.indexOf(separator)
  if(split>0&&track.slice(0,split).trim().toLocaleLowerCase()===performer.toLocaleLowerCase())track=track.slice(split+separator.length).trim()
 }
 return {title:track,artist:performer,duration:Number.isFinite(duration)&&duration>0?Math.round(duration):0}
}
export function lyricsFailure(error){
 const kind=error&&typeof error==='object'?error.kind:''
 if(kind==='offline')return 'No pudimos conectarnos para buscar la letra. Revisá tu conexión y volvé a intentar.'
 if(kind==='timeout')return 'La búsqueda está tardando más de lo esperado. Probá de nuevo en un momento.'
 if(kind==='invalid')return 'Todavía no tenemos los datos necesarios para buscar esta canción.'
 return 'Las letras no están disponibles por un momento. Podés seguir escuchando y volver a intentar.'
}
// Deduplicate overlapping lookups (including StrictMode) and cache only a bounded
// set. No polling or background fetching while lyrics are disabled.
export function createLyricsLoader(lookup,{now=Date.now,maxEntries=24}={}){
 const entries=new Map()
 return query=>{
  const key=JSON.stringify([query.title,query.artist,query.duration])
  const old=entries.get(key)
  if(old&&(old.pending||old.until>now()))return old.promise
  entries.delete(key)
  const entry={pending:true,until:0,promise:null}
  entry.promise=Promise.resolve().then(()=>lookup(query)).then(result=>{
   entry.pending=false;entry.until=now()+(result.status==='synced'||result.status==='instrumental'?2*60*60*1000:2*60*1000)
   return result
  },error=>{entry.pending=false;entry.until=now()+Math.min(60,Math.max(5,Number(error?.retryAfterSeconds)||15))*1000;throw {kind:error?.kind||'temporary',retryAfterSeconds:Math.ceil((entry.until-now())/1000),retryAt:entry.until}})
  entries.set(key,entry)
  while(entries.size>maxEntries)entries.delete(entries.keys().next().value)
  return entry.promise
 }
}
