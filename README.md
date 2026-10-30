# Bimu

Bimu es una aplicación de escritorio para Windows que reúne herramientas cotidianas en un notch minimalista, integrado al borde superior de la pantalla. Al pasar el cursor se despliega suavemente; cuando termina la interacción vuelve a su formato compacto.

[Conocer Bimu](https://marianoarias1.github.io/Bimu/) · [Versiones y descargas](https://github.com/marianoarias1/Bimu/releases)

## Funciones

- **Música y letras:** muestra la canción, su artista y portada, permite controlar la reproducción y consultar letras sincronizadas cuando están disponibles. La búsqueda de letras utiliza LRCLIB; los controles multimedia dependen de las sesiones compatibles de Windows.
- **Archivos a mano:** arrastrá archivos o carpetas al notch para guardar accesos y volver a abrirlos. Quitar un elemento de la bandeja no borra el archivo original. Durante una vinculación, la otra PC muestra esos elementos y puede descargar y abrir una copia a pedido (límite inicial: 10 MB por selección). Ocultar un elemento remoto solo lo quita de la vista local.
- **Atajos rápidos:** configurá modos para abrir aplicaciones, carpetas y páginas con un botón.
- **Páginas:** activá, ocultá y ordená las vistas desde Ajustes avanzados. Siempre queda al menos una activa.
- **Conexión entre equipos:** vinculá dos computadoras en la misma red con un código de seis dígitos. Bimu busca equipos disponibles y vuelve a conectar los que ya fueron vinculados. La credencial se guarda protegida para el usuario de Windows; podés desvincularla desde ajustes. La detección puede depender de las reglas del firewall y de la configuración de la red. La conexión actual admite un equipo vinculado por vez.
- **Portapapeles:** historial local opcional para texto, imágenes y rutas de archivos o carpetas copiados desde el Explorador. Se activa desde su página, puede pausarse y vaciarse, y se conserva solo durante la sesión. Respeta las marcas de Windows que excluyen contenido del historial.
- **Clima:** consultá el pronóstico para una ubicación seleccionada o detectada. Usa Open-Meteo y, para la detección aproximada, ipapi.co.
- **Personalización:** elegí tema claro, oscuro o del sistema; color predeterminado o personalizado; transparencia y tamaño en reposo. También podés mover el notch por el borde superior.
- **Información del sistema:** muestra hora, fecha, conexión o batería, y avisa al cambiar el estado de mayúsculas.
- **Modo solapa:** permite reducir el notch manualmente o, en ventanas y controles compatibles, cuando detecta que podría taparlos.

Bimu no incluye actualmente funciones de chat, IA local ni reconocimiento de voz. El indicador musical usa el nivel general de audio de la salida del sistema; no mide frecuencias individuales ni identifica el sonido de una sola aplicación.

## Instalación

En [Releases](https://github.com/marianoarias1/Bimu/releases) está el instalador para Windows x64. Cerrá Bimu desde el ícono de la bandeja antes de actualizar y ejecutá el archivo `Bimu_<versión>_x64-setup.exe`.

La versión pública anterior fue detectada por Microsoft Defender en una computadora. La detección está pendiente de análisis por Microsoft; no se debe asumir que sea un falso positivo. No agregues exclusiones ni restaures archivos en cuarentena para evitar la alerta. El instalador no está firmado digitalmente.

## Privacidad y datos

La configuración, los atajos y las referencias de la bandeja se guardan localmente. Las funciones de letras, clima y actualizaciones consultan servicios de Internet cuando se utilizan. La búsqueda de letras envía nombre de canción y artista a LRCLIB; el clima envía la ubicación o consulta una ubicación aproximada por IP si se solicita detección. El indicador musical consulta un nivel agregado de la salida de audio, sin grabar el micrófono.

## Desarrollo en Windows

Requiere Node.js, Rust con toolchain MSVC, herramientas de compilación de C++ de Visual Studio y WebView2. Consultá los [requisitos de Tauri](https://v2.tauri.app/start/prerequisites/).

```powershell
npm ci
npm run desktop:dev
```

Para comprobar y construir:

```powershell
npm test
cargo test --manifest-path native-tests/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run desktop:build -- --bundles nsis
```

Las pruebas automatizadas no sustituyen la verificación manual en Windows de animaciones, audio, arrastre, ventanas y distintas escalas de pantalla.

## Actualizaciones

El flujo de GitHub Actions construye el instalador para versiones nuevas. Bimu verifica el tamaño y el SHA-256 proporcionado por GitHub antes de abrir una actualización; esta comprobación no equivale a una firma digital independiente.

### Publicación de la web

La página de descarga se publica desde `docs/` mediante GitHub Pages. Los cambios en esa carpeta se validan con el workflow `Website` y no requieren incrementar la versión de Bimu ni compilar instaladores. Las PR que solo modifican la web, README, ROADMAP o los workflows de web/verificación/release omiten la compilación de Windows. Los cambios de la aplicación conservan los controles de versión, pruebas y compilación habituales.

Microsoft Store es la descarga recomendada: https://apps.microsoft.com/detail/9NZRVTXK49XS. El instalador tradicional de GitHub sigue disponible como alternativa sin firma digital. Publicar cambios en la web no envía actualizaciones a Microsoft Store.
