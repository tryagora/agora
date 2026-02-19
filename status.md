---
# agora — project status

last updated: 2026-02-19 (vibe rooms)

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
- 2026-02-17 **added server/room management endpoints** — create, join, list rooms and spaces
- 2026-02-17 **added member management** — get room members and invite users
- 2026-02-17 **created discord-like three-pane ui** — server list + channel list + chat
- 2026-02-17 **implemented server creation/joining** — create spaces and join via room id
- 2026-02-17 **implemented channel management** — create channels within servers
- 2026-02-17 **wired up real message sending** — POST /rooms/send route calls matrix send_message
- 2026-02-17 **implemented space hierarchy** — channels belong to parent server via m.space.child state events
- 2026-02-17 **real channel name in chat header** — displays actual channel name
- 2026-02-17 **member list sidebar** — shows room members with presence dots
- 2026-02-17 **message deduplication** — sync uses event_id to avoid duplicate messages
- 2026-02-17 **auto-scroll messages** — chat area scrolls to bottom when new messages arrive
- 2026-02-17 **added direct message (DM) functionality** — DmList component, start DMs with any user
- 2026-02-17 **DM mode toggle** — clicking home button switches to DM view vs server view
- 2026-02-17 **server management panel** — ServerManage component with permissions editor
- 2026-02-17 **default dark mode** — app launches in dark mode automatically
- 2026-02-17 **admin-only create channel button** — only server admins can create channels
- 2026-02-17 **categories support** — channels organized into collapsible categories (subspaces)
- 2026-02-17 **permissions endpoints** — GET/POST /rooms/permissions for power level management
- 2026-02-17 **leave server UI** — ServerList and ServerManage have functioning leave buttons
- 2026-02-17 **channel deletion UI** — admin-only delete buttons with confirmation dialogs
- 2026-02-17 **restructured ChannelList** — supports categories with collapsible sections
- 2026-02-17 **PowerLevels methods to Matrix client** — get_power_levels, set_power_levels, create_category
- 2026-02-18 **friends system** — friends table (migration 002), /friends backend routes, FriendsList.svelte
- 2026-02-18 **friend request flow** — send/accept/reject requests by @username
- 2026-02-18 **friends DM** — clicking message on a friend creates/reuses a Matrix DM room
- 2026-02-18 **fixed DM invite/join** — get_or_create_dm calls join_room on every open
- 2026-02-18 **fixed DM room named "undefined"** — create_dm_room sets m.room.name
- 2026-02-18 **real presence** — POST /presence/set and GET /presence/get via Matrix presence API
- 2026-02-18 **profile system** — GET/PUT /profile/get for displayname and avatar
- 2026-02-18 **ProfileModal.svelte** — shows avatar, displayname, presence dot, status message; edit own profile
- 2026-02-18 **animated login background** — Grainient.svelte WebGL animated gradient behind auth card
- 2026-02-18 **onboarding wizard** — Onboarding.svelte: 4-step wizard (welcome → notifications → server url → done)
- 2026-02-18 **configurable API URL** — server URL configurable during onboarding, saved to localStorage
- 2026-02-18 **notification system** — Chat.svelte fires desktop notifications for new messages via tauri-plugin-notification
- 2026-02-18 **fixed presence always offline** — Redis-backed presence with 5-min TTL and 2-min heartbeat
- 2026-02-18 **fixed Tauri desktop notifications** — installed tauri-plugin-notification, uses sendNotification API
- 2026-02-18 **realtime presence via websocket** — /ws/presence endpoint with broadcast channel; frontend uses persistent WebSocket with auto-reconnect
- 2026-02-18 **user panel and settings** — UserPanel.svelte at bottom of sidebars with avatar, status picker, settings button; SettingsModal.svelte with account/appearance/notifications/about tabs
- 2026-02-18 **livekit voice integration** — POST /voice/token endpoint (JWT via jsonwebtoken crate); voice channel type stored as agora.room.type Matrix state event; VoiceChannel.svelte with join/leave/mute/participant list with speaking indicators; voice channels show 🔊 icon in ChannelList; clicking voice channel replaces main chat area; DM call button in chat header shows VoiceChannel panel inline above messages; livekit-client npm package installed
- 2026-02-18 **DM call signaling** — POST /voice/call backend endpoint sends agora.call Matrix messages; Chat.svelte detects ring/accept/cancel events in sync loop; IncomingCall.svelte full-screen overlay with Web Audio API ringtone, pulsing ring animation, accept/decline, 30s auto-decline; outgoing "calling..." overlay with cancel; call button in DM header now triggers startDmCall() instead of toggling inline panel
- 2026-02-18 **persistent voice bar** — ChannelList.svelte shows green animated "voice connected" bar at sidebar bottom when connected to a voice channel; includes disconnect button; state flows down from Chat.svelte via activeVoiceChannelId/activeVoiceChannelName props
- 2026-02-18 **voice participant polling** — ChannelList polls /voice/participants every 5s for all voice channels; shows participant count badge on channel row and participant name list below each voice channel with green presence dot
- 2026-02-18 **GET /voice/participants** backend endpoint — calls LiveKit REST API with admin JWT; returns empty list gracefully if LiveKit unreachable

