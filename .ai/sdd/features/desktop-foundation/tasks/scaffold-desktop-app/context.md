# Task: scaffold-desktop-app

## Type
implementation

## Description
Establish Tauri 2, Rust, React, TypeScript and SCSS, lockfiles, dev/build commands and modular boundaries.

Traceability covers RT-001 RT-003 RT-034.

## Depends On
- No implementation dependency. The platform matrix remains a feature acceptance dependency.

## Likely Files
- src/ and build configuration

## Progress
- [x] Add the Tauri 2 app, Vite/React/TypeScript/SCSS tooling, npm and Cargo lockfiles and native icons.
- [x] Document build, test and local packaging in the root README. Use native window decorations.
- Deferred release requirement: [native Windows 11 x64 build, installation and launch](../../../release-acceptance/tasks/installation-updates-and-release-signoff/context.md).

## Validation
- Clean temporary copy on macOS Apple Silicon: `npm ci --offline` from the preloaded registry cache and `npm run build` passed.
- `cargo check --locked --offline --manifest-path src-tauri/Cargo.toml`, formatting, tests and Clippy passed.
- Native WebKit loaded the app from the development server and rendered the synthetic Home fixture. `CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app -- --locked` built the arm64 release bundle. The application stayed running on a synthetic profile without the server. `codesign --verify --strict` passed on the ad hoc signed bundle.
- The npm production audit reported zero advisories. Vite was updated to 8.3.8 to remove direct advisories from the initial install. The full audit still flags the dev-only transitive `picomatch` 4.0.7. The configured registry did not list a newer version.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Scaffold integrated in the root frontend configuration, `src/main.tsx` and `src-tauri/`. Continue on Windows 11 x64 with `npm ci`, `npm run tauri -- dev -- --locked` and `npm run tauri -- build --bundles nsis -- --locked`. Install the resulting local package and verify startup as a standard user. The user confirmed no Windows environment is currently available.
- 2026-11-18 codex. The native macOS build and startup pass the foundation checkpoint. The Windows build and startup are assigned to release acceptance.
