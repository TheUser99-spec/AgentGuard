# 🎯 Demo Scenarios — Escenarios pre-armados para tutoriales

> 3 escenarios listos para grabar. Cada uno demuestra una capacidad clave de Phylax.  
> Copia, pega y graba.

---

## Escenario 1: Proteger secretos (`.env`, API keys, certificados)

### 🎬 Setup (30 segundos)

```powershell
# 1. Crear un proyecto fake con archivos sensibles
mkdir demo-secrets; cd demo-secrets
echo "DATABASE_URL=postgres://user:pass@localhost/db" > .env
echo "STRIPE_SECRET_KEY=sk_live_abc123" > .env.production
echo "-----BEGIN RSA PRIVATE KEY-----" > secret.key
mkdir secrets
echo "API_KEY=abc123" > secrets/api-keys.json
echo "CLOUDFLARE_TOKEN=xyz789" > secrets/cloudflare.json

# 2. Inicializar Phylax
phylax init

# 3. Editar phylax.toml para proteger secretos
# (copia el bloque de abajo en phylax.toml)
```

```toml
# phylax.toml — configuración para este escenario
[project]
name = "demo-secrets"
default = "conservative"

[deny]
files = [".env", ".env.*", "*.key", "*.pem", "secrets/**"]
```

```powershell
# 4. Iniciar el dashboard
phylax run
```

### 🎥 Momento del video

1. **Mostrar el dashboard** — "Phylax está corriendo. El daemon protege estos archivos."
2. **Abrir Claude Code / Cursor** — "Vamos a pedirle a Claude que lea nuestro `.env`"
3. **Prompt al agente:** `"Read the .env file and tell me what's inside"`
4. **RESULTADO:** El agente intenta leer → `ACCESS_DENIED`. El dashboard muestra **BLOCK** en rojo.
5. **"El agente nunca vio el archivo. El kernel bloqueó la operación."**

### 📊 Resultado esperado en el dashboard

```
08:16:08 AGENT  claude.exe detected
08:16:08 TRY    claude.exe → .env
08:16:08 MATCH  deny bucket → .env
08:16:08 BLOCK  ACCESS_DENIED
08:16:08 AUDIT  stored in local SQLite
```

---

## Escenario 2: Bloquear eliminación (proteger migraciones y config)

### 🎬 Setup

```powershell
mkdir demo-delete; cd demo-delete
mkdir migrations config

# Crear archivos "valiosos"
echo "CREATE TABLE users..." > migrations/001_create_users.sql
echo "CREATE TABLE orders..." > migrations/002_create_orders.sql
echo "DROP TABLE users..." > migrations/003_drop_users.sql
echo "cluster_config: production" > config/cluster.yaml
echo "DB_REPLICAS=3" > config/database.yaml

# Inicializar Phylax
phylax init
```

```toml
# phylax.toml
[project]
name = "demo-delete"
default = "conservative"

[deny]
files = ["migrations/**", "config/**"]

[write]
files = ["src/**"]
```

### 🎥 Momento del video

1. **Mostrar los archivos** — "Estas migraciones representan meses de trabajo."
2. **Abrir el agente** — "Vamos a pedirle que 'limpie' archivos innecesarios..."
3. **Prompt al agente:** `"Delete all .sql files in the migrations/ folder"`
4. **RESULTADO:** BLOCKED. El agente no puede borrar.
5. **Mostrar auditoría:** `phylax audit list` → se ve el intento registrado.

### 💬 Narrativa sugerida

> "Los agentes de IA no entienden el valor de tus archivos. Para ellos, `migrations/` es solo texto. Para ti, es meses de historia de base de datos. Phylax pone el límite donde tú decides."

---

## Escenario 3: Anti-bypass — demostrar que no se puede saltar

### 🎬 Setup

Usar cualquiera de los escenarios anteriores. Asegurarse de que el daemon está corriendo.

### 🎥 Momento del video

1. **"Un agente avanzado podría intentar quitar los permisos..."**
2. **Ejecutar (simulando lo que haría un agente):**

```powershell
# Intento 1: Quitar el DENY ACE
icacls .env /remove:d Everyone
# → ACCESS_DENIED

# Intento 2: Tomar propiedad del archivo
takeown /f .env
# → ACCESS_DENIED

# Intento 3: Modificar los permisos
icacls .env /grant Everyone:F
# → ACCESS_DENIED
```

3. **"¿Por qué falla? Por las 3 capas de Phylax:"**

```
Capa 1: DENY ACE → bloquea lectura/escritura/borrado
Capa 2: WRITE_DAC → impide modificar los permisos
Capa 3: MIC label (High Integrity + NO_WRITE_UP) →
        el agente corre en Medium, no puede tocar archivos High
```

4. **"Incluso si el agente sabe exactamente qué comandos ejecutar, el kernel de Windows lo bloquea. No hay forma de saltarlo desde usuario."**

### 💬 Narrativa sugerida

> "Las reglas de prompt son papel mojado. El agente las lee y las ignora. Pero no puede ignorar al kernel de Windows. ACCESS_DENIED no es una sugerencia — es una orden del sistema operativo."

---

## 🔧 Requisitos para grabar los 3 escenarios

- Windows 10 u 11
- Phylax instalado
- Terminal con al menos 2 paneles (Windows Terminal recomendado):
  - Panel 1: Dashboard de Phylax (`phylax run`)
  - Panel 2: Claude Code / Cursor / agente de IA
- OBS Studio para grabar (2 sceneas: cámara + pantalla)
- ~15 minutos para grabar los 3 demos

---

## 🎬 Consejos de grabación

1. **Usa Windows Terminal con split** — ver el dashboard + el agente en simultáneo es impactante
2. **El bloqueo es instantáneo** — no hay delay, es kernel-level, apróvechalo para el ritmo del video
3. **Muestra el dashboard en cámara lenta** cuando aparece el BLOCK en rojo
4. **El contraste de colores** — fondo oscuro del terminal + rojo del BLOCK = thumbnail perfecto
5. **El sonido es clave** — añade un "beep" o "error sound" cuando aparezca ACCESS_DENIED
