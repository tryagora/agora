<script lang="ts">
	import Auth from '$lib/components/Auth.svelte';
	import Chat from '$lib/components/Chat.svelte';
	import Onboarding from '$lib/components/Onboarding.svelte';
	import { initPresence, resetPresence } from '$lib/presence';

	// check if onboarding was already completed — read once at startup
	let onboardingDone = $state(localStorage.getItem('agora_onboarding_done') === '1');

	// server url can be overridden by onboarding; falls back to default
	let apiUrl = $state(localStorage.getItem('agora_server_url') || 'http://localhost:3000');

	let userId = $state('');
	let accessToken = $state('');
	let isAuthenticated = $state(false);

	async function setPresence(token: string, uid: string, presence: 'online' | 'offline' | 'unavailable') {
		try {
			await fetch(`${apiUrl}/presence/set`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: token, user_id: uid, presence })
			});
		} catch {
			// non-fatal — presence is best-effort
		}
	}

	async function handleAuthSuccess(id: string, token: string) {
		userId = id;
		accessToken = token;
		isAuthenticated = true;
		await setPresence(token, id, 'online');
		// start the shared presence poll loop
		initPresence(token, apiUrl);
	}

	async function handleLogout() {
		// mark offline before clearing credentials
		if (accessToken && userId) {
			await setPresence(accessToken, userId, 'offline');
		}
		resetPresence();
		userId = '';
		accessToken = '';
		isAuthenticated = false;
	}

	function handleOnboardingComplete(serverUrl: string) {
		apiUrl = serverUrl;
		onboardingDone = true;
	}

	// set offline if the window/tab is closed while logged in
	$effect(() => {
		if (!isAuthenticated) return;
		const token = accessToken;
		const uid = userId;
		const url = apiUrl;
		function handleUnload() {
			// use sendBeacon so the request fires even as the page unloads
			const body = JSON.stringify({ access_token: token, user_id: uid, presence: 'offline' });
			navigator.sendBeacon
				? navigator.sendBeacon(`${url}/presence/set`, new Blob([body], { type: 'application/json' }))
				: fetch(`${url}/presence/set`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body, keepalive: true });
		}
		window.addEventListener('beforeunload', handleUnload);
		return () => window.removeEventListener('beforeunload', handleUnload);
	});

	// idle detection + heartbeat
	// - marks user as "unavailable" after 3 minutes of no mouse/keyboard activity
	// - marks back to "online" immediately on any activity
	// - heartbeats every 2 minutes to keep the redis TTL alive
	$effect(() => {
		if (!isAuthenticated) return;
		const token = accessToken;
		const uid = userId;
		const url = apiUrl;

		const IDLE_MS = 3 * 60 * 1000; // 3 minutes
		const HEARTBEAT_MS = 120_000;   // 2 minutes

		let currentPresence: 'online' | 'unavailable' = 'online';
		let idleTimer: ReturnType<typeof setTimeout>;
		let heartbeat: ReturnType<typeof setInterval>;

		function goIdle() {
			if (currentPresence === 'unavailable') return;
			currentPresence = 'unavailable';
			setPresence(token, uid, 'unavailable');
		}

		function goActive() {
			clearTimeout(idleTimer);
			idleTimer = setTimeout(goIdle, IDLE_MS);
			if (currentPresence === 'unavailable') {
				currentPresence = 'online';
				setPresence(token, uid, 'online');
			}
		}

		// activity events to listen for
		const events = ['mousemove', 'mousedown', 'keydown', 'pointerdown', 'scroll', 'touchstart'];
		for (const e of events) window.addEventListener(e, goActive, { passive: true });

		// start the idle countdown and heartbeat
		idleTimer = setTimeout(goIdle, IDLE_MS);
		heartbeat = setInterval(() => {
			// only heartbeat if currently online — no need to keep refreshing unavailable TTL
			if (currentPresence === 'online') setPresence(token, uid, 'online');
		}, HEARTBEAT_MS);

		return () => {
			clearTimeout(idleTimer);
			clearInterval(heartbeat);
			for (const e of events) window.removeEventListener(e, goActive);
		};
	});
</script>

<svelte:head>
	<script>
		// Force dark mode by default
		document.documentElement.classList.add('dark');
	</script>
</svelte:head>

<main class="h-screen w-screen bg-background text-foreground">
	{#if !onboardingDone}
		<!-- show onboarding wizard on first launch -->
		<Onboarding onComplete={handleOnboardingComplete} />
	{:else if isAuthenticated}
		<Chat {userId} {accessToken} {apiUrl} onLogout={handleLogout} />
	{:else}
		<!-- auth fills the screen and handles its own layout + background -->
		<Auth {apiUrl} onAuthSuccess={handleAuthSuccess} />
	{/if}
</main>
