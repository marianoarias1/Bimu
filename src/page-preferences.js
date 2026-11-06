export const PAGE_DEFINITIONS=[['music','Música'],['shelf','Archivos'],['modes','Atajos rápidos'],['weather','Clima'],['clipboard','Portapapeles'],['cleanup','Limpieza']]
export const DEFAULT_PAGES=PAGE_DEFINITIONS.map(([id])=>({id,enabled:true}))
export function normalizePages(value){
 const seen=new Set(),out=[]
 if(Array.isArray(value))for(const item of value){if(item&&PAGE_DEFINITIONS.some(([id])=>id===item.id)&&!seen.has(item.id)){seen.add(item.id);out.push({id:item.id,enabled:item.enabled!==false})}}
 for(const item of DEFAULT_PAGES)if(!seen.has(item.id))out.push({...item})
 if(!out.some(item=>item.enabled))out[0].enabled=true
 return out
}
export function togglePage(pages,id){const count=pages.filter(p=>p.enabled).length;return pages.map(p=>p.id===id?{...p,enabled:p.enabled&&count===1?true:!p.enabled}:p)}
export function movePage(pages,from,to){if(from===to||!pages.some(p=>p.id===from)||!pages.some(p=>p.id===to))return pages;const result=pages.slice(),index=result.findIndex(p=>p.id===from),target=result.findIndex(p=>p.id===to),[item]=result.splice(index,1);result.splice(target,0,item);return result}
export function readPages(){try{return normalizePages(JSON.parse(localStorage.getItem('bimu-pages-v1')||'null'))}catch{return normalizePages(null)}}
