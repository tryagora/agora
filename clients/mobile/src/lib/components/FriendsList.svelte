<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import ProfileModal from './ProfileModal.svelte';
	import { presenceMap, presenceDotClass, presenceLabel, track } from '$lib/presence';

	interface Friend {
		user_id: string;
		status: 'accepted' | 'pending_sent' | 'pending_received';
		dm_room_id: string | null;
	}

	interface Props {
		accessToken: string;
		userId: string;
		onOpenDm: (roomId: string, friendId: string) => void;
		refreshTrigger?: number;
	}

	let { accessToken, userId, onOpenDm, refreshTrigger = 0 }: Props = $props();

	type Tab = 'all' | 'pending' | 'add';
	let activeTab = $state<Tab>('all');
	let friends = $state<Friend[]>([]);
	let loading = $state(false);
	let error = $state('');
	let addInput = $state('');
	let addSuccess = $state('');
	let openingDm = $state<string | null>(null);
	let profileTarget = $state<string | null>(null);

	const API_URL = 'http://localhost:3000';

	// mirror the shared presence store into local reactive state
	let presence = $state<Record<string, string>>({});
	$effect(() => {
		const unsub = presenceMap.subscribe((val) => { presence = val; });
		return unsub;
	});

	// track/untrack friends as the accepted list changes
	let untrackers: Array<() => void> = [];
	function syncTracking(accepted: { user_id: string }[]) {
		for (const fn of untrackers) fn();
		untrackers = accepted.map((f) => track(f.user_id));
	}
	$effect(() => {
		return () => { for (const fn of untrackers) fn(); };
	});

	async function loadFriends() {
		try {
			loading = true;
			error = '';
			const res = await fetch(
				`${API_URL}/friends?access_token=${encodeURIComponent(accessToken)}&user_id=${encodeURIComponent(userId)}`
			);
			if (res.ok) {
				const data = await res.json();
				friends = data.friends;
				// register accepted friends with the shared presence store
				syncTracking(data.friends.filter((f: Friend) => f.status === 'accepted'));
			} else {
				error = 'failed to load friends';
			}
		} catch {
			error = 'network error';
		} finally {
			loading = false;
		}
	}

	function presenceDot(uid: string): string {
		return presenceDotClass(presence[uid] ?? 'offline');
	}

	function friendPresenceLabel(uid: string): string {
		return presenceLabel(presence[uid] ?? 'offline');
	}

	async function sendFriendRequest() {
		const target = addInput.trim();
		if (!target) return;

		// normalise to full matrix id
		const friendId = target.includes(':') ? target : `@${target}:localhost`;

		addSuccess = '';
		error = '';

		try {
			const res = await fetch(`${API_URL}/friends/add`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, user_id: userId, friend_id: friendId })
			});
			if (res.ok) {
				addInput = '';
				addSuccess = `friend request sent to ${friendId}`;
				await loadFriends();
			} else {
				error = 'failed to send friend request';
			}
		} catch {
			error = 'network error';
		}
	}

	async function acceptRequest(friendId: string) {
		try {
			const res = await fetch(`${API_URL}/friends/accept`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, user_id: userId, friend_id: friendId })
			});
			if (res.ok) await loadFriends();
			else error = 'failed to accept request';
		} catch {
			error = 'network error';
		}
	}

	async function rejectRequest(friendId: string) {
		try {
			const res = await fetch(`${API_URL}/friends/reject`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, user_id: userId, friend_id: friendId })
			});
			if (res.ok) await loadFriends();
			else error = 'failed to reject request';
		} catch {
			error = 'network error';
		}
	}

	async function removeFriend(friendId: string) {
		try {
			const res = await fetch(`${API_URL}/friends/remove`, {
				method: 'DELETE',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, user_id: userId, friend_id: friendId })
			});
			if (res.ok) await loadFriends();
			else error = 'failed to remove friend';
		} catch {
			error = 'network error';
		}
	}

	async function openDm(friend: Friend) {
		openingDm = friend.user_id;
		error = '';
		try {
			const res = await fetch(`${API_URL}/friends/dm`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					user_id: userId,
					friend_id: friend.user_id
				})
			});
			if (res.ok) {
				const data = await res.json();
				// refresh so the cached room_id is reflected
				await loadFriends();
				onOpenDm(data.room_id, friend.user_id);
			} else {
				error = 'failed to open dm';
			}
		} catch {
			error = 'network error';
		} finally {
			openingDm = null;
		}
	}

	// derived lists
	let acceptedFriends = $derived(friends.filter((f) => f.status === 'accepted'));
	let pendingReceived = $derived(friends.filter((f) => f.status === 'pending_received'));
	let pendingSent = $derived(friends.filter((f) => f.status === 'pending_sent'));
	let pendingCount = $derived(pendingReceived.length);

	function shortId(userId: string) {
		// strip @....: prefix for display
		return userId.replace(/^@/, '').split(':')[0];
	}

	function initials(userId: string) {
		const name = shortId(userId);
		return name.slice(0, 2).toUpperCase();
	}

	$effect(() => {
		const _t = refreshTrigger;
		loadFriends();
	});


