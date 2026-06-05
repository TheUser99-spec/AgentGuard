# 📚 Phylax desde cero — Currículum completo

> Curso estructurado en 6 capítulos · 35 lecciones · ~2 horas total  
> Ideal para academias como midu.dev, plataformas de cursos, o YouTube series.

---

## 📦 Capítulo 01 — INTRODUCCIÓN (~20 min)

| # | Lección | Duración | Formato |
|---|---|---|---|
| 01 | **El problema: los agentes de IA destruyen datos** | 4m | Teoría + casos reales |
| 02 | **Qué es Phylax y cómo funciona** | 4m | Teoría + demo rápida |
| 03 | **Instalación en 1 comando** | 3m | Tutorial práctico |
| 04 | **Ventajas vs reglas de prompt** | 4m | Comparativa |
| 05 | 🧪 Examen práctico | — | Quiz + ejercicio |

### Objetivos del capítulo
- Entender el riesgo real de los agentes de IA con acceso al filesystem
- Instalar Phylax correctamente
- Diferenciar seguridad real (OS-level) de reglas de texto (prompt rules)

---

## 📦 Capítulo 02 — PRIMEROS PASOS (~25 min)

| # | Lección | Duración | Formato |
|---|---|---|---|
| 06 | **Tu primer phylax.toml** | 3m | Tutorial práctico |
| 07 | **Entendiendo el daemon** | 4m | Teoría |
| 08 | **El dashboard en vivo (60fps)** | 3m | Demo |
| 09 | **Los 6 buckets de permisos** | 5m | Teoría |
| 10 | **Primer bloqueo real** | 5m | Demo en vivo |
| 11 | **Conservador vs unrestricted** | 3m | Comparativa |
| 12 | 🧪 Examen práctico | — | Quiz + ejercicio |

### Objetivos del capítulo
- Configurar un proyecto con phylax.toml
- Entender los 6 niveles de permisos
- Ver el primer bloqueo en tiempo real

---

## 📦 Capítulo 03 — PROTEGIENDO PROYECTOS (~30 min)

| # | Lección | Duración | Formato |
|---|---|---|---|
| 13 | **Proteger .env, secretos y claves** | 5m | Tutorial práctico |
| 14 | **Bloquear eliminaciones con glob patterns** | 4m | Tutorial práctico |
| 15 | **Probando con Claude Code** | 5m | Demo con agente real |
| 16 | **Probando con Cursor** | 4m | Demo con agente real |
| 17 | **Probando con Windsurf y Copilot** | 5m | Demo con agente real |
| 18 | **Verificando bloqueos en el dashboard** | 4m | Demo |
| 19 | **Auditoría: `phylax audit list` y `tail`** | 3m | Tutorial práctico |
| 20 | 🧪 Examen práctico | — | Quiz + ejercicio |

### Objetivos del capítulo
- Proteger archivos sensibles con diferentes patrones
- Verificar que la protección funciona con múltiples agentes
- Usar las herramientas de auditoría

---

## 📦 Capítulo 04 — DETECCIÓN AVANZADA (~25 min)

| # | Lección | Duración | Formato |
|---|---|---|---|
| 21 | **Cómo Phylax detecta agentes** | 5m | Teoría |
| 22 | **Las 5 señales de detección** | 5m | Teoría profunda |
| 23 | **Reglas globales vs por proyecto** | 4m | Tutorial práctico |
| 24 | **Personalizando phylax.toml avanzado** | 4m | Tutorial práctico |
| 25 | **Comandos avanzados de CLI** | 4m | Tutorial práctico |
| 26 | **Integración con CI/CD** | 3m | Tutorial práctico |
| 27 | 🧪 Examen práctico | — | Quiz + ejercicio |

### Objetivos del capítulo
- Entender el sistema de detección multi-señal
- Usar reglas globales para proteger todos los proyectos
- Dominar los comandos avanzados de la CLI

---

## 📦 Capítulo 05 — SEGURIDAD TOTAL (~25 min)

| # | Lección | Duración | Formato |
|---|---|---|---|
| 28 | **Las 3 capas anti-bypass** | 5m | Teoría |
| 29 | **DENY ACEs explicados** | 4m | Teoría profunda |
| 30 | **MIC labels y NO_WRITE_UP** | 5m | Teoría profunda |
| 31 | **Demostración: intentar saltar la seguridad** | 5m | Demo en vivo |
| 32 | **Entendiendo la arquitectura interna** | 4m | Teoría |
| 33 | **Buenas prácticas de seguridad** | 3m | Guía |
| 34 | 🧪 Examen práctico | — | Quiz + ejercicio |

### Objetivos del capítulo
- Entender por qué el anti-bypass es efectivo
- Ver en vivo cómo fallan los intentos de bypass
- Aplicar buenas prácticas de seguridad con agentes de IA

---

## 📦 Capítulo 06 — FUTURO Y COMUNIDAD (~15 min)

| # | Lección | Duración | Formato |
|---|---|---|---|
| 35 | **Phase 2: Kernel minifilter driver** | 4m | Teoría + roadmap |
| 36 | **Roadmap: macOS, Linux, distributed trust** | 3m | Roadmap |
| 37 | **Cómo contribuir al proyecto** | 4m | Guía |
| 38 | **Comunidad: Discord, GitHub, X** | 3m | Comunidad |
| 39 | 🧪 Examen final | — | Evaluación completa |

### Objetivos del capítulo
- Entender hacia dónde va el proyecto
- Saber cómo contribuir
- Unirse a la comunidad

---

## 🎓 Información del curso

| Campo | Valor |
|---|---|
| **Título** | Phylax desde cero |
| **Duración total** | ~2 horas |
| **Lecciones** | 39 (35 clases + 4 exámenes) |
| **Nivel** | Principiante a intermedio |
| **Prerrequisitos** | Windows 10/11, terminal básica |
| **Stack** | Phylax • Windows Security • Rust • Claude Code • Cursor • Terminal |

---

## 🏆 Certificación

Al completar todos los capítulos y exámenes, el estudiante recibe el certificado:

> **"Phylax Certified — AI Agent Security Specialist"**

Verificable y compartible en LinkedIn.

---

## 📝 Notas para el instructor

- Cada capítulo es independiente — los estudiantes pueden saltar al que necesiten
- Las demos requieren Windows (Phase 1 es Windows-only)
- Tener los escenarios de `DEMO-SCENARIOS.md` pre-configurados
- Recomendar a los estudiantes tener un proyecto real para practicar
- Los exámenes son opcionales pero recomendados para la certificación
