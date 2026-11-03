# Proceso de revisión y hardening

## Alcance

Código de Bimu, interfaz WebView/IPC, configuración local, vinculación LAN, intercambio de archivos/portapapeles, cliente HTTP, actualizaciones y workflows de GitHub. No modifica las políticas de Windows del usuario, el firewall, la cuenta Microsoft ni las reglas administrativas de GitHub. CIS Controls no equivale a aplicar un CIS Windows Benchmark al equipo personal.

Referencias:

- [OWASP Top 10:2025](https://top10.owasp.org/2025/).
- [CIS Controls v8.1](https://www.cisecurity.org/controls/v8-1).
- [NIST SP 800-218 SSDF 1.1, final](https://csrc.nist.gov/pubs/sp/800/218/final). La revisión SSDF 1.2 es un borrador, no se presenta aquí como estándar final.
- [NIST CSF 2.0](https://www.nist.gov/cyberframework).

## Ciclo obligatorio por cambio

1. Identificar datos afectados y frontera de confianza. Registrar atacante, precondición, impacto y evidencia; separar vulnerabilidad comprobada, defensa adicional y deuda.
2. Priorizar exposición/impacto/reproducibilidad. No asignar RCE, CVSS o criticidad alta sin evidencia. Registrar controles no aplicables y pruebas no realizadas.
3. Trabajar en PR; conservar versión anterior para comparar y no mezclar funcionalidades nuevas. No cambiar protocolo, identificadores, rutas de configuración o límites de recursos sin justificar compatibilidad.
4. Corregir de forma acotada. Agregar una regresión que falle con el comportamiento vulnerable, incluyendo entradas malformadas, timeout, cancelación y límites exactos. No eliminar un control para pasar una prueba.
5. Ejecutar `npm test`, `npm run test:security`, `npm run build`, pruebas de ambos crates Rust y `npm run test:browser` en Windows/Edge. La escena de navegador no sustituye la app instalada.
6. Revisar `Security baseline`: npm, RustSec y política de CSP/IPC/actions. Conservar JSON e inventario como evidencia. Un fallo de red o del escáner no significa ausencia de vulnerabilidades. Las advertencias RustSec también se revisan; no se silencian en bloque. Toda excepción debe tener dueño, motivo, alcance, mitigación y fecha de revisión.
7. Probar instalador y MSIX: actualización conserva datos; Store no ofrece EXE; vinculación nueva/reconexión/desvinculación; copiar texto/imagen/archivos y carpeta de 70 MB con principal 250/secundaria 30; límites, cancelación y errores. Hover con/sin letras y cuatro bordes, tema claro/oscuro/color/opacidad, arrastre, páginas y atajos.
8. Comparar arranque, RAM/CPU en reposo desconectado/conectado y una transferencia antes/después bajo las mismas condiciones. No inferir una mejora de rendimiento a partir de pruebas funcionales. No agregar registros con claves, códigos, contenido del portapapeles, cuerpos de archivos o URLs privadas.
9. Revisión humana y checks requeridos en verde. Publicar GitHub y Store según su flujo separado, con versión nueva, hashes y resultados. Ante regresión, detener el envío y preparar una corrección de versión superior; Store no se resuelve publicando un número menor.
10. Actualizar hallazgos y repetir tras cambios en dependencias, permisos, red, archivos o pipeline; revisar periódicamente aunque no haya features nuevas.

## Automatización

- `Security baseline`: por PR, push y semanal; permisos de lectura, acciones fijadas por SHA, npm audit, cargo-audit 0.22.2 en ambos lockfiles y reportes. Npm bloquea high/critical; RustSec bloquea vulnerabilidades, unsound y yanked. Avisos no bloqueantes son deuda a evaluar.
- Dependabot propone cambios de npm, Cargo y GitHub Actions. No se habilita automerge ni se ejecuta `audit fix --force`.
- `scripts/security-policy.mjs`: controla CSP, permisos genéricos, referencias de acciones y algunos patrones de secretos sin imprimir valores. Es un guard sencillo: no sustituye un escáner completo de secretos/SAST/pentest y omite binarios/textos de más de 2 MB.
- El propietario debe hacer obligatorios los checks en la regla de `main`; añadir un workflow no lo hace obligatorio por sí solo.

## Correspondencia práctica

| Referencia | Revisión / evidencia de Bimu |
|---|---|
| OWASP A01 Broken Access Control | IDs nativos, catálogo compartido, ninguna ruta suministrada por el peer, validación antes de escribir |
| A02 Security Misconfiguration | CSP, capabilities locales mínimas, privilegios CI y perfiles de red pendientes |
| A03 Software Supply Chain Failures | lockfiles, auditorías npm/RustSec, Dependabot, acciones por SHA, distribución |
| A04 Cryptographic Failures | PAKE, confirmación HMAC, Noise, aleatoriedad del SO, DPAPI; no certificado criptográfico |
| A05 Injection | argumentos de procesos sin shell para entradas del usuario, URLs web, perfiles y nombres/rutas recibidos |
| A06 Insecure Design | consentimiento/vinculación, peer confiado, retención de descargas, modelo de amenazas |
| A07 Authentication Failures | código único de 120 s/un intento, clave persistente, plazo absoluto de handshake |
| A08 Integrity Failures | SHA-256 y origen del instalador; escrituras exclusivas/atómicas; firma independiente pendiente |
| A09 Logging and Alerting Failures | reportes de CI y procedimiento de respuesta; telemetría local de seguridad pendiente sin datos privados |
| A10 Exceptional Conditions | lectura limitada, errores cerrados, timeouts, cancelación y archivos parciales |
| CIS 2, 7, 16 | inventario, vulnerabilidades y desarrollo seguro |
| CIS 3, 4, 6 | datos, configuración y acceso; sujeto a límites de Windows y cuenta del usuario |
| CIS 8, 17 | evidencia mínima y respuesta; registro operativo local es deuda |
| NIST SSDF PO / PS / PW / RV | requisitos y herramientas / protección e integridad / diseño y pruebas / respuesta y prevención de recurrencias |
| NIST CSF Govern / Identify / Protect / Detect / Respond / Recover | dueño y excepciones / inventario y amenazas / controles / CI y pruebas / coordinación / actualización y recuperación |

La tabla es trazabilidad de revisión, no un resultado de conformidad. Autorización SQL, roles SaaS, Active Directory o configuración MDM no se inventan donde Bimu no los usa.