</script>

<div class="flex flex-col h-full min-w-0">
	<!-- tab bar -->
	<div class="flex items-center gap-1 px-4 py-3 border-b border-border bg-card">
		<button
			class="px-3 py-1.5 rounded text-sm font-medium transition-colors"
			class:bg-primary={activeTab === 'all'}
			class:text-primary-foreground={activeTab === 'all'}
			class:text-muted-foreground={activeTab !== 'all'}
			class:hover:bg-muted={activeTab !== 'all'}
			onclick={() => activeTab = 'all'}
		>
			all friends
		</button>
		<button
			class="px-3 py-1.5 rounded text-sm font-medium transition-colors flex items-center gap-1.5"
			class:bg-primary={activeTab === 'pending'}
			class:text-primary-foreground={activeTab === 'pending'}
			class:text-muted-foreground={activeTab !== 'pending'}
			class:hover:bg-muted={activeTab !== 'pending'}
			onclick={() => activeTab = 'pending'}
		>
			pending
			{#if pendingCount > 0}
				<span class="bg-destructive text-destructive-foreground text-xs rounded-full w-4 h-4 flex items-center justify-center leading-none">
					{pendingCount}
				</span>
			{/if}
		</button>
		<button
			class="px-3 py-1.5 rounded text-sm font-medium transition-colors"
			class:bg-primary={activeTab === 'add'}
			class:text-primary-foreground={activeTab === 'add'}
			class:text-muted-foreground={activeTab !== 'add'}
			class:hover:bg-muted={activeTab !== 'add'}
			onclick={() => { activeTab = 'add'; addSuccess = ''; error = ''; }}
		>
			add friend
		</button>
	</div>

	{#if error}
		<div class="mx-4 mt-2 text-sm text-destructive">{error}</div>
	{/if}

	<!-- all friends tab -->
	{#if activeTab === 'all'}
		<ScrollArea class="flex-1 p-4">
			{#if loading}
				<p class="text-muted-foreground text-sm text-center py-8">loading...</p>
			{:else if acceptedFriends.length === 0}
				<div class="text-center py-12">
					<p class="text-muted-foreground text-sm">no friends yet</p>
					<p class="text-muted-foreground text-xs mt-1">add someone to get started</p>
				</div>
			{:else}
				<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">
					friends — {acceptedFriends.length}
				</p>
				<div class="space-y-1">
					{#each acceptedFriends as friend (friend.user_id)}
						<div class="flex items-center gap-3 px-3 py-2 rounded hover:bg-muted group transition-colors">
							<!-- avatar (clickable for profile) -->
							<button
								class="relative flex-shrink-0"
								onclick={() => profileTarget = friend.user_id}
								title="view profile"
								type="button"
							>
								<div class="w-9 h-9 rounded-full bg-secondary flex items-center justify-center text-secondary-foreground text-sm font-bold">
									{initials(friend.user_id)}
								</div>
								<span class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full {presenceDot(friend.user_id)} ring-2 ring-card block"></span>
							</button>
							<!-- name + presence label -->
							<div class="flex-1 min-w-0">
								<p class="text-sm font-medium text-card-foreground truncate">{shortId(friend.user_id)}</p>
								<p class="text-xs text-muted-foreground truncate">{friendPresenceLabel(friend.user_id)}</p>
							</div>
							<!-- actions (shown on hover) -->
							<div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
								<button
									class="w-8 h-8 rounded-full bg-muted hover:bg-primary hover:text-primary-foreground flex items-center justify-center transition-colors"
									title="message"
									onclick={() => openDm(friend)}
									disabled={openingDm === friend.user_id}
								>
									{#if openingDm === friend.user_id}
										<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
											<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
											<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8z"></path>
										</svg>
									{:else}
										<!-- chat bubble icon -->
										<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
										</svg>
									{/if}
								</button>
								<button
									class="w-8 h-8 rounded-full bg-muted hover:bg-destructive hover:text-destructive-foreground flex items-center justify-center transition-colors"
									title="remove friend"
									onclick={() => removeFriend(friend.user_id)}
								>
									<!-- x icon -->
									<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</ScrollArea>
	{/if}

	<!-- pending tab -->
	{#if activeTab === 'pending'}
		<ScrollArea class="flex-1 p-4">
			{#if pendingReceived.length === 0 && pendingSent.length === 0}
				<p class="text-muted-foreground text-sm text-center py-8">no pending requests</p>
			{/if}

			{#if pendingReceived.length > 0}
				<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">
					incoming — {pendingReceived.length}
				</p>
				<div class="space-y-1 mb-6">
					{#each pendingReceived as friend (friend.user_id)}
						<div class="flex items-center gap-3 px-3 py-2 rounded hover:bg-muted transition-colors">
							<div class="w-9 h-9 rounded-full bg-secondary flex items-center justify-center text-secondary-foreground text-sm font-bold flex-shrink-0">
								{initials(friend.user_id)}
							</div>
							<div class="flex-1 min-w-0">
								<p class="text-sm font-medium text-card-foreground truncate">{shortId(friend.user_id)}</p>
								<p class="text-xs text-muted-foreground">incoming friend request</p>
							</div>
							<div class="flex items-center gap-1">
								<button
									class="w-8 h-8 rounded-full bg-muted hover:bg-primary hover:text-primary-foreground flex items-center justify-center transition-colors"
									title="accept"
									onclick={() => acceptRequest(friend.user_id)}
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
									</svg>
								</button>
								<button
									class="w-8 h-8 rounded-full bg-muted hover:bg-destructive hover:text-destructive-foreground flex items-center justify-center transition-colors"
									title="decline"
									onclick={() => rejectRequest(friend.user_id)}
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							</div>
						</div>
					{/each}
				</div>
			{/if}

			{#if pendingSent.length > 0}
				<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">
					sent — {pendingSent.length}
				</p>
				<div class="space-y-1">
					{#each pendingSent as friend (friend.user_id)}
						<div class="flex items-center gap-3 px-3 py-2 rounded hover:bg-muted transition-colors">
							<div class="w-9 h-9 rounded-full bg-secondary flex items-center justify-center text-secondary-foreground text-sm font-bold flex-shrink-0">
								{initials(friend.user_id)}
							</div>
							<div class="flex-1 min-w-0">
								<p class="text-sm font-medium text-card-foreground truncate">{shortId(friend.user_id)}</p>
								<p class="text-xs text-muted-foreground">friend request pending</p>
							</div>
							<button
								class="w-8 h-8 rounded-full bg-muted hover:bg-destructive hover:text-destructive-foreground flex items-center justify-center transition-colors"
								title="cancel request"
								onclick={() => rejectRequest(friend.user_id)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
								</svg>
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</ScrollArea>
	{/if}

	<!-- add friend tab -->
	{#if activeTab === 'add'}
		<div class="flex-1 p-6">
			<h3 class="text-base font-semibold text-card-foreground mb-1">add a friend</h3>
			<p class="text-sm text-muted-foreground mb-4">
				you can add friends using their username. it's case sensitive.
			</p>
			<div class="flex gap-2">
				<Input
					type="text"
					placeholder="@username or @username:server"
					bind:value={addInput}
					onkeydown={(e) => e.key === 'Enter' && sendFriendRequest()}
					class="flex-1 bg-muted border-input"
				/>
				<Button onclick={sendFriendRequest} disabled={!addInput.trim()}>
					send request
				</Button>
			</div>
			{#if addSuccess}
				<p class="text-sm text-primary mt-3">{addSuccess}</p>
			{/if}
		</div>
	{/if}
</div>

{#if profileTarget}
	<ProfileModal
		targetUserId={profileTarget}
		selfUserId={userId}
		{accessToken}
		onClose={() => profileTarget = null}
	/>
{/if}
