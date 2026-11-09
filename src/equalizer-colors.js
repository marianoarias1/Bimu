// Ensure both animated colors remain visible against the chosen solid notch color.
const parse = hex => /^#[\da-f]{6}$/i.test(hex) ? [1,3,5].map(i=>parseInt(hex.slice(i,i+2),16)) : [179,154,245]
const luminance = rgb => rgb.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0)
export const contrast = (a,b) => {const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)}
export function equalizerColors(background,swatch,neutral=false){
 const bg=background.map(Number),dark=contrast([255,255,255],bg)>=contrast([0,0,0],bg)
 const foreground=dark?[255,255,255]:[0,0,0],source=parse(neutral?'#b39af5':swatch)
 // Start with the selected hue and blend toward the contrasting end until readable.
 let accent=source
 for(let n=0;n<=20;n++){const t=n/20;accent=source.map((v,i)=>Math.round(v*(1-t)+foreground[i]*t));if(contrast(accent,bg)>=3)break}
 const accentInk=contrast([255,255,255],accent)>=contrast([0,0,0],accent)?'#fff':'#000'
 return {accentInk,base:`rgb(${foreground.join(',')})`,accent:`rgb(${accent.join(',')})`}
}
