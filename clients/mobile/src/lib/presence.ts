/**
 * shared presence store — websocket edition
 *
 * connects to /ws/presence once on login. the server sends the full snapshot
 * of currently-online users on connect, then pushes every subsequent change
 * instantly. components call track(userId) so the store knows which users are
 * interesting (used for the one-shot HTTP fallback when the socket isn't ready
 * yet). the store itself is updated by the websocket, so all components
 * — member list, friends list, profile modal — see changes at the same time
 * with no polling delay.
 */

import { writable, get } from 'svelte/store';

// map from user_id → presence string ("online" | "offline" | "unavailable")
export const presenceMap = writable<Record<string, string>>({});

// reference counts — how many components currently care about each user
const watchCounts = new Map<string, number>();

let currentToken = '';
let currentApiUrl = 'http://localhost:3000';
let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1_000; // ms, doubles on each failure up to 30s
let intentionalClose = false;

/** call once when the user logs in */
export function initPresence(accessToken: string, apiUrl: string) {
	currentToken = accessToken;
	currentApiUrl = apiUrl;
	intentionalClose = false;
	reconnectDelay = 1_000;
	connect();
}

/** call on logout */
export function resetPresence() {
	intentionalClose = true;
	currentToken = '';
	if (reconnectTimer !== null) {
		clearTimeout(reconnectTimer);
		reconnectTimer = null;
	}
	if (socket) {
		socket.close();
		socket = null;
	}
	watchCounts.clear();
	presenceMap.set({});
}

/** register interest in a user; returns cleanup fn */
export function track(userId: string): () => void {
	watchCounts.set(userId, (watchCounts.get(userId) ?? 0) + 1);
	// if the socket isn't open yet (connecting / backoff), do a one-shot HTTP
	// fetch so the component doesn't show a stale "offline" on first render
	if (!socket || socket.readyState !== WebSocket.OPEN) {
		fetchOnce(userId);
	}
	return () => {
		const count = watchCounts.get(userId) ?? 0;
		if (count <= 1) watchCounts.delete(userId);
		else watchCounts.set(userId, count - 1);
	};
}

function connect() {
	if (!currentToken) return;

	// build ws url — replace http(s) scheme with ws(s)
	const wsBase = currentApiUrl.replace(/^http/, 'ws');
	const url = `${wsBase}/ws/presence?access_token=${encodeURIComponent(currentToken)}`;

	socket = new WebSocket(url);

	socket.onopen = () => {
		reconnectDelay = 1_000; // reset backoff on successful connect
	};

	socket.onmessage = (ev) => {
		try {
			const event = JSON.parse(ev.data) as { user_id: string; presence: string };
			if (event.user_id && event.presence) {
				presenceMap.update((m) => ({ ...m, [event.user_id]: event.presence }));
			}
		} catch {
			// malformed frame — ignore
		}
	};

	socket.onclose = () => {
		socket = null;
		if (intentionalClose) return;
		// exponential backoff reconnect
		reconnectTimer = setTimeout(() => {
			reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
			connect();
		}, reconnectDelay);
	};

	socket.onerror = () => {
		// onclose will fire after onerror — reconnect handled there
	};
}

async function fetchOnce(userId: string) {
	if (!currentToken) return;
	try {
		const res = await fetch(
			`${currentApiUrl}/presence/get?access_token=${encodeURIComponent(currentToken)}&user_id=${encodeURIComponent(userId)}`
		);
		if (res.ok) {
			const data = await res.json();
			presenceMap.update((m) => ({ ...m, [userId]: data.presence ?? 'offline' }));
		}
	} catch {
		// network blip — leave previous value
	}
}

/** helpers used by components */
export function presenceDotClass(p: string): string {
	if (p === 'online') return 'bg-green-500';
	if (p === 'unavailable') return 'bg-yellow-500';
	return 'bg-muted-foreground/50';
}

export function presenceLabel(p: string): string {
	if (p === 'online') return 'online';
	if (p === 'unavailable') return 'away';
	return 'offline';
}
