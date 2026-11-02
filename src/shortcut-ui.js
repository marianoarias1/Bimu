export const appNameKey=value=>(value||'').normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLowerCase().trim()
export function registeredApp(items,candidate){
 return items.find(item=>item.available&&item.id===candidate.id)||items.find(item=>item.available&&[item.name,...(item.aliases||[])].some(n=>appNameKey(n)===appNameKey(candidate.name)))||null
}
export function selectableApps(items,discovered,query=''){
 const known=items.filter(item=>item.available).map(item=>({...item,registered:true}))
 const extra=discovered.filter(item=>!registeredApp(items,item)).map(item=>({...item,available:true,registered:false}))
 const key=appNameKey(query)
 return [...known,...extra].filter(item=>appNameKey(item.name).includes(key)).sort((a,b)=>a.name.localeCompare(b.name,undefined,{sensitivity:'base'}))
}
export function moveShortcut(items,id,target){
 const from=items.findIndex(item=>item.id===id),to=items.findIndex(item=>item.id===target)
 if(from<0||to<0)return items.map(item=>item.id)
 const ids=items.map(item=>item.id);ids.splice(to,0,ids.splice(from,1)[0]);return ids
}
export function actionTitle(step,apps){
 if(step.kind==='app')return apps.find(app=>app.id===step.appId)?.name||'Aplicación no encontrada'
 if(step.kind==='url'){try{return new URL(step.url).hostname}catch{return step.url||'Página web'}}
 return (step.path||'').replace(/[\\/]+$/,'').split(/[\\/]/).pop()||'Carpeta'
}
export function shortcutDescription(item,apps){
 if(item.actions.length===1){const step=item.actions[0];return 'Abre '+actionTitle(step,apps)}
 return item.actions.length+' acciones · '+item.actions.map(step=>actionTitle(step,apps)).slice(0,3).join(', ')+(item.actions.length>3?'…':'')
}
export function validShortcutDraft(draft){
 return !!draft.name.trim()&&draft.name.trim().length<=80&&draft.actions.length>=1&&draft.actions.length<=12&&draft.actions.every(step=>{
  if(step.kind==='app')return !!step.appId
  if(step.kind==='folder')return !!step.path
  if(step.kind!=='url'||!step.url||/\s/.test(step.url))return false
  try{return ['http:','https:'].includes(new URL(step.url).protocol)}catch{return false}
 })
}
