import {test} from 'node:test'
import assert from 'node:assert/strict'
import {musicEdgePaths} from '../src/music-edge.js'
const points=d=>d.slice(1).split(' L').map(p=>p.split(',').map(Number))
test('the audio contour leaves and returns to the silhouette without inward cuts',()=>{
 for(const length of [27,48,144,150,483,598])for(const phase of [0,.7,2,5])for(const level of [0,.5,1])for(const lateral of [false,true]){
  const {fill,line}=musicEdgePaths(length,phase,level,lateral),p=points(line),axis=lateral?0:1,along=1-axis
  assert.equal(p[0][axis],0);assert.equal(p.at(-1)[axis],0)
  assert.ok(p.every(v=>v[axis]>=0&&v[axis]<=1.601))
  assert.ok(p.every((v,i)=>!i||v[along]>p[i-1][along]))
  assert.ok(fill.endsWith(' Z'))
 }
})
test('phase animates smoothly and the contour remains bounded on bad input',()=>{
 const a=points(musicEdgePaths(150,0,1).line),b=points(musicEdgePaths(150,.01,1).line)
 assert.ok(a.some((p,i)=>p[1]!==b[i][1]));assert.ok(a.every((p,i)=>Math.abs(p[1]-b[i][1])<.02))
 for(const phase of [NaN,Infinity])assert.ok(!musicEdgePaths(150,phase,NaN).line.includes('NaN'))
 assert.deepEqual(musicEdgePaths(0),{fill:'',line:''})
})
