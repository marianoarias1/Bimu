//! Playback selection is independent of which app Windows last focused.
#[derive(Clone,Copy)]
pub struct Candidate {pub playing:bool,pub current:bool,pub previous:bool}
pub fn ranked(items:&[Candidate])->Vec<usize>{
 let mut order:Vec<_>=(0..items.len()).collect();
 order.sort_by_key(|&i|(!items[i].playing,!items[i].current,!items[i].previous,i));order
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn playing_spotify_wins_over_a_paused_or_empty_current_browser(){let items=[Candidate{playing:false,current:true,previous:false},Candidate{playing:true,current:false,previous:true}];assert_eq!(ranked(&items),vec![1,0]);}
 #[test]fn current_playing_session_wins_and_previous_playback_breaks_fallback_ties(){let items=[Candidate{playing:true,current:false,previous:false},Candidate{playing:true,current:false,previous:true},Candidate{playing:true,current:true,previous:false}];assert_eq!(ranked(&items),vec![2,1,0]);assert_eq!(ranked(&[]),Vec::<usize>::new());}
}
