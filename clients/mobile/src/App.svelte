<script lang="ts">
	import Auth from '$lib/components/Auth.svelte';
	import Chat from '$lib/components/Chat.svelte';

	let userId = $state('');
	let accessToken = $state('');
	let isAuthenticated = $state(false);

	function handleAuthSuccess(id: string, token: string) {
		userId = id;
		accessToken = token;
		isAuthenticated = true;
	}

	function handleLogout() {
		userId = '';
		accessToken = '';
		isAuthenticated = false;
	}
</script>

<main class="h-screen w-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
	{#if isAuthenticated}
		<Chat {userId} {accessToken} onLogout={handleLogout} />
	{:else}
		<div class="flex h-full items-center justify-center p-4">
			<Auth onAuthSuccess={handleAuthSuccess} />
		</div>
	{/if}
</main>
