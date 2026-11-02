import {test} from 'node:test'
import assert from 'node:assert/strict'
import {registeredApp,selectableApps,moveShortcut,actionTitle,shortcutDescription,validShortcutDraft} from '../src/shortcut-ui.js'
const spotify={id:'spotify',name:'Spotify',aliases:[],available:true}
const chrome={id:'chrome',name:'Google Chrome',aliases:['Chrome'],available:true}
test('one app is a complete shortcut and can use the application name',()=>{
 const draft={name:'Spotify',actions:[{kind:'app',appId:'spotify'}]}
 assert.ok(validShortcutDraft(draft));assert.equal(shortcutDescription(draft,[spotify]),'Abre Spotify')
 assert.ok(!validShortcutDraft({...draft,actions:[]}));assert.ok(!validShortcutDraft({...draft,actions:[{kind:'app',appId:''}]}))
 assert.ok(!validShortcutDraft({...draft,actions:Array(13).fill(draft.actions[0])}))
})
test('picker merges configured/detected apps without duplicate registrations and tolerates accents',()=>{
 const found=[{id:'detected',name:'Spotify'},{id:'extra',name:'Edición'},{id:'chrome-other',name:'Chrome'}]
 assert.equal(registeredApp([spotify,chrome],found[0]).id,'spotify')
 assert.equal(selectableApps([spotify,chrome],found).length,3)
 assert.equal(selectableApps([spotify],found,'edicion')[0].name,'Edición')
 assert.equal(registeredApp([{...spotify,available:false}],found[0]),null)
})
test('reordering uses ids, preserves every shortcut and supports moving in both directions',()=>{
 const items=[{id:'work'},{id:'spotify'},{id:'game'}]
 assert.deepEqual(moveShortcut(items,'work','game'),['spotify','game','work'])
 assert.deepEqual(moveShortcut(items,'game','work'),['game','work','spotify'])
 assert.deepEqual(moveShortcut(items,'missing','work'),items.map(i=>i.id))
 assert.deepEqual(items.map(i=>i.id),['work','spotify','game'])
})
test('web/folder actions stay distinct, browser profiles survive display and invalid schemes cannot save',()=>{
 const action={kind:'url',url:'https://mail.google.com',browserId:'chrome',profileDirectory:'Profile 2'}
 const draft={name:'Modo trabajo',actions:[action,{kind:'folder',path:'C:\\Proyectos'}]}
 assert.ok(validShortcutDraft(draft));assert.equal(actionTitle(action,[]),'mail.google.com')
 assert.ok(shortcutDescription(draft,[]).includes('Proyectos'));assert.equal(action.profileDirectory,'Profile 2')
 for(const url of ['javascript:alert(1)','file:///C:/test','https://bad url'])assert.ok(!validShortcutDraft({name:'Web',actions:[{kind:'url',url}]}))
})
