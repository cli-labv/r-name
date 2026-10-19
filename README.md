# 🏷️ renamr

CLI minimalista en **Rust** para **renombrar archivos en masa** de forma segura:
regex, mayúsculas/minúsculas, limpieza de nombres, numeración, cambio de
extensión y orden por fecha EXIF. Siempre muestra la tabla `Actual → Nuevo`,
pide confirmación y guarda un historial para **deshacer**. Cien por ciento
**local**, sin nube.

---

## 🎯 ¿Para qué sirve?
Cuando tienes carpetas con nombres desordenados (`IMG_0001 (2).jpg`, espacios,
letras raras), renamr:
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

## 🧰 Requisitos
- 🦀 Rust + Cargo instalados (`rustup`).

## 🚀 Primer uso
La **primera vez** compila (tarda un poco). Las siguientes son instantáneas:

```bash
./start.sh      # compila y deja el binario listo en dist/
./start         # ya puedes usarla
```

## 🎮 Uso
```bash
./start                              # aplica reglas a ./input
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
./start undo json/renamr-undo-…json  # revierte una ejecución concreta
./start clean                        # borra el historial local
```
> 🧩 Los flags se combinan: `./start --lower --trim --number --ext md`

## 🗂️ Estructura
```
renamr/
├── input/   ← archivos a renombrar (por defecto)
└── json/    ← historial JSON para undo
```

## 🛠️ Cómo se hizo
- 🦀 **Rust** con `clap` (CLI derive), `regex` (buscar/reemplazar), `jwalk`
  (escaneo recursivo), `kamadak-exif` (fecha EXIF de fotos), `dialoguer` +
  `console` (confirmación y tablas con render propio, sin TUI pesada).
- 🔒 Sin escrituras fuera de la carpeta local (`RENAMR_HOME`, por defecto el
  directorio de la app).

## 📄 Licencia
MIT OR Apache-2.0.
