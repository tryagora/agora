# agora

a free open-source federated discord-like communication platform

## architecture

- **homeserver**: [conduit](https://conduit.rs) (matrix)
- **voice/video**: [livekit](https://livekit.io)
- **mobile/desktop**: [tauri](https://tauri.app) with flutter
- **web client**: [next.js](https://nextjs.org)
- **custom api**: [axum](https://github.com/tokio-rs/axum) (rust)
- **database**: postgresql
- **pubsub/presence**: redis
- **reverse proxy**: traefik

## development

### prerequisites

- docker & docker compose
- rust toolchain
- flutter sdk
- node.js 20+

### quick start

```bash
docker compose up -d
```

## project structure

```
agora/
├── backend/          # axum api and rust workers
├── clients/          # mobile (tauri/flutter) and web (next.js)
├── infrastructure/   # docker, k8s, traefik configs
├── docs/            # documentation
└── scripts/         # utility scripts
```

## principles

- free and open source forever
- federation first
- privacy respecting
- no subscriptions, no data harvesting

## license

agpl-3.0
