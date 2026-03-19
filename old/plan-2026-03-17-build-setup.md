# Plan para compilar Mina Rust localmente

Fuente base: https://o1-labs.github.io/mina-rust/docs/developers/getting-started/

Contexto local detectado:
- Este directorio (`/home/mtrapaglia/mina`) está vacío.
- El plan asume que vas a clonar `mina-rust` aquí o en un subdirectorio.

## Log de progreso

- 2026-03-17: se leyó la guía oficial de Mina Rust para desarrolladores.
- 2026-03-17: se detectó que `/home/mtrapaglia/mina` está vacío, así que no conviene clonar el repo en `.` porque ya existe este `plan.md`.
- 2026-03-17: se verificó el host local:
  - Debian 13 (`trixie`)
  - `aarch64`
  - `nix` disponible en `/nix/var/nix/profiles/default/bin/nix`
  - `docker` disponible en `/usr/bin/docker`
  - `rustup` disponible en `/home/mtrapaglia/.cargo/bin/rustup`
- 2026-03-17: se decidió priorizar un entorno aislado con `nix develop`, porque la guía oficial lo soporta y reduce el riesgo de conflictos de versiones con el sistema base.
- 2026-03-17: se clonó el repositorio oficial en `/home/mtrapaglia/mina/mina-rust` (`commit ab69eaed8`).
- 2026-03-17: se verificó que el repo incluye `flake.nix`, `rust-toolchain.toml`, `Makefile` y `CLAUDE.md`.
- 2026-03-17: se detectó una discrepancia entre la página y el repo actual:
  - la página consultada todavía habla de `Rust 1.84`
  - el repo fijó `Rust 1.92` en `rust-toolchain.toml`
  - a partir de este punto se seguirá la versión fijada por el repo
- 2026-03-17: se entró correctamente al entorno aislado con `nix develop`.
- 2026-03-17: dentro del entorno aislado se confirmó:
  - `rustc 1.92.0`
  - `cargo 1.92.0`
  - `GNU Make 4.4.1`
- 2026-03-17: se revisó `make help` dentro del entorno y quedaron confirmados los targets principales (`build`, `build-release`, `check`, `test`, `download-circuits`, `run-node`).
- 2026-03-17: capacidad detectada de la máquina para planificar el build:
  - `4` CPUs lógicas
  - `15 GiB` de RAM total
  - `39 GiB` libres en disco
- 2026-03-17: decisión operativa:
  - compilar con prioridad reducida (`nice`)
  - limitar el paralelismo de Cargo para no degradar tanto la máquina durante el primer build
- 2026-03-17: se inició la compilación del binario principal `mina-cli` en entorno aislado con:
  - `nice -n 10`
  - `CARGO_BUILD_JOBS=2`
  - log completo redirigido a `/home/mtrapaglia/mina/build-mina-cli.log`
- 2026-03-17: el build superó la fase de descarga inicial de dependencias y entró en compilación de crates sin errores tempranos de toolchain o librerías del sistema.
- 2026-03-17: el primer intento de build falló en `crates/p2p/build.rs` porque `prost-build` no encontró `protoc`.
- 2026-03-17: diagnóstico:
  - `protoc` sí está disponible dentro de `nix develop`
  - el problema fue haber lanzado el build con `bash -lc` dentro de la shell de Nix, lo que alteró el `PATH`
  - corrección elegida: relanzar con `bash -c` para preservar el entorno del `flake.nix`
- 2026-03-17: se relanzó la compilación corregida de `mina-cli` reutilizando los artefactos ya compilados.
- 2026-03-17: validación final del build:
  - `cargo build -p mina-cli --bin mina` terminó con `EXIT:0`
  - binario generado en `/home/mtrapaglia/mina/mina-rust/target/debug/mina`
  - tamaño del binario debug: `984M`
  - `mina --help` respondió correctamente dentro de `nix develop`

## Objetivo

Dejar un entorno listo para compilar Mina Rust desde código fuente, validar que el build funciona y tener a mano los siguientes pasos de desarrollo.

## Decisión de aislamiento

Para esta máquina, la estrategia recomendada es:

1. Mantener este directorio como carpeta contenedora
2. Clonar el repo en `/home/mtrapaglia/mina/mina-rust`
3. Usar `nix develop` dentro del repo para evitar instalar dependencias de compilación con `apt`

Motivo:

- evita ensuciar el sistema con versiones específicas de toolchains y librerías del proyecto
- reduce el riesgo de conflictos entre dependencias
- facilita abandonar el proyecto después: borrás el checkout y luego, si querés, limpiás el store de Nix

Tradeoff:

- el build igual va a consumir CPU, RAM y disco mientras compile
- Nix no elimina el costo de compilación; solo aísla mejor las dependencias
- en una máquina ARM64 modesta el primer build puede ser largo

## Limpieza cuando dejes de usar Mina

Si más adelante abandonás este entorno y querés recuperar espacio:

1. borrar el checkout:

```bash
rm -rf /home/mtrapaglia/mina/mina-rust
```

2. borrar artefactos locales de compilación si todavía existe el repo:

```bash
cd /home/mtrapaglia/mina/mina-rust
cargo clean
```

3. liberar paquetes descargados por Nix que ya no estén referenciados:

