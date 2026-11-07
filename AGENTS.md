# AGENTS.md

ClipRTC is a LAN clipboard-sync app: Tauri v2 (Rust backend) + React 19 / Vite 7 / Tailwind v4 / shadcn UI. One root pnpm app; the Rust crate is in `src-tauri/`.

## Commands

- `pnpm install` — pnpm only. `pnpm-lock.yaml` is the source of truth; npm/yarn will create a second lockfile.
- `pnpm tauri dev` — full app (Vite on 1420 + Rust). Use this for any Tauri API work.
- `pnpm dev` — frontend only; most `@tauri-apps/*` calls fail outside a webview.
- `pnpm build` — `tsc && vite build`. This is the only typecheck + bundle step; there is no separate `typecheck` script.
- `pnpm tauri build` — production bundle (runs `pnpm build` first); CI does the same per platform.
- `pnpm lint` / `pnpm lint:fix` — ESLint (`@antfu/eslint-config` + React). `no-console` is a warning; `curly` is `multi-line`/`consistent`.
- `cargo check --manifest-path src-tauri/Cargo.toml` — Rust check. `cargo clippy --manifest-path src-tauri/Cargo.toml` and `cargo fmt --manifest-path src-tauri/Cargo.toml` also apply.
- `cargo test -p cliprtc-core --manifest-path src-tauri/Cargo.toml` — headless `cliprtc-core` tests. The core crate must not depend on Tauri.
- `cargo clippy -p cliprtc-core --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` — core lint gate.
- `cargo test --manifest-path src-tauri/Cargo.toml` — currently only the new core crate has tests; the legacy Tauri crate still has none. There is no JS test runner and no pre-commit hook; `lint-staged` is installed but unconfigured. Never claim "tests pass" without adding/running tests.
- CI (`.github/workflows/ci.yml`) runs on push/PR: frontend lint+build, plus core fmt/clippy/test on Windows/macOS/Ubuntu. Release CI (`.github/workflows/publish-to-auto-release.yml`) runs only on `v*` tags or manual dispatch.
- The release version lives in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`; bump all three together.
- Commit history uses Conventional Commits (`feat:`, `fix:`, `chore(ci):`, ...); keep that style.

## Runtime wiring

- `src-tauri/src/lib.rs` builds Tauri, stores `APP`, then calls `init::start()` in `setup`. `init::start` starts the clipboard watcher, the QUIC endpoint (`0.0.0.0:0`), then mDNS register + browse.
- `start_server` calls `init::start()` again. It is exposed to the frontend but unused; it is **not** idempotent (duplicate watcher, endpoint, and mDNS daemon).
- Two windows (`main`, `settings`) load the same SPA at `/main` and `/settings`. The tray is created from the **settings** webview (`src/utils/init.ts`) even though that window starts hidden. Closing either window only hides it; the app exits via tray `exit_app`.
- Rust globals in `utils/constant.rs` (`STORE_INFO`, `STORE_SETTINGS`, `UUID`, `KEY`, `DEVICE_ID`, `CLIPBOARD_INFO`, `APP_TMP_DIR`) are `Lazy`/`OnceCell` and assume `APP` is already set. Using them before Tauri `setup` panics.
- Dev and release use different store files: `.info.dev.json` / `.settings.dev.json` in debug builds, `.info.json` / `.settings.json` in release. Both `src-tauri/src/utils/constant.rs` and `src/utils/store.ts` encode this; keep them in sync.
- Discovery: mDNS service is `_cliprtc._quic._udp.local.`; TXT records are `version`, `port`, `uuid`, and an AES-GCM-encrypted `fingerprint`. `mdns/query.rs` keys `DEVICES` by mDNS `fullname`; the UI keys peers by `uuid`. `uuid` is persisted, while the mDNS instance name is the hostname (`DEVICE_ID`).
- Transfer framing (`quic/struct_type.rs`): one unidirectional QUIC stream per clipboard event per linked peer. Body = `u32` big-endian JSON meta length + JSON `PayloadMeta` + raw payload. Text/image are raw; File payloads are an **uncompressed tar** of the selected paths.
- `PayloadMeta.total_size` is advisory; the receiver reads until stream end and never verifies it.
- Dedup uses `PayloadMeta.seq` against `CLIPBOARD_INFO.remote_seq_map[source_id]`.
- `EClipboardKind` is only `Text` / `Image` / `File`. Directories are tarred by the sender and untarred under `APP_TMP_DIR_FILES_CACHE` on the receiver.
- Adding a Tauri command requires registering it in `lib.rs` `invoke_handler` **and** adding a typed wrapper in `src/utils/cmds.ts`. New UI strings must be added to both `src/locales/en-US.ts` and `src/locales/zh-CN.ts`.
- Tailwind v4 is wired through `@tailwindcss/vite` and `src/App.css` (CSS variables); there is no `tailwind.config.*`. The `@/` alias maps to `src/` in both `vite.config.ts` and `tsconfig.json`.
- App-data identifier is `cn.imlete.tauri.cliprtc` (`tauri.conf.json`). Deleting its app-data directory resets UUID, key, linked peers, and stores.

## Security model: do not trust README/UI claims

- QUIC provides encryption, but peer authentication is **not** per-device. `ALLOW_FINGERPRINT` is one global set of every certificate seen from any mDNS peer; a client accepts any fingerprint in that set, not necessarily the peer it intended to reach.
- The server authorizes on `ALLOW_DEVICE_IDS.contains(&meta.source_id)`, but `source_id` is self-declared by the sender in `PayloadMeta` and is not bound to the TLS certificate. A discovered peer can claim another device's UUID.
- `KEY` (default literal `"cliprtc"`) only encrypts the certificate fingerprint in mDNS TXT. Anyone on the LAN who knows or guesses the default key can register a valid-looking service. mDNS also exposes `uuid` in cleartext. The README/UI wording ("Private & secure", "only users with the same key can access") overstates this.
- `quic/mod.rs` generates a fresh self-signed certificate on every launch. Peer identity is therefore the per-session fingerprint advertised over mDNS; it is not a stable cryptographic identity.
- `quic/server.rs::handle_file` does `save_path.push(&name)` with a peer-supplied name, then untars into a derived directory. Treat it as path traversal until explicitly fixed; validate/canonicalize names.
- `QuicTransfer::recv_payload` trusts a wire `u32 meta_len` and allocates that many bytes. Cap it before allocating.
- The mDNS TXT `version` is hardcoded to `"1.0"` and never compared, so peers with different framing are still accepted. Add a compatibility check before changing `PayloadMeta`.
- The QUIC server accepts every incoming connection and spawns a task, with no connection/stream caps or read timeouts; the only handshake gate is the global fingerprint set.
- `ALLOW_DEVICE_IDS` is in-memory only. The frontend persists the linked-peer list in the `.info` store and replays `allow_device_add` on startup; if the main window never loads, no peer is authorized.

## Discovery / transfer / stability landmines

- `mdns/query.rs` resets `remote_seq_map[source_id] = 0` on **every** `ServiceResolved`, including periodic re-announcements. Old/replayed payloads can be accepted after a refresh. Conversely, `seq` is a per-process counter, so if a peer restart misses the reset, new events are dropped until `seq` exceeds the stale value.
- `ServiceEvent::ServiceRemoved` removes a device from `DEVICES` but does **not** emit `DEVICE_FOUND`, so the UI can keep showing stale peers until something else refreshes it.
- `init::start` drops the `ServiceDaemon` handle (`_mdns_handle`). The daemon thread keeps running, but there is no way to shut it down or re-register; repeated `start_server` calls leak daemons.
- The client picks `device.addresses.v4.first()` with no reachability filter. mDNS includes VPN/Docker/secondary interfaces, and IPv6 is discovered but never used for sending. "Visible but cannot connect" is usually this.
- QUIC binds an ephemeral port (`0`) on every launch; mDNS advertises it, but per-port firewall allowlists will not survive a restart.
- `quic/client.rs` iterates `DEVICES.iter()` and moves the DashMap `RefMulti` guard into `tokio::spawn`, holding a shard read lock across `connect` and the whole send. Do not hold DashMap guards across `.await`.
- Clipboard cache race: `clipboard/mod.rs` deletes `APP_TMP_DIR_FILES_CACHE` on every local clipboard change, including while a file tar may still be opening or sending.
- Echo suppression is one global `AtomicBool` (`CLIPBOARD_INFO.lock`). Two peers writing concurrently, or a local copy during a remote apply, can be misclassified and re-broadcast; a counter/queue is needed for multi-peer correctness.
- `CLIPBOARD_INFO` uses an unbounded mpsc channel. Rapid large copies have no backpressure; `quic/client.rs` also clones the full text/image body per peer (`data.content.clone()`).
- Throughput is unoptimized: uncompressed tar, `tokio::io::copy`'s 8 KiB buffer, a fresh QUIC connection per event per peer, and default `TransportConfig` (no tuning/keepalive). There is no retry or reconnect; failures are only logged.
- Tar compression runs synchronously in the clipboard watcher callback; tar decompression and clipboard `set_*`/image decode run inside Tokio tasks. Large directories or images can block the watcher/runtime; move heavy work to `spawn_blocking` or a worker.
- `mdns/register.rs` uses the hostname as the service instance name, so duplicate hostnames can collide. mDNS registration/browse errors and several startup paths are `.expect()`/`.unwrap()`ed and can panic.
- `quic/server.rs::handle_file` calls `fs::read_dir(...).unwrap()`; a missing extract directory panics that request task.
- `src/views/settings/main.tsx` `About` has a `useEffect` with no dependency array, so it re-runs every render; do not copy it as a data-loading pattern.
- Two instances on one machine share the persisted `uuid` and the same app-data store. Discovery drops peers whose `uuid == UUID` (self-filter), so local two-instance testing will not work; use two hosts/VMs or separate app-data dirs.
- `tauri.conf.json` sets `security.csp: null`; both windows also set `dragDropEnabled: false`. `src-tauri/gen/**` is generated, and `dist/`, `src-tauri/target/`, and `node_modules/` are ignored.

## Refactor status

- The approved design is `docs/refactor-design.md`; Phase 0 plan is `docs/superpowers/plans/2026-11-07-core-v2-phase-0.md`.
- `src-tauri/crates/cliprtc-core` is the new headless core crate. Phase 0 currently contains: deterministic room CA (`HKDF-SHA256` -> Ed25519), per-device Ed25519 identity, CA-issued device certs (SAN `cliprtc://device/<uuid>`), trust store, protocol envelope/codec, Lamport clock, dedup window.
- The legacy Tauri runtime in `src-tauri/src/{quic,mdns,clipboard}` is still active. Do not assume the new core is wired into the app yet.
- Later phases (transport/session, discovery, file transfer, clipboard actor, persistence, typed IPC, UI) are not implemented.

## Improvement priorities (from the audit)

1. Security: bind each certificate fingerprint to its `uuid` and authorize per linked peer; cap `meta_len`; sanitize/canonicalize receive paths; remove the `"cliprtc"` default or replace it with a real pairing/PSK flow.
2. Discovery: keep the `ServiceDaemon` handle, emit refresh on removal, stop resetting `remote_seq_map`, filter reachable addresses (and use IPv6), and use a unique stable service instance name.
3. Throughput: reuse QUIC connections, send `Bytes`/`Arc` instead of cloning, use a larger copy buffer, compress text/large payloads, and add backpressure.
4. Stability: make `init::start` idempotent, remove startup/receive panics, avoid holding DashMap guards across `.await`, add retry/timeouts, and add tests (Rust unit tests plus a two-peer harness).

## Verification expectations

- Frontend: `pnpm lint` and `pnpm build`.
- Rust core: `cargo fmt -p cliprtc-core -- --check`, `cargo clippy -p cliprtc-core --all-targets -- -D warnings`, `cargo test -p cliprtc-core` (run from `src-tauri/` or with `--manifest-path src-tauri/Cargo.toml`).
- Rust app: `cargo check --manifest-path src-tauri/Cargo.toml` (add `cargo clippy` when changing protocol/TLS paths).
- End-to-end: run on two hosts on the same LAN with the same key; test text, PNG image, single file, directory, and a large file. Debug builds open both devtools automatically.
- The legacy runtime still has no automated tests, so any behavior change to discovery, framing, TLS, or file receive needs a manual two-peer check plus log inspection. The new core must not regress `cargo test -p cliprtc-core`.
