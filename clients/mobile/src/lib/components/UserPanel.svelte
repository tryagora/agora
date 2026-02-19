<script lang="ts">
	import { presenceMap, presenceDotClass, track } from '$lib/presence';

	interface Props {
		userId: string;
		accessToken: string;
		apiUrl?: string;
		onOpenSettings: () => void;
	}

	let { userId, accessToken, apiUrl = 'http://localhost:3000', onOpenSettings }: Props = $props();

	let showStatusMenu = $state(false);
	let displayname = $state<string | null>(null);
	// ghost mode — user appears offline to everyone else but still receives sync
	let ghostMode = $state(false);
	// the presence value to restore when ghost mode is turned off
	let preGhostPresence = $state<'online' | 'unavailable' | 'offline'>('online');

	// live presence from shared store
	let presence = $state('offline');
	$effect(() => {
		const untrack = track(userId);
		const unsub = presenceMap.subscribe((m) => { presence = m[userId] ?? 'offline'; });
		return () => { untrack(); unsub(); };
	});

	// load own displayname once
	$effect(() => {
		fetch(`${apiUrl}/profile/get?access_token=${encodeURIComponent(accessToken)}&user_id=${encodeURIComponent(userId)}`)
			.then(r => r.ok ? r.json() : null)
			.then(d => { if (d) displayname = d.displayname || null; })
			.catch(() => {});
	});

	function shortName(uid: string) {
		return uid.replace(/^@/, '').split(':')[0];
	}

	function initials(uid: string) {
		const n = displayname || shortName(uid);
		return n.slice(0, 2).toUpperCase();
	}

	async function setStatus(p: 'online' | 'unavailable' | 'offline') {
		showStatusMenu = false;
		// turning off ghost mode when user explicitly sets a status
		if (ghostMode) {
			ghostMode = false;
		}
		try {
			await fetch(`${apiUrl}/presence/set`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, user_id: userId, presence: p })
			});
		} catch { /* best-effort */ }
	}

	async function toggleGhost() {
		showStatusMenu = false;
		if (ghostMode) {
			// coming out of ghost — restore previous presence
			ghostMode = false;
			await setStatus(preGhostPresence);
		} else {
			// save current presence so we can restore it later
			const cur = (presence === 'unavailable' ? 'unavailable' : presence === 'online' ? 'online' : 'offline') as 'online' | 'unavailable' | 'offline';
			preGhostPresence = cur;
			ghostMode = true;
			// set presence to offline on the server so everyone else sees us as offline
			try {
				await fetch(`${apiUrl}/presence/set`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ access_token: accessToken, user_id: userId, presence: 'offline' })
				});
			} catch { /* best-effort */ }
		}
	}

	const statusOptions: { label: string; value: 'online' | 'unavailable' | 'offline'; dot: string }[] = [
		{ label: 'online',      value: 'online',      dot: 'bg-green-500' },
		{ label: 'away',        value: 'unavailable', dot: 'bg-yellow-500' },
		{ label: 'offline',     value: 'offline',     dot: 'bg-muted-foreground/50' },
	];
</script>

<!-- close status menu on outside click -->
<svelte:window onclick={(e) => {
	if (showStatusMenu) {
		const el = e.target as HTMLElement;
		if (!el.closest('[data-status-menu]')) showStatusMenu = false;
	}
}} />

<div class="relative h-14 border-t border-border bg-card flex items-center px-2 gap-2 flex-shrink-0">
	<!-- avatar + status dot (click to open status picker) -->
	<div class="relative" data-status-menu>
		<button
			class="w-9 h-9 rounded-full bg-primary flex items-center justify-center text-primary-foreground text-sm font-bold flex-shrink-0 hover:opacity-80 transition-opacity"
			onclick={() => showStatusMenu = !showStatusMenu}
			title="set status"
			type="button"
		>
			{initials(userId)}
		</button>
		{#if ghostMode}
			<!-- ghost badge replaces presence dot when ghosting -->
			<span class="absolute -bottom-0.5 -right-0.5 text-sm leading-none pointer-events-none select-none" title="ghost mode active">👻</span>
		{:else}
			<span class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full {presenceDotClass(presence)} ring-2 ring-card pointer-events-none block"></span>
		{/if}

		<!-- status dropdown -->
		{#if showStatusMenu}
			<div
				class="absolute bottom-12 left-0 bg-card border border-border rounded-lg shadow-xl w-44 py-1 z-50"
				data-status-menu
			>
				<p class="px-3 py-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider">set status</p>
				{#each statusOptions as opt}
					<button
						class="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-card-foreground hover:bg-muted transition-colors text-left"
						class:font-semibold={!ghostMode && (presence === opt.value || (opt.value === 'unavailable' && presence === 'unavailable'))}
						onclick={() => setStatus(opt.value)}
						type="button"
					>
						<span class="w-2.5 h-2.5 rounded-full {opt.dot} flex-shrink-0"></span>
						{opt.label}
						{#if !ghostMode && ((opt.value === presence) || (opt.value === 'unavailable' && presence === 'unavailable'))}
							<svg class="ml-auto h-3.5 w-3.5 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
							</svg>
						{/if}
					</button>
				{/each}
				<!-- divider -->
				<div class="my-1 border-t border-border"></div>
				<!-- ghost mode toggle -->
				<button
					class="w-full flex items-center gap-2.5 px-3 py-2 text-sm hover:bg-muted transition-colors text-left"
					class:font-semibold={ghostMode}
					class:text-purple-400={ghostMode}
					class:text-card-foreground={!ghostMode}
					onclick={toggleGhost}
					type="button"
				>
					<span class="text-base leading-none flex-shrink-0">👻</span>
					<span>ghost mode</span>
					{#if ghostMode}
						<svg class="ml-auto h-3.5 w-3.5 text-purple-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
						</svg>
					{/if}
				</button>
				{#if ghostMode}
					<p class="px-3 pb-2 text-[10px] text-purple-400/70">you appear offline to others</p>
				{/if}
			</div>
		{/if}
	</div>

	<!-- name + status label -->
	<div class="flex-1 min-w-0">
		<p class="text-sm font-semibold text-card-foreground truncate leading-tight">{displayname || shortName(userId)}</p>
		<p class="text-xs truncate leading-tight capitalize" class:text-purple-400={ghostMode} class:text-muted-foreground={!ghostMode}>
			{ghostMode ? '👻 ghost mode' : presence === 'unavailable' ? 'away' : presence}
		</p>
	</div>

	<!-- settings button -->
	<button
		class="w-8 h-8 rounded flex items-center justify-center text-muted-foreground hover:text-card-foreground hover:bg-muted transition-colors flex-shrink-0"
		onclick={onOpenSettings}
		title="settings"
		type="button"
		aria-label="open settings"
	>
		<!-- gear icon -->
		<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
		</svg>
	</button>
</div>
