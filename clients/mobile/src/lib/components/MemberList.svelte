<script lang="ts">
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import ProfileModal from './ProfileModal.svelte';
	import { presenceMap, presenceDotClass, track } from '$lib/presence';

	interface Member {
		user_id: string;
		display_name: string | null;
		avatar_url: string | null;
	}

	interface Props {
		accessToken: string;
		selfUserId: string;
		channelId: string | null;
		apiUrl?: string;
	}

	let { accessToken, selfUserId, channelId, apiUrl = 'http://localhost:3000' }: Props = $props();

	let members = $state<Member[]>([]);
	let loading = $state(false);
	let profileTarget = $state<string | null>(null);

	// cleanup functions for tracked user IDs — called when members change or component destroys
	let untrackers: Array<() => void> = [];

	function stopTracking() {
		for (const fn of untrackers) fn();
		untrackers = [];
	}

	function startTracking(list: Member[]) {
		stopTracking();
		untrackers = list.map((m) => track(m.user_id));
	}

	async function loadMembers() {
		if (!channelId) {
			stopTracking();
			members = [];
			return;
		}
		try {
			loading = true;
			const params = new URLSearchParams({ access_token: accessToken, room_id: channelId });
			const response = await fetch(`${apiUrl}/rooms/members?${params}`);
			if (response.ok) {
				const data = await response.json();
				members = data.members || [];
				startTracking(members);
			}
		} catch (e) {
			console.error('failed to load members:', e);
		} finally {
			loading = false;
		}
	}

	// reload when channel changes
	$effect(() => {
		const _id = channelId;
		loadMembers();
	});

	// clean up trackers when component is destroyed
	$effect(() => {
		return () => stopTracking();
	});

	function getDisplayName(member: Member): string {
		return member.display_name || member.user_id.split(':')[0].replace('@', '');
	}

	function getInitial(member: Member): string {
		return (getDisplayName(member)[0] || '?').toUpperCase();
	}

	// mirror the shared store into a local reactive state so Svelte 5 runes
	// re-render when it changes (stores are svelte 4 style, need explicit subscribe)
	let presence = $state<Record<string, string>>({});
	$effect(() => {
		const unsub = presenceMap.subscribe((val) => { presence = val; });
		return unsub;
	});

	// group members: online first, then offline
	let sortedMembers = $derived(
		[...members].sort((a, b) => {
			const pa = presence[a.user_id] ?? 'offline';
			const pb = presence[b.user_id] ?? 'offline';
			if (pa === pb) return 0;
			if (pa === 'online') return -1;
			if (pb === 'online') return 1;
			return 0;
		})
	);
</script>

<div class="flex flex-col h-full w-56 bg-card border-l border-border">
	<!-- header -->
	<div class="p-4 border-b border-border">
		<h2 class="font-semibold text-sm text-muted-foreground uppercase tracking-wide">
			members — {members.length}
		</h2>
	</div>

	<!-- member list -->
	<ScrollArea class="flex-1 p-2">
		{#if loading}
			<div class="text-center text-muted-foreground text-sm py-4">loading...</div>
		{:else if !channelId}
			<div class="text-center text-muted-foreground text-sm py-4">select a channel</div>
		{:else if members.length === 0}
			<div class="text-center text-muted-foreground text-sm py-4">no members</div>
		{:else}
			<div class="space-y-1">
				{#each sortedMembers as member (member.user_id)}
					<button
						class="w-full flex items-center gap-3 px-2 py-1.5 rounded hover:bg-muted transition-colors text-left"
						onclick={() => profileTarget = member.user_id}
						type="button"
					>
						<!-- avatar -->
						<div class="relative flex-shrink-0">
							<div class="w-8 h-8 rounded-full bg-primary flex items-center justify-center text-primary-foreground text-sm font-semibold">
								{getInitial(member)}
							</div>
							<span class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full {presenceDotClass(presence[member.user_id] ?? 'offline')} ring-2 ring-card block"></span>
						</div>
						<!-- name -->
						<div class="min-w-0 flex-1">
							<p class="text-sm text-card-foreground truncate">{getDisplayName(member)}</p>
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</ScrollArea>
</div>

{#if profileTarget}
	<ProfileModal
		targetUserId={profileTarget}
		selfUserId={selfUserId}
		{accessToken}
		onClose={() => profileTarget = null}
	/>
{/if}
