# FolderAI
An open-source, AI-native file explorer with semantic search and autonomous agents.

[Requisiti del primo rilascio](requirements/README.md)

## Sviluppo desktop

Usare Node.js 24 con npm e un toolchain Rust stabile. La build locale è stata verificata con Rust 1.99.1 su Apple Silicon. Su macOS servono gli Xcode Command Line Tools. Su Windows 11 x64 servono il toolchain Rust `x86_64-pc-windows-msvc`, Microsoft C++ Build Tools e WebView2. [Prerequisiti Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
npm ci
npm run tauri -- dev -- --locked
```

Per verificare tipi, build web e backend dalla radice del repository.

```sh
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Per costruire il bundle locale usare `npm run tauri -- build --bundles app -- --locked` su macOS oppure `npm run tauri -- build --bundles nsis -- --locked` su Windows. I risultati sono nella directory `src-tauri/target/release/bundle/`.

Il bundle macOS locale usa la firma ad hoc. Per una build con un’identità di firma configurata, impostare `APPLE_SIGNING_IDENTITY`. [Firma macOS in Tauri](https://v2.tauri.app/distribute/sign/macos/)

FolderAI apre la Home del sistema in sola lettura. Si possono aprire cartelle, navigare con il percorso o con la cronologia e riprovare quando una posizione è indisponibile. La Home si può usare offline senza configurare l’AI. Sui sistemi candidati la copertura delle primitive native è registrata nella [matrice delle piattaforme](docs/platform-probes/README.md).
