<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { presenceMap, presenceLabel as getPresenceLabel, track } from '$lib/presence';

	interface Props {
		/** matrix user_id of the profile to show */
		targetUserId: string;
		/** our own user_id — used to decide if edit controls are shown */
		selfUserId: string;
		accessToken: string;
		onClose: () => void;
	}

	let { targetUserId, selfUserId, accessToken, onClose }: Props = $props();

	const API_URL = 'http://localhost:3000';

	let displayname = $state<string | null>(null);
	let avatarUrl = $state<string | null>(null);
	let statusMsg = $state<string | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let success = $state('');

	// live presence — subscribe to the shared store and track this user
	let presenceValue = $state('offline');
	$effect(() => {
		const untrack = track(targetUserId);
		const unsub = presenceMap.subscribe((m) => {
			presenceValue = m[targetUserId] ?? 'offline';
		});
		return () => { untrack(); unsub(); };
	});

	// edit state — only used when viewing own profile
	let editName = $state('');
	let editStatusMsg = $state('');
	let isEditing = $state(false);

	const isSelf = $derived(targetUserId === selfUserId);

	function shortId(uid: string) {
		return uid.replace(/^@/, '').split(':')[0];
	}

	function initials(uid: string) {
		const name = displayname || shortId(uid);
		return name.slice(0, 2).toUpperCase();
	}

	function presenceColor(p: string) {
		if (p === 'online') return 'bg-green-500';
		if (p === 'unavailable') return 'bg-yellow-500';
		return 'bg-muted-foreground';
	}

	async function load() {
		loading = true;
		error = '';
		try {
			// only fetch profile — presence comes from the shared store
			const profileRes = await fetch(
				`${API_URL}/profile/get?access_token=${encodeURIComponent(accessToken)}&user_id=${encodeURIComponent(targetUserId)}`
			);
			if (profileRes.ok) {
				const p = await profileRes.json();
				displayname = p.displayname || null;
				avatarUrl = p.avatar_url || null;
			}
			// pre-fill edit fields for own profile
			editName = displayname || shortId(targetUserId);
			editStatusMsg = statusMsg || '';
		} catch {
			error = 'failed to load profile';
		} finally {
			loading = false;
		}
	}

	async function saveProfile() {
		saving = true;
		error = '';
		success = '';
		try {
			// update displayname
			const res = await fetch(`${API_URL}/profile/set`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					user_id: selfUserId,
					displayname: editName.trim() || null
				})
			});

			// update status message via presence
			await fetch(`${API_URL}/presence/set`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					user_id: selfUserId,
					presence: 'online',
					status_msg: editStatusMsg.trim() || null
				})
			});

			if (res.ok) {
				displayname = editName.trim() || null;
				statusMsg = editStatusMsg.trim() || null;
				success = 'profile updated';
				isEditing = false;
			} else {
				error = 'failed to update profile';
			}
		} catch {
			error = 'network error';
		} finally {
			saving = false;
		}
	}

	// close on escape
	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	$effect(() => {
		load();
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- backdrop -->
<div
	class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
	role="dialog"
	aria-modal="true"
	onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
>
	<!-- panel -->
	<div
		class="bg-card rounded-lg w-80 shadow-xl overflow-hidden"
		onclick={(e) => e.stopPropagation()}
		role="presentation"
	>
		{#if loading}
			<div class="p-8 text-center text-muted-foreground text-sm">loading...</div>
		{:else}
			<!-- banner + avatar -->
			<div class="h-20 bg-secondary relative">
				<!-- close button -->
				<button
					class="absolute top-2 right-2 w-7 h-7 rounded-full bg-black/30 hover:bg-black/50 flex items-center justify-center transition-colors"
					onclick={onClose}
					aria-label="close"
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>

				<!-- avatar overlapping the banner -->
				<div class="absolute -bottom-8 left-4">
					<div class="relative">
						<div class="w-16 h-16 rounded-full bg-primary flex items-center justify-center text-primary-foreground text-xl font-bold ring-4 ring-card">
							{initials(targetUserId)}
						</div>
						<!-- presence dot — live from shared store -->
						<span class="absolute bottom-0.5 right-0.5 w-4 h-4 rounded-full {presenceColor(presenceValue)} ring-2 ring-card block"></span>
					</div>
				</div>
			</div>

			<!-- body -->
			<div class="pt-10 px-4 pb-4">
				<!-- name + id -->
				<div class="mb-3">
					<p class="text-base font-bold text-card-foreground">
						{displayname || shortId(targetUserId)}
					</p>
					<p class="text-xs text-muted-foreground">{targetUserId}</p>
					<p class="text-xs text-muted-foreground mt-0.5">
						{getPresenceLabel(presenceValue)}{statusMsg ? ` — ${statusMsg}` : ''}
					</p>
				</div>

				{#if error}
					<p class="text-xs text-destructive mb-2">{error}</p>
				{/if}
				{#if success}
					<p class="text-xs text-primary mb-2">{success}</p>
				{/if}

				{#if isSelf}
					<!-- own profile: show edit form -->
					{#if isEditing}
						<div class="space-y-2 border-t border-border pt-3">
							<div>
								<label class="text-xs text-muted-foreground block mb-1" for="edit-displayname">display name</label>
								<Input
									id="edit-displayname"
									bind:value={editName}
									class="bg-muted border-input text-sm"
									placeholder="your display name"
								/>
							</div>
							<div>
								<label class="text-xs text-muted-foreground block mb-1" for="edit-status">status message</label>
								<Input
									id="edit-status"
									bind:value={editStatusMsg}
									class="bg-muted border-input text-sm"
									placeholder="what are you up to?"
								/>
							</div>
							<div class="flex gap-2 pt-1">
								<Button variant="outline" class="flex-1 text-xs" onclick={() => isEditing = false}>
									cancel
								</Button>
								<Button class="flex-1 text-xs" onclick={saveProfile} disabled={saving}>
									{saving ? 'saving...' : 'save'}
								</Button>
							</div>
						</div>
					{:else}
						<Button variant="outline" class="w-full text-xs mt-1" onclick={() => isEditing = true}>
							edit profile
						</Button>
					{/if}
				{/if}
			</div>
		{/if}
	</div>
</div>
