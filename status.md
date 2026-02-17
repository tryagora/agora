---
# agora — project status

last updated: 2026-02-17

## current focus
full stack functional — need to test end-to-end messaging

## completed
- 2026-02-17 initialized git repository
- 2026-02-17 created monorepo directory structure (backend, clients, infrastructure)
- 2026-02-17 set up docker compose with conduit, postgres, redis, livekit
- 2026-02-17 created axum api skeleton with modular routing
- 2026-02-17 created next.js web client skeleton
- 2026-02-17 created tauri mobile client skeleton
- 2026-02-17 added readme and gitignore
- 2026-02-17 created health check script for services
- 2026-02-17 implemented basic auth endpoints in axum (register/login stubs)
- 2026-02-17 created matrix client library for backend api
- 2026-02-17 improved conduit configuration with encryption and media storage
- 2026-02-17 **conduit homeserver running on localhost:8448** — supports matrix spec v1.5
- 2026-02-17 fixed conduit binding issue — added `address = "0.0.0.0"` to config
- 2026-02-17 **implemented matrix registration via axum api** — handles UIA flow with m.login.dummy
- 2026-02-17 **implemented matrix login via axum api** — returns access_token and device_id
- 2026-02-17 tested user registration and login against conduit — both working
- 2026-02-17 **replaced tauri 1.x with tauri 2 + svelte 5 + shadcn-svelte**
- 2026-02-17 **created auth ui in tauri client** — connects to axum api for registration/login
- 2026-02-17 **added database connection pool to axum api** — postgres with sqlx
- 2026-02-17 **created initial database migrations** — users and sessions tables
- 2026-02-17 **implemented matrix sync endpoint** — /sync fetches messages from conduit
- 2026-02-17 **created chat interface in tauri** — two-pane layout with sidebar and message view
- 2026-02-17 **connected auth to chat flow** — successful login switches to chat view

## in progress

## up next
- test full end-to-end: register → login → sync messages → send message
- join a room and test actual matrix messaging
- add livekit voice integration

## known issues
- flutter lsp errors from old file still cached (harmless)

## decisions log
- 2026-02-17 initialized agora as monorepo with separate directories for each major component (backend, clients, infrastructure)
- 2026-02-17 following strict lowercase convention throughout codebase
- 2026-02-17 using snake_case for rust and dart code
- 2026-02-17 conduit 0.7.0 for matrix homeserver
- 2026-02-17 switched from flutter to tauri for mobile/desktop client
- 2026-02-17 **migrated to tauri 2 + svelte 5 + shadcn-svelte** — modern stack with better dx and mobile support
- 2026-02-17 axum api structured with modular routes (health, auth, sync) and state management
- 2026-02-17 matrix client in rust backend to proxy requests to conduit
- 2026-02-17 conduit must bind to 0.0.0.0 (not localhost) inside docker container to accept external connections
- 2026-02-17 matrix uia (user-interactive authentication) flow requires two-step registration: first get session, then complete with auth
- 2026-02-17 conduit returns `home_server` in login response but not in registration response — made field optional in rust structs
- 2026-02-17 **api gracefully handles missing database/redis** — continues operating with just matrix functionality
