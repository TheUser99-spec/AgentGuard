# 🎬 Tutorial Kit — Crea tu video sobre Phylax

> Todo lo que necesitas para grabar un tutorial, review o curso sobre Phylax.  
> Scripts, timestamps, comandos, thumbnails y escenarios pre-armados.

---

## 📹 Script de video — 8 minutos (formato YouTube Short/Tutorial)

### Opción A: Tutorial rápido (8 min)

| Timestamp | Duración | Qué decir | Qué mostrar |
|---|---|---|---|
| **0:00** | 0:45 | **Hook:** "¿Sabías que Claude Code puede leer tu `.env` ahora mismo? Sin preguntar. Sin avisar. Y no hay nada que puedas hacer... hasta ahora." | Pantalla negra con texto dramático. Corte a terminal. |
| **0:45** | 0:45 | **El problema:** "Los agentes de IA tienen acceso total a tu sistema de archivos. Pueden leer secretos, borrar migraciones, destrozar configuraciones. Miles de issues en GitHub lo confirman. Las reglas de prompt NO funcionan — el agente las lee y las ignora." | Mostrar issues reales de Claude Code/Cursor. Mostrar CLAUDE.md siendo ignorado. |
| **1:30** | 0:30 | **La solución:** "Phylax. Una capa de seguridad a nivel de sistema operativo. El kernel de Windows devuelve ACCESS_DENIED antes de que el agente toque un solo byte." | Logo de Phylax. Terminal con el dashboard. |
| **2:00** | 1:00 | **Instalación:** "Un comando. 10 segundos. Sin registro, sin nube, sin telemetría." | Ejecutar en vivo: `irm https://raw.githubusercontent.com/TheUser99-spec/Phylax/main/install.ps1 \| iex` |
| **3:00** | 0:30 | **Inicializar:** `phylax init` + `phylax run` | Dashboard abriéndose. Stats en vivo. |
| **3:30** | 1:30 | **Demo 1 — Proteger secretos:** "Vamos a proteger un `.env`. Le decimos a Claude que lo lea..." | Terminal split: Claude a la izquierda, dashboard a la derecha. Claude intenta leer → BLOCKED. Dashboard muestra el bloqueo en rojo. |
| **5:00** | 1:30 | **Demo 2 — Bloquear borrado:** "Ahora protegemos `migrations/`. Le pedimos al agente que borre..." | Agente intenta borrar → BLOCKED. Log de auditoría muestra el intento. |
| **6:30** | 0:45 | **Anti-bypass:** "¿Y si el agente intenta saltarse la protección con `icacls /remove:d`? Mira." | Ejecutar icacls → ACCESS_DENIED. Explicar capa MIC. |
| **7:15** | 0:45 | **Cierre + CTA:** "Phylax es gratis, open source, 100% local. Si quieres dormir tranquilo sabiendo que tus secretos están a salvo de cualquier agente de IA — ya sabes qué hacer." | Mostrar GitHub, estrellas, link a la web. |

### Opción B: Review/Análisis (12 min)

Misma estructura pero con segmentos extendidos de explicación técnica:
- Añadir 2 min extra explicando el modelo de permisos (6 buckets)
- Añadir 2 min extra explicando las 3 capas anti-bypass
- Más tiempo en cada demo para que se vea claro

### Opción C: Curso "Phylax desde cero" (6 capítulos × 20-30 min)

Ver `CURRICULUM.md` para el temario completo de 35 lecciones.

---

## 🎨 Títulos para thumbnail (probados en YouTube)

| Título | Estilo | CTR estimado |
|---|---|---|
| "Claude Code DESTRUYÓ mi proyecto. Esto lo soluciona." | Drama + solución | ⭐⭐⭐⭐⭐ |
| "Por qué NO debes confiar en las reglas de prompt" | Controversia educativa | ⭐⭐⭐⭐ |
| "Protege tus archivos de CUALQUIER agente de IA en 5 min" | Tutorial rápido | ⭐⭐⭐⭐ |
| "La herramienta que todo dev con IA necesita (es gratis)" | Recomendación | ⭐⭐⭐⭐ |
| "Cursor intentó leer mi .env. Esto pasó." | Storytelling | ⭐⭐⭐⭐⭐ |
| "Phylax desde cero — Protege tu código de la IA" | Curso | ⭐⭐⭐ |

