<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';
	import UserPanel from './UserPanel.svelte';

	interface DM {
		room_id: string;
		name: string | null;
		other_user: string;
	}

	interface Props {
		accessToken: string;
		selectedDmId: string | null;
		onSelectDm: (dmId: string, dmName?: string) => void;
		onCreateDm: () => void;
		onShowFriends?: () => void;
		refreshTrigger?: number;
		userId?: string;
		apiUrl?: string;
		onOpenSettings?: () => void;
	}

	let { accessToken, selectedDmId, onSelectDm, onCreateDm, onShowFriends, refreshTrigger = 0, userId = '', apiUrl = 'http://localhost:3000', onOpenSettings }: Props = $props();

	let dms = $state<DM[]>([]);
	let loading = $state(false);

	const API_URL = 'http://localhost:3000';

	async function loadDms() {
		try {
			loading = true;
			
			const response = await fetch(`${API_URL}/rooms?access_token=${accessToken}`);
			if (response.ok) {
				const data = await response.json();
				// filter for DM rooms (non-space rooms with 2 members typically)
				// for now, show all non-space rooms as potential DMs
				dms = data.rooms
					.filter((r: any) => !r.is_space)
					.map((r: any) => ({
						room_id: r.room_id,
						name: r.name,
						other_user: r.name || 'unknown'
					}));
			}
		} catch (e) {
			console.error('failed to load dms:', e);
		} finally {
			loading = false;
		}
	}

	function dmDisplayName(dm: DM): string {
		if (dm.name) return dm.name;
		// fall back to a short slice of the room_id
		return dm.room_id.split(':')[0].replace('!', '').slice(0, 8);
	}

	// load on mount and when refreshTrigger changes
	$effect(() => {
		const _trigger = refreshTrigger;
		loadDms();
	});
</script>

<div class="flex flex-col h-full w-60 bg-card border-r border-border">
	<!-- friends button at top -->
	<button
		class="flex items-center gap-2 px-4 py-3 border-b border-border hover:bg-muted transition-colors w-full text-left"
		onclick={onShowFriends}
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
		</svg>
		<span class="text-sm font-semibold text-card-foreground">friends</span>
	</button>

	<!-- dm list header -->
	<div class="px-4 pt-3 pb-1">
		<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">direct messages</p>
	</div>

	<!-- dm list -->
	<ScrollArea class="flex-1 p-2">
		{#if loading}
			<div class="text-center text-muted-foreground text-sm py-4">loading...</div>
		{:else if dms.length === 0}
			<div class="text-center text-muted-foreground text-sm py-4">no direct messages</div>
		{:else}
			<div class="space-y-1">
				{#each dms as dm (dm.room_id)}
					<button
						class="w-full text-left px-3 py-2 rounded text-muted-foreground hover:bg-muted hover:text-card-foreground transition-colors flex items-center gap-2"
						class:bg-muted={selectedDmId === dm.room_id}
						class:text-card-foreground={selectedDmId === dm.room_id}
						onclick={() => onSelectDm(dm.room_id, dmDisplayName(dm))}
					>
						<span class="w-2 h-2 rounded-full bg-primary flex-shrink-0"></span>
						<span class="truncate">{dmDisplayName(dm)}</span>
					</button>
				{/each}
			</div>
		{/if}
	</ScrollArea>

	<!-- new dm button -->
	<div class="p-3 border-t border-border">
		<Button 
			variant="outline" 
			class="w-full"
			onclick={onCreateDm}
		>
			+ new message
		</Button>
	</div>

	<!-- user panel at bottom -->
	{#if userId}
		<UserPanel
			{userId}
			{accessToken}
			{apiUrl}
			onOpenSettings={onOpenSettings || (() => {})}
		/>
	{/if}
</div>
