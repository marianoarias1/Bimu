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

### Bordes de pantalla y transferencias locales (0.55.2)

En Ajustes avanzados → General se puede elegir el borde superior, inferior, izquierdo o derecho. El botón de movimiento desplaza Bimu sobre el borde seleccionado y cada borde recuerda su posición. En los laterales, el reloj en reposo muestra horas y minutos en dos líneas con dos puntos; al desplegarse, todas las páginas mantienen su orientación horizontal. Las letras compactas laterales se muestran en varias líneas; el reproductor desplegado conserva la vista de letras sincronizadas. La posición respeta el área de trabajo de Windows para evitar la barra de tareas.

En Ajustes avanzados → Dispositivos, el límite de archivos es de 30 MB por defecto y se puede configurar entre 1 y 250 MB por selección (incluyendo el total de una carpeta), usando unidades de 1.048.576 bytes. La PC que generó el código sigue siendo la principal, también al reconectar. Si guardó explícitamente un límite, Portapapeles de archivos y Archivos a mano usan ese límite en ambos sentidos durante la conexión. Si la principal está en el valor por defecto sin personalizar, cada PC aplica su propio límite al enviar. La recepción valida ese límite autenticado del origen. Ninguna PC sobrescribe la configuración guardada de la otra: desconectar restaura su política local. «Restablecer por defecto» borra la personalización, vuelve a 30 MB y libera la anulación de la principal. El ajuste persiste después de reiniciar y los cambios se comunican por el canal autenticado. Las imágenes copiadas como imagen conservan su límite independiente de resolución/memoria, y los textos su límite de 1 MB.

El límite no reserva RAM ni precarga archivos. La preparación ocurre únicamente con conexión activa y se cancela al desconectarse o al reemplazar la selección. El formato de transferencia actual conserva temporalmente el contenido y su codificación Base64 en memoria, por lo que durante una transferencia la memoria puede superar el tamaño del archivo; al terminar o cancelar se liberan esos buffers. No se promete un consumo máximo de RAM igual al límite de archivos. El historial local es independiente de la sincronización.

Esta versión usa el protocolo local v5 para negociar límites: actualizar Bimu en **ambas PCs**. Las vinculaciones guardadas se conservan. Una PC con el protocolo anterior no se conecta hasta actualizarla.

Validación manual antes de publicar: probar cada borde y tamaño de reposo, reproductor con/sin letras y frases largas, las cinco páginas, ajustes rápidos y todas las secciones avanzadas, recepción por drag & drop, solapa manual/automática y movimiento cerca de esquinas. Repetir con escala de Windows 100/125/150 %, tema claro y color personalizado. Entre dos PCs, probar 29/30/31 MB, carpetas con varios archivos, límites diferentes, cambio de límite conectado, desconexión durante preparación y persistencia al reiniciar. Medir RAM en reposo conectado/desconectado y durante/después de transferir.

El indicador de reproducción es un trazo luminoso que recorre el contorno libre de Bimu, incluidas sus esquinas, sin pasar por el borde pegado al monitor ni deformar la silueta. La cabeza delantera es la parte más gruesa y brillante; la cola pierde grosor y opacidad progresivamente. Se adapta a los cuatro bordes, al color del tema/personalizado y a la transparencia. Al pausar se desvanece. Animación a ~30 fps sin renders React por cuadro, sin redimensionar la ventana y sin activar el medidor de audio. Ocultar la ventana suspende la animación; reducir movimiento usa una luz estática.

Pruebas de esta revisión: ondas algo más altas y brillo moderadamente mayor; reposo lateral de 40 × 126 px en tamaño medio, reloj de 13 px y un único «•». Transferencias de hasta 250 MB por selección; probar principal con 200 MB y secundaria con 10 MB en ambos sentidos, reconectar y comprobar que la secundaria sigue conservando 10 MB. Restablecer la principal: envía hasta 30 MB y secundaria hasta 10 MB; desconectar y revisar que ambas conservan sus preferencias. El formato Base64 puede elevar el pico temporal de RAM al transferir archivos grandes; 250 MB es un tope, no RAM reservada.

Revisión de Archivos a mano (0.55.2): los cambios del catálogo se anuncian inmediatamente y antes que el portapapeles, con un pequeño reanuncio cada 3 segundos conectado. La página visible reconcilia su lista cada 2 segundos; no carga archivos ni hace polling con esa página oculta. Se suscribe a eventos antes de leer la lista inicial. Quitar y volver a agregar una carpeta crea una publicación nueva para no heredar una ocultación remota anterior. La lista remota nunca depende del tamaño de su contenido: solo abrirla descarga los archivos. Regresión: catálogo, solicitud y descarga de carpeta de 70 MB (con subcarpeta), principal 250 MB/secundaria 30 MB y comprobación de integridad. Sigue pendiente verificar el caso reportado en las dos PCs reales. Brillo de ondas apenas mayor, sin cambiar amplitud ni animación.

Revisión visual 0.55.3: trazo viajero aprobado (silueta fija). Verificar los cuatro bordes, reposo/hover/letras, temas claros/oscuros/personalizados, transparencia, pausas y retorno desde ventana oculta. La cabeza debe avanzar por delante de toda la cola, también en las curvas; al terminar se apaga por completo antes de iniciar el siguiente recorrido. La geometría y las preferencias existentes se conservan. Cuatro pruebas de contorno, taper y recorrido reemplazan las dos de las ondas anteriores.
