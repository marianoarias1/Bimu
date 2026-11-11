use std::path::Path;
pub fn startup_command(exe:&Path)->Result<String,String>{let path=exe.to_str().ok_or("La ruta de Bimu no es válida.")?;if !exe.is_absolute()||path.contains(['"','\n','\r']){return Err("La ruta de Bimu no es válida para el inicio de Windows.".into())}let command=format!("\"{path}\" --autostart");if command.chars().count()>260{return Err("La ruta de instalación es demasiado larga para el inicio de Windows.".into())}Ok(command)}
pub fn initial_startup(configured:Option<bool>,listen_preferred:bool)->Option<bool>{configured.or(if listen_preferred{Some(true)}else{None})}
#[cfg(test)]mod tests{use super::*;#[test]fn explicit_choice_is_preserved(){assert_eq!(initial_startup(None,true),Some(true));assert_eq!(initial_startup(None,false),None);assert_eq!(initial_startup(Some(false),true),Some(false));assert_eq!(initial_startup(Some(true),false),Some(true));}#[test]fn command_quotes_path_and_includes_only_fixed_flag(){let exe=std::env::temp_dir().join("Bimu para Mariano/Bimu.exe");let command=startup_command(&exe).unwrap();assert_eq!(command,format!("\"{}\" --autostart",exe.display()));assert!(startup_command(Path::new("Bimu.exe")).is_err());}}

#[cfg(all(test,windows))]
mod registry_tests {
 use super::*;
 use winreg::{RegKey,enums::{HKEY_CURRENT_USER,REG_SZ}};
 struct TemporaryKey { root:RegKey, name:String }
 impl Drop for TemporaryKey {
  fn drop(&mut self) { let _=self.root.delete_subkey_all(&self.name); }
 }
 #[test]
 fn windows_registry_preserves_the_quoted_startup_command() {
  let unique=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
  let fixture=TemporaryKey {
   root:RegKey::predef(HKEY_CURRENT_USER),
   name:format!("Software\\Bimu\\Tests\\startup-{}-{unique}",std::process::id()),
  };
  let (key,_)=fixture.root.create_subkey(&fixture.name).unwrap();
  let exe=std::env::temp_dir().join("Bimu imágenes de prueba").join("Bimu.exe");
  let command=startup_command(&exe).unwrap();
  key.set_value("Bimu",&command).unwrap();
  assert_eq!(key.get_value::<String,_>("Bimu").unwrap(),command);
  let raw=key.get_raw_value("Bimu").unwrap();
  assert_eq!(raw.vtype,REG_SZ);
  let expected:Vec<u8>=command.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect();
  assert_eq!(raw.bytes.as_ref(),expected.as_slice());
  key.delete_value("Bimu").unwrap();
  assert_eq!(key.get_value::<String,_>("Bimu").unwrap_err().kind(),std::io::ErrorKind::NotFound);
 }
}