---

## 🖼️ Assets visuales

Todos los assets están en `PRESS-KIT.md` y en `landing/public/`:

- **Logo:** `public/logo.jpg` (1200x1200)
- **OG Card:** `public/og-card.png` (1200x630) — ideal para thumbnail base
- **Dashboard screenshot:** Ejecutar `phylax run` y capturar
- **Terminal block screenshot:** Capturar el momento exacto de ACCESS_DENIED
- **Colores oficiales:** `#111316` (ink), `#f7f5ee` (paper), `#e34145` (red/deny), `#17a36c` (green)

---

## 💻 Comandos copy-paste para el video

### Instalación
```powershell
irm https://raw.githubusercontent.com/TheUser99-spec/Phylax/main/install.ps1 | iex
```

### Inicialización
```powershell
phylax init
phylax run
```

### Demo 1: Proteger .env
```toml
# phylax.toml
[deny]
files = [".env", ".env.*", "secrets/**"]
```
Luego ejecutar Claude Code / Cursor y pedirle que lea `.env` → BLOCKED

### Demo 2: Bloquear eliminación
```toml
# phylax.toml
[deny]
files = ["migrations/**", "config/**"]
```
Luego pedir al agente que borre archivos en `migrations/` → BLOCKED

### Demo 3: Anti-bypass
```powershell
# El agente intenta saltar la protección...
icacls .env /remove:d Everyone
# → ACCESS_DENIED (la capa MIC lo bloquea)
```

### Ver auditoría
```powershell
phylax audit list
phylax audit tail
```

---

## 🎙️ Frases clave para mencionar

- "Kernel-level enforcement" (suena técnico y potente)
- "El kernel de Windows devuelve ACCESS_DENIED" (concreto)
- "100% local — sin nube, sin cuentas, sin telemetría" (privacidad)
- "Deny always wins" (simple, memorable)
- "3 capas de protección anti-bypass" (profundidad técnica)
- "No es un linter. No es un sandbox. Es el SO." (diferenciación)
- "Gratis y open source — Apache 2.0" (accesibilidad)

---

## 🔗 Enlaces para la descripción del video

```
🛡️ Phylax — OS-level protection for AI coding agents
🔗 Web: https://phylax.pages.dev
⭐ GitHub: https://github.com/TheUser99-spec/Phylax
🐦 X/Twitter: https://x.com/Phylaxdev
📖 Docs: https://phylax.pages.dev/docs
❓ FAQ: https://phylax.pages.dev/faq

⚡ Instalar en 10 segundos:
irm https://raw.githubusercontent.com/TheUser99-spec/Phylax/main/install.ps1 | iex
```

---

## 📊 Hashtags recomendados

```
#Phylax #AISecurity #ClaudeCode #Cursor #OpenCode #DevTools #WindowsSecurity
#Cybersecurity #VibeCoding #AI #Programming #Rust #OpenSource #DevSecOps
```

---

## ✅ Checklist antes de grabar

- [ ] Tener Windows 10/11 listo
- [ ] Tener Phylax instalado (`irm ... | iex`)
- [ ] Tener un proyecto fake con `.env`, `secrets/`, `migrations/`
- [ ] Tener Claude Code o Cursor instalado para la demo
- [ ] Grabar el dashboard en una ventana separada (OBS: 2 sceneas)
- [ ] Tener el phylax.toml pre-configurado para cada demo
- [ ] Ensayar los timings (el bloqueo es instantáneo, no hay que esperar)
- [ ] Preparar el thumbnail con el logo de Phylax + texto dramático

---

## 🎓 ¿Quieres hacer un curso completo?

El temario "Phylax desde cero" con 35 lecciones está disponible. Cubre desde instalación hasta el kernel driver de Phase 2. Ideal para creadores como midudev que quieren contenido estructurado para su academia.

Ver `CURRICULUM.md` para el temario completo.
