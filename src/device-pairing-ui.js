// Presentation rules only; pairing and transfer authority stay in native code.
export function pairingView(status,route='start',allowNew=false){
 if(status.phase==='loading')return 'loading'
 if(status.phase==='connected')return 'connected'
 if(status.phase==='waiting')return 'code'
 if(status.linked&&!allowNew)return 'reconnecting'
 if(status.phase==='connecting'||status.phase==='disconnecting')return 'connecting'
 if(route==='choose')return 'start'
 if(route==='join')return 'join'
 if(route==='host')return 'create-error'
 if(status.linked)return 'reconnecting'
 if(route==='host')return 'create-error'
 return route==='join'?'join':'start'
}
export function canPair(address,code,peers,manual=false){
 return /^\d{6}$/.test(code)&&!!address.trim()&&(manual||peers.some(peer=>peer.address===address))
}
export function remainingCodeTime(expiresAt,now=Date.now()){
 if(!Number.isFinite(expiresAt)||!Number.isFinite(now))return null
 return Math.max(0,Math.ceil((expiresAt*1000-now)/1000))
}
export function codeTimeLabel(seconds){
 if(seconds==null)return 'Válido por dos minutos'
 return seconds===0?'Código vencido':`Vence en ${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')}`
}
export function limitSummary(status){
 const inherited=status.phase==='connected'&&status.usesMainLimit&&!status.isMain
 const connected=status.phase==='connected'
 return {
  value:connected?(status.effectiveLimitMb??status.transferLimitMb??30):(status.transferLimitMb??30),
  inherited,
  editLabel:inherited?'Mi límite':'Cambiar',
  source:inherited?`Definido por ${status.peer||'la PC principal'}`:connected&&status.usesMainLimit?'Se aplica a ambos equipos.':'Tu límite para enviar archivos.'
 }
}
export function validTransferLimit(value){return /^\d+$/.test(value)&&Number(value)>=1&&Number(value)<=250}
