# M3 Frontend Migration — Notes (IN PROGRESS; not a stage release)

Status as of M3-CP002 (2026-11-27): READY FOR REVIEW.

## Approach
The existing React components are reused unchanged. A separate web build (`vite.web.config.ts`) aliases the Tauri modules to browser shims, so `src/api.ts` still calls `invoke("command", args)` but the call lands in `src/web/transport.ts`, which maps it onto `/api/v1`. The desktop build (`vite.config.ts`, `src/api.ts`, every existing component) is untouched; **no existing source file was modified** (verified by diff against M2-CP001; only `package.json` scripts were added).

| Piece | File |
|---|---|
| Wire types (backend JSON, English) | `src/web/wire.ts` |
| Pure mappers wire → existing desktop types (Node-tested) | `src/web/adapt.ts`, `web-tests/adapt.test.mjs` |
| Typed HTTP client (`X-Requested-With`, JSON error shape) | `src/web/http.ts` |
| `invoke` replacement | `src/web/transport.ts`, `src/web/shims/core.ts` |
| Dialog / opener / event shims | `src/web/shims/{dialog,opener,event}.ts` |
| "Not available yet" mechanism (error + toast) | `src/web/unavailable.ts` |
| Web shell (workspace list, upload, tabs, Download, reuses `<Belge>`) | `src/web/WebApp.tsx`, `src/web/main.tsx`, `web.html` |
| Static serving + SPA fallback | `web/server` (`STATIC_DIR`), tested |

Commands: `npm run build:web` (→ `dist-web/`), `npm run dev:web` (Vite on :1421, proxies `/api` to :8080), `npm run test:web` (Node tests), then
`cd web && STATIC_DIR=../dist-web COOKIE_SECURE=false cargo run -p zyn-server`.

## What works (verified)
Upload `.rpf` → workspace list → open via the real core → existing tree/summary UI renders it → "Download archive" (and the existing "Farklı kaydet") downloads the verified re-serialized archive → reload restores the workspace and open tabs (session cookie) → closing a tab closes the server document. Hostile entry names render as inert text.

## Implemented commands (4 of ~60 `api.ts` commands, plus one stub)
`arsiv_ac` (only via `web-file:<id>` handles), `belge_ozet`, `belge_kapat`, `acik_belgeler`; `ayarlar_oku` returns `{}` (there are genuinely no stored settings). **Every other command rejects with `NOT_AVAILABLE_IN_WEB`** ("… is not available in the web version yet."). File dialogs, "show in folder" and undo/redo show a toast and behave as cancelled. Nothing is faked as success.

## Bugs found and fixed during M3 (kept for the record)
- Initial-load effect depended on the unstable `ui` context object → one failed request re-triggered itself through its own error toast (hundreds of requests, `ERR_INSUFFICIENT_RESOURCES`), and every toast re-fetched and reset the active tab. Reproduced in 2 of 6 runs; fixed with a ref; 10/10 clean runs afterwards.
- Parallel first requests with no cookie created 3 workspaces per page load; the first request is now sequential (1 workspace per browser).

## Known limitations
- UI text of reused components is still Turkish; the new shell is English. Translation is M7.
- Editing/saving/undo, textures, models, maps, AI panel, backups, mod conflicts: backend routes do not exist yet (M4+); the matching buttons show the "not available" notice.
- The existing Save button stays disabled (documents are never dirty yet); use "Download archive".
- Drag-and-drop: dropping `.rpf` files onto the window uploads and opens them (tested in Chromium with synthetic `DataTransfer` events, not a native OS drag). Folder drops and the desktop's per-panel drop targets are not implemented.
- Any anonymous request to `/api/v1/*` creates a workspace (50 max by default): a bot could exhaust capacity. Needs rate limiting (M8).
- Desktop `tauri build` and the desktop app were not run; only `tsc` and `vite build` of the desktop frontend.
- Web bundle is large (AI/three.js/CodeMirror chunks are lazy but shipped); no CSP is set for the SPA yet (strict CSP is only on the dev harness).