```bash
nix store gc
```

Notas:

- el aislamiento con Nix evita conflictos de versiones estilo Python en el sistema base
- lo que sí queda ocupando espacio es el store de Nix y el `target/` del repo
- borrar solo la carpeta del proyecto no limpia automáticamente el store de Nix

## 1. Verificar requisitos de máquina

Antes de empezar, confirmar:

- SO soportado: Ubuntu 22.04 / 24.04 o macOS 13 / 14 / 15
- RAM: mínimo 8 GB, recomendado 16 GB
- Disco libre: mínimo 20 GB
- Internet estable para bajar dependencias y circuitos

## 2. Instalar dependencias del sistema

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  libssl-dev \
  pkg-config \
  protobuf-compiler \
  git \
  curl \
  shellcheck
```

### macOS

```bash
if ! command -v brew >/dev/null 2>&1; then
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
fi

brew install \
  openssl \
  pkg-config \
  protobuf \
  git \
  curl \
  shellcheck
```

## 3. Instalar Rust y toolchains requeridos

La guía oficial indica `Rust 1.84` como toolchain estable más `nightly`.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

rustup install 1.84
rustup default 1.84
rustup install nightly

rustup component add rustfmt clippy rust-src --toolchain 1.84
rustup component add rustfmt clippy rust-src --toolchain nightly

cargo install taplo-cli --locked
```

## 4. Instalar herramientas adicionales

### Node.js

Necesario si más adelante vas a tocar documentación o frontend.

#### Ubuntu / Debian

```bash
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs
```

#### macOS

```bash
brew install node@18
brew link node@18
```

### Docker

Opcional. Útil para flujos containerizados.

#### Ubuntu / Debian

```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
rm get-docker.sh
```

#### macOS

```bash
brew install --cask docker
```

### Herramientas WASM

Solo si pensás trabajar con el web node.

```bash
cargo install wasm-pack
cargo install -f wasm-bindgen-cli --version 0.2.99
rustup target add wasm32-unknown-unknown
```

## 5. Clonar el repositorio

Si querés usar este directorio vacío (`/home/mtrapaglia/mina`) como root del repo:

```bash
git clone https://github.com/o1-labs/mina-rust.git .
```

Si preferís dejarlo en un subdirectorio:

```bash
git clone https://github.com/o1-labs/mina-rust.git
cd mina-rust
```

## 6. Descargar circuitos requeridos

Este paso aparece explícitamente en la guía y conviene hacerlo antes del build:

```bash
make download-circuits
```

## 7. Hacer el primer build local

### Build de desarrollo

```bash
make build
```

Este es el build mínimo recomendado para empezar a desarrollar.

### Build release

```bash
make build-release
```

Usalo si querés ejecutar el nodo compilado con optimizaciones.

## 8. Validar que el entorno quedó bien

Revisar targets disponibles:

```bash
make help
```

Validaciones básicas:

```bash
make check
make test
make test-p2p
```

Chequeos de calidad útiles antes de contribuir:

```bash
make format
make check-format
make lint
make fix-trailing-whitespace
make check-trailing-whitespace
```

## 9. Ejecutar Mina localmente

La guía muestra el arranque del binario release:

```bash
./target/release/mina node --network devnet
```

Si solo compilaste con `make build`, primero hacé `make build-release` o ubicá el binario debug equivalente en `target/debug/`.

## 10. Pasos opcionales según lo que quieras desarrollar

### Ledger

```bash
make build-ledger
```

### WebAssembly / web node

```bash
make build-wasm
```

### Testing framework

```bash
make build-testing
```

### VRF

```bash
make build-vrf
```

### Documentación local

```bash
make docs-serve
```

### Frontend

```bash
cd frontend
npm install
npm start
```

## 11. Variables de entorno para archive node

Solo si vas a trabajar con archive node:

```bash
export MINA_ARCHIVE_ADDRESS="http://localhost:3007"
export PG_USER="mina"
export PG_PW="minamina"
export PG_DB="mina_archive"
```

Luego:

```bash
make postgres-setup
make run-archive
```

## 12. Camino recomendado para arrancar sin desviarte

Orden práctico:

1. Instalar dependencias del sistema
2. Instalar Rust 1.84, nightly y componentes
3. Instalar Node.js solo si vas a tocar docs/frontend
4. Clonar el repo
5. Ejecutar `make download-circuits`
6. Ejecutar `make build`
7. Ejecutar `make test`
8. Ejecutar `make check` y `make lint`
9. Si querés correr el nodo, ejecutar `make build-release`
10. Correr `./target/release/mina node --network devnet`

## 13. Criterio de éxito

Considero que la instalación quedó bien si:

- `make build` termina sin errores
- `make test` corre al menos el set básico sin romper por toolchain faltante
- `make lint` y `make check-format` no muestran faltantes de setup
- el binario `./target/release/mina` arranca sobre `devnet`

## 14. Próximo paso sugerido

Cuando termines este plan, lo siguiente debería ser:

1. Leer la documentación de arquitectura
2. Revisar `CLAUDE.md` en la raíz del repo
3. Explorar `make help` y `tests.yaml`
4. Elegir si tu foco inicial va a ser `node`, `ledger`, `p2p`, `frontend` o `testing`
