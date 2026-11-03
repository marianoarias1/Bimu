# Seguridad de Bimu

Bimu es una aplicación de escritorio para Windows. Su revisión de seguridad usa OWASP Top 10:2025 como taxonomía de riesgos, CIS Controls v8.1 como referencia de higiene y NIST SSDF 1.1 / CSF 2.0 para el proceso. Esto no es una certificación de cumplimiento ni una garantía de ausencia de vulnerabilidades.

## Reportar un problema

Contactá a **marianoariasiwngh@outlook.com**. Para un problema que permita acceso no autorizado, exposición de datos o ejecución de código, evitá publicar una prueba explotable en una issue pública antes de coordinar la corrección.

Incluí versión/canal (Microsoft Store o instalador), versión de Windows, impacto, condiciones necesarias y pasos mínimos. No envíes claves, códigos de vinculación vigentes, contenido privado del portapapeles ni archivos personales; usá ejemplos ficticios. No se promete un plazo de respuesta fijo para este proyecto personal.

## Límites de confianza

- Solo los equipos que el usuario vincula deben intercambiar contenido. El descubrimiento de red no autoriza conexiones.
- Una PC vinculada es una fuente de contenido confiada por el usuario, no un antivirus. No abras archivos ejecutables de origen dudoso.
- DPAPI protege las credenciales persistentes para la cuenta actual de Windows. No protege frente a malware ejecutándose con esa misma cuenta.
- El historial en memoria y los archivos descargados son cosas distintas. Los archivos recibidos permanecen en las carpetas de datos de Bimu; vaciar el historial no promete borrado seguro ni eliminación de todas las descargas.
- La distribución Store está firmada por Microsoft. El instalador tradicional no tiene firma Authenticode independiente; su SHA-256 verifica integridad respecto de GitHub, no una identidad independiente de la cuenta de GitHub.

## Proceso y revisión

Ver [proceso de hardening](security/HARDENING.md) y [revisión inicial](security/REVIEW-2026-11-03.md). Se mantiene el funcionamiento existente; toda corrección requiere regresiones y revisión antes de publicar. Las mejoras de seguridad se distribuyen en versiones nuevas: no se garantiza mantenimiento de ramas antiguas.
