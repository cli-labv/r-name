# 🏷️ r-name

CLI minimalista en **Rust** para **renombrar archivos en masa** de forma segura:
regex, mayúsculas/minúsculas, limpieza de nombres, numeración, cambio de
extensión y orden por fecha EXIF. Siempre muestra la tabla `Actual → Nuevo`,
pide confirmación y guarda un historial para **deshacer**. Cien por ciento
**local**, sin nube.

---

## 🎯 ¿Para qué sirve?
Cuando tienes carpetas con nombres desordenados (`IMG_0001 (2).jpg`, espacios,
letras raras), r-name:
- 🔁 aplica **regex** buscar/reemplazar,
- 🧹 limpia espacios y caracteres raros (queda `A-Za-z0-9._-`),
- 🔢 numera en secuencia (`nombre_0001.ext`),
- 📸 puede prefijar la **fecha EXIF** (o mtime) de las fotos,
- 🛟 **nunca sobrescribe**: si hay colisión, avisa y añade ` (1)`, ` (2)`…,
- ↩️ guarda un log JSON para **revertir** cualquier ejecución.

## ✨ Características
- 🚀 Rápido (escaneo recursivo con `jwalk`).
- 🎛️ Flags combinables: `--replace`, `--lower`, `--upper`, `--trim`, `--number`, `--ext`, `--by-date`.
- 👀 `--dry-run` muestra la tabla sin tocar nada; `--yes` aplica sin preguntar.
- ↩️ `undo` revierte la última ejecución (o una concreta pasando el log).
- 🧹 `clean` borra el historial local (nunca tus archivos).
- 🔒 Todo queda en la carpeta de la app: **nada** sale a tu HOME ni al PATH.
- 🎨 Banner de arranque conciso y spinner de progreso (escaneo/renombrado), sin texto de sobra.

## 🧰 Requisitos
- 🦀 Rust + Cargo instalados (`rustup`).

## 🚀 Primer uso
La **primera vez** compila (tarda un poco). Las siguientes son instantáneas:

```bash
./start.sh      # compila y deja el binario listo en dist/
./start         # ya puedes usarla
```

## 🎮 Uso

### Menú interactivo (al ejecutar sin flags)
```bash
./start                              # abre el menú interactivo (requiere terminal)
```
El menú es directo, con pocas preguntas y orientado al objetivo:

1. **Resumen** → se muestra cuántos archivos hay y cómo se distribuyen por carpeta (t1, t2, …).
2. **Elegir modo**:
   - **Renombrar por temporada (carpetas)** → ideal para series. Detecta automáticamente si hay una carpeta superior que agrupe a todas las temporadas (ej. `re zero/` con `t1/`, `t2/` dentro) y usa su nombre como título por defecto (placeholder dinámico). Pedís el **título de la serie** (una pregunta) y el **orden de renombrado** (nombre asc/desc, fecha creación reciente primero, fecha modificación asc/desc).
     - Temporada 1 → `1 titulo.ext` (solo número de episodio al inicio, luego el título).
     - Temporada 2 en adelante → `t2 1 titulo.ext`, `t3 1 titulo.ext`… (indicador de temporada, número de episodio, luego el título).
     Se conserva la extensión original.
      - Si hay **varias series** (carpetas distintas), cada una se renombra con **su propio nombre de carpeta**, no con un título único.
      - Si no hay subcarpetas de temporada, los archivos se tratan como **temporada 1** y se renombran en su mismo sitio (sin crear carpetas `t1`).
   - **Acciones rápidas (clásico)** → minúsculas, mayúsculas, limpiar nombres, numerar, cambiar extensión, prefijo por fecha.
3. **Preview** → tabla `Actual → Nuevo`.
4. **Confirmar** → aplicar o cancelar. Todo queda registrado para poder deshacer (`undo`).

### Modo directo (flags)
```bash
./start fotos/                       # trabaja sobre la carpeta que elijas
./start --replace "IMG_=Vacaciones"  # regex: buscar=reemplazo
./start --replace "\s+=_"            # espacios → guion bajo (repetible)
./start --lower                      # todo a minúsculas
./start --upper                      # todo a MAYÚSCULAS
./start --trim                       # quita espacios y caracteres raros
./start --number                     # numeración: nombre_0001.ext
./start --ext jpg                    # cambia la extensión
./start --by-date                    # AAAA-MM-DD_nombre.ext (EXIF o mtime)
./start --dry-run                    # solo muestra la tabla, no renombra
./start --yes                        # aplica sin preguntar
./start undo                         # revierte la última ejecución
./start undo json/r-name-undo-…json  # revierte una ejecución concreta
./start clean                        # borra el historial local
```
> 🧩 Los flags se combinan: `./start --lower --trim --number --ext md`. Si pasás cualquier flag, se usa el modo directo (sin menú).

### Modo temporada directo (sin menú, sin preguntar)
```bash
./start --set                  # renombra por temporada usando .env
./start --set .                # sobre la carpeta actual
./start --set fotos/           # sobre otra carpeta
```
Lee el archivo `.env` de la carpeta de la app:
- `R_NAME_TITLE` → título de la serie. Si está vacío, se usa el nombre de la carpeta superior que agrupe a las temporadas (ej. `re zero`).
- `R_NAME_SORT` → orden de episodios: `name`, `name-desc`, `created-desc`, `modified-asc`, `modified-desc` (por defecto `name`).

Aplica directo sin pedir confirmación. Muestra una vista previa informativa (solo nombres de archivo, sin rutas).
> Con varias series se usa el nombre de cada carpeta; sin carpetas de temporada, todo se trata como temporada 1 en el mismo sitio.

## 🗂️ Estructura

## 🗂️ Estructura
```
r-name/
├── input/   ← archivos a renombrar (por defecto)
└── json/    ← historial JSON para undo
```

## 🛠️ Cómo se hizo
- 🦀 **Rust** con `clap` (CLI derive), `regex` (buscar/reemplazar), `jwalk`
  (escaneo recursivo), `kamadak-exif` (fecha EXIF de fotos), `dialoguer` +
  `console` (confirmación y tablas con render propio, sin TUI pesada).
- 🔒 Sin escrituras fuera de la carpeta local (`R_NAME_HOME`, por defecto el
  directorio de la app).

## 📄 Licencia
MIT OR Apache-2.0.
