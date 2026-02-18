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

	async function setPresence(token: string, uid: string, presence: 'online' | 'offline') {
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

	// heartbeat: refresh the redis presence TTL every 2 minutes while logged in.
	// the server-side TTL is 5 minutes, so this gives a 3-minute grace window
	// before presence expires if the app crashes.
	$effect(() => {
		if (!isAuthenticated) return;
		const token = accessToken;
		const uid = userId;
		const url = apiUrl;
		const interval = setInterval(() => setPresence(token, uid, 'online'), 120_000);
		return () => clearInterval(interval);
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