- 2026-02-19 **vibe rooms** — one person sets a shared ambient vibe (rain/lo-fi/campfire/space/off) that all voice channel participants hear; vibe stored as agora.vibe Matrix state event; GET/POST /voice/vibe backend endpoints; VibeRoom.svelte component with Web Audio API procedural sound synthesis (no asset files); VoiceChannel.svelte polls vibe every 5s and renders VibeRoom picker; animated sound-wave status bar and active-vibe pulse animation; cross-fade between vibes; nothing like this exists in Discord, Element, or Signal

## in progress

## up next
- create web client with same features
- self-hostable single `docker compose up` (backend api in compose)
- push notifications via tauri

## completed (continued)

## known issues
- flutter lsp errors from old file still cached (harmless)
- redis v0.24 and sqlx-postgres v0.7.4 have future-compat warnings (still compile fine)

## decisions log
- 2026-02-17 initialized agora as monorepo with separate directories for each major component (backend, clients, infrastructure)
- 2026-02-17 following strict lowercase convention throughout codebase
- 2026-02-17 using snake_case for rust and dart code
- 2026-02-17 conduit 0.7.0 for matrix homeserver
- 2026-02-17 switched from flutter to tauri for mobile/desktop client
- 2026-02-17 **migrated to tauri 2 + svelte 5 + shadcn-svelte** — modern stack with better dx and mobile support
- 2026-02-17 axum api structured with modular routes (health, auth, rooms, sync) and state management
- 2026-02-17 matrix client in rust backend to proxy requests to conduit
- 2026-02-17 conduit must bind to 0.0.0.0 (not localhost) inside docker container to accept external connections
- 2026-02-17 matrix uia (user-interactive authentication) flow requires two-step registration: first get session, then complete with auth
- 2026-02-17 conduit returns `home_server` in login response but not in registration response — made field optional
- 2026-02-17 **api gracefully handles missing database/redis** — continues operating with just matrix functionality
- 2026-02-17 **discord servers map to matrix spaces** — spaces are rooms with type "m.space"
- 2026-02-17 **discord channels map to matrix rooms** — child rooms of a space, linked via m.space.child
- 2026-02-17 **all api communication in client goes through fetch() to axum** — no tauri IPC needed
- 2026-02-18 **presence stored in redis** — keys with 5-minute TTL, heartbeat every 2 minutes, graceful offline fallback
- 2026-02-18 **websocket presence broadcast** — single broadcast channel in AppState, all clients receive instant updates
- 2026-02-18 **Tauri notification plugin** — uses tauri-plugin-notification for native desktop notifications
