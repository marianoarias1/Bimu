//! Recover transparent RGBA from black/white icon renders, including legacy
//! masked icons. Fixed 32px bounds; does not inspect user files or allocate a cap.
pub const SIDE:usize=32;
pub fn rgba_from_mattes(black:&[u8],white:&[u8])->Option<Vec<u8>>{
 if black.len()!=SIDE*SIDE*4||white.len()!=black.len(){return None}
 let mut out=vec![0;black.len()];
 for i in (0..black.len()).step_by(4){
  let delta=(0..3).map(|c|white[i+c].saturating_sub(black[i+c])).max()?;
  let alpha=255-delta;out[i+3]=alpha;
  if alpha>0{for (dest,source) in [(0,2),(1,1),(2,0)]{out[i+dest]=((black[i+source]as u32*255+alpha as u32/2)/alpha as u32).min(255)as u8;}}
 }Some(out)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn transparent_background_and_opaque_black_remain_distinct(){
  let black=vec![0;SIDE*SIDE*4];let white=vec![255;black.len()];assert!(rgba_from_mattes(&black,&white).unwrap().iter().all(|b|*b==0));
  let result=rgba_from_mattes(&black,&black).unwrap();assert_eq!(&result[..4],&[0,0,0,255]);
 }
 #[test]fn unpremultiplies_color_and_rejects_wrong_buffers(){
  let mut black=vec![0;SIDE*SIDE*4];let mut white=vec![255;black.len()];
  black[..4].copy_from_slice(&[0,0,128,0]);white[..4].copy_from_slice(&[127,127,255,0]);
  assert_eq!(&rgba_from_mattes(&black,&white).unwrap()[..4],&[255,0,0,128]);
  assert!(rgba_from_mattes(&[],&[]).is_none());
 }
}
