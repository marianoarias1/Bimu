# Bimu website — reference redesign

The page follows the supplied Landing rediseñada-html.zip: dark background, lavender accent, Instrument Serif headings, Inter body text, editorial hero, Windows desktop, feature cards, installation steps and download panel. The requested reference palette/layout takes precedence over generic design defaults.

Only the website and its dedicated CI workflow change. No desktop source, application version, installer, permissions or Rust code changes. The existing Store product link, dynamic latest-exe download, privacy page and project links remain.

## Live demonstrations

demo.html is a same-origin, isolated presentation, driven by demo.js. Its UI uses an unchanged snapshot of the real src/desktop.css in assets/bimu-ui.css, plus unchanged src/dock-geometry.js and src/music-edge.js. These snapshots ensure website changes cannot alter the app and app releases cannot silently alter the site.

Bimu itself is HTML, CSS and SVG, not a screenshot. Real classes and dimensions cover resting notch, music, waveform/timeline, controls, lyric split panel, weather, clipboard with thumbnails and floating pill. The toolbar, happy-eye polygon, pill shape, progress thumb and comet geometry follow the desktop implementation. The simulated enabled pages are music, weather and clipboard.

The hero has a Windows Bloom backdrop and decorative Explorer windows built in HTML. Other views demonstrate resting music, example forecasts, clipboard copy/remove/pause/clear/reset feedback, and a draggable pill with theme/custom-color controls. No audio is loaded or played. Demo copy feedback stays in the demo: it does not read/write the visitor's actual clipboard, access their location, invoke Tauri, clean files or connect devices. The lyric excerpt is limited to seven words.

Demos are suspended outside the viewport and when the document is hidden. A global motion toggle pauses their loops, and reduced-motion starts them paused. Controls have accessible names and work with the keyboard. Parent and iframes accept only same-origin messages from the expected window, and custom colors accept only six-digit hex values.

## Assets

- Windows 11 Bloom, copyright Microsoft; original wallpaper mirrored at https://github.com/viridivn/windows11wallpapers/blob/master/Windows/img19.jpg, resized to 1600 px WebP.
- Michael Jackson, Bad: low-resolution front cover via https://musicbrainz.org/release/c334b20b-fecd-4a82-b4b3-445fb17b675b/cover-art / Cover Art Archive. Used as artwork in the requested demonstration, not as endorsement.
- Instrument Serif and Inter: Google Fonts, SIL Open Font License; font licenses stored beside the locally hosted subsets.
- Existing Bimu icon/banner remain for favicon and social preview; old UI screenshot files are not used in the live page.

## Validation

Node syntax checks cover the website scripts; local link/asset checks validate every HTML page and copied UI sources. docs/tests/browser.mjs checks header eyes, playback/pause/seek/volume, clipboard thumbnail and clear/reset, weather city switching, pill dragging and custom color, image loading, overflow/fit at 1440/1024/768/390/320, and reduced motion. The website workflow runs it in Chromium and uploads screenshots.

The ZIP's generated React/DC runtime is unnecessary: the published page uses browser-native HTML/CSS/JS with no build step and keeps existing GitHub Pages hosting.

Local checks passed: JavaScript syntax, HTML/CSS assets, privacy links, exact UI/geometry snapshot equality, and DOM execution/interactions in all five demo views (including rejection of cross-origin color messages). Local Chromium could not start: its process-singleton socket is denied by the execution environment. Visual screenshots, responsive layout and real pointer/keyboard interaction remain pending in the website Action. No CI polling or waiting is performed; the user will confirm completion before follow-up review.

## Correcciones de fidelidad
Los controles del encabezado se ubican dentro del panel nativo; Pill no incluye flecha de solapa. Los SVG corresponden a Lucide Grip/Settings/ArrowUpToLine de la app. Volumen inicializado y montado únicamente al abrir, con geometría lyrics-volume de 206 px. El arrastre del reproductor usa captura del puntero, suspende el avance y activa is-scrubbing para que onda y bolita avancen juntas. El encabezado web incluye los hombros cóncavos del notch. Pasaron regresiones DOM de las cinco vistas; la comprobación visual queda pendiente en Actions.
