use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().https_only(true).connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(12)).user_agent("Bimu/0.34.0 (https://github.com/marianoarias1/Bimu)").build().map_err(|_| "No se pudo iniciar la consulta del clima.".into())
}
async fn get(url: &str, params: &[(&str, String)]) -> Result<Value, String> {
    let response = client()?.get(url).query(params).send().await.map_err(|_| "No se pudo conectar. Revisá tu conexión e intentá de nuevo.")?;
    if !response.status().is_success() { return Err("El servicio del clima no está disponible en este momento.".into()); }
    crate::http_response::json_limited(response,1_000_000).await
}
#[derive(Deserialize)]
struct IpLocation { city: Option<String>, region: Option<String>, country_name: Option<String>, latitude: Option<f64>, longitude: Option<f64> }
#[tauri::command]
pub async fn weather_detect_location() -> Result<Value, String> {
    let value = get("https://ipapi.co/json/", &[]).await?;
    let loc: IpLocation = serde_json::from_value(value).map_err(|_| "No pudimos detectar tu ubicación. Buscá tu ciudad manualmente.")?;
    let (lat, lon) = (loc.latitude.ok_or("No pudimos detectar tu ubicación. Buscá tu ciudad manualmente.")?, loc.longitude.ok_or("No pudimos detectar tu ubicación. Buscá tu ciudad manualmente.")?);
    let name = loc.city.filter(|v| !v.is_empty()).ok_or("No pudimos detectar tu ciudad. Buscala manualmente.")?;
    Ok(serde_json::json!({"name":name,"admin1":loc.region,"country":loc.country_name,"latitude":lat,"longitude":lon}))
}
#[tauri::command]
pub async fn weather_search_city(query: String) -> Result<Value, String> {
    let query = query.trim();
    if query.chars().count() < 2 || query.len() > 200 { return Err("Ingresá al menos dos caracteres.".into()); }
    let value = get("https://geocoding-api.open-meteo.com/v1/search", &[("name",query.into()),("count","8".into()),("language","es".into()),("format","json".into())]).await?;
    Ok(value.get("results").cloned().unwrap_or_else(|| serde_json::json!([])))
}
#[tauri::command]
pub async fn weather_forecast(latitude: f64, longitude: f64) -> Result<Value, String> {
    if !latitude.is_finite() || !longitude.is_finite() || !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) { return Err("La ubicación no es válida.".into()); }
    get("https://api.open-meteo.com/v1/forecast", &[("latitude",latitude.to_string()),("longitude",longitude.to_string()),("current","temperature_2m,apparent_temperature,weather_code,is_day".into()),("daily","temperature_2m_max,temperature_2m_min".into()),("hourly","temperature_2m,precipitation_probability,weather_code".into()),("forecast_days","2".into()),("timezone","auto".into())]).await
}
