<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';

	interface Channel {
		room_id: string;
		name: string | null;
		topic: string | null;
		is_space: boolean;
	}

	interface Props {
		accessToken: string;
		serverId: string | null;
		selectedChannelId: string | null;
		onSelectChannel: (channelId: string, channelName?: string) => void;
	}

	let { accessToken, serverId, selectedChannelId, onSelectChannel }: Props = $props();

	let channels = $state<Channel[]>([]);
	let loading = $state(false);
	let error = $state('');
	let showCreateDialog = $state(false);
	let newChannelName = $state('');

	const API_URL = 'http://localhost:3000';

	async function loadChannels() {
		if (!serverId) {
			channels = [];
			return;
		}

		try {
			loading = true;
			error = '';
			
			// fetch children of the selected space/server
			const params = new URLSearchParams({
				access_token: accessToken,
				space_id: serverId
			});
			const response = await fetch(`${API_URL}/rooms/children?${params}`);
			if (response.ok) {
				const data = await response.json();
				channels = data.children || [];
			} else {
				error = 'failed to load channels';
			}
		} catch (e) {
			error = 'network error';
		} finally {
			loading = false;
		}
	}

	async function handleCreateChannel() {
		if (!newChannelName.trim() || !serverId) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/create`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					name: newChannelName,
					is_space: false,
					parent_space_id: serverId
				})
			});
			
			if (response.ok) {
				showCreateDialog = false;
				newChannelName = '';
				await loadChannels();
			} else {
				error = 'failed to create channel';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	// load channels when server changes
	$effect(() => {
		// track serverId so effect re-runs when it changes
		const _id = serverId;
		loadChannels();
	});

	function getChannelIcon(): string {
		return '#';
	}
</script>

<div class="flex flex-col h-full w-60 bg-slate-800 border-r border-slate-700">
	<!-- header -->
	<div class="p-4 border-b border-slate-700">
		<h2 class="font-semibold text-white">
			{serverId ? 'channels' : 'dms'}
		</h2>
	</div>

	<!-- channel list -->
	<ScrollArea class="flex-1 p-2">
		{#if loading}
			<div class="text-center text-slate-500 text-sm py-4">loading...</div>
		{:else if channels.length === 0}
			<div class="text-center text-slate-500 text-sm py-4">
				{serverId ? 'no channels' : 'select a server'}
			</div>
		{:else}
			<div class="space-y-1">
				{#each channels as channel (channel.room_id)}
					<button
						class="w-full text-left px-3 py-2 rounded text-slate-300 hover:bg-slate-700 hover:text-white transition-colors flex items-center gap-2"
						class:bg-slate-700={selectedChannelId === channel.room_id}
						class:text-white={selectedChannelId === channel.room_id}
						onclick={() => onSelectChannel(channel.room_id, channel.name || undefined)}
					>
						<span class="text-slate-500">{getChannelIcon()}</span>
						<span class="truncate">{channel.name || 'unnamed'}</span>
					</button>
				{/each}
			</div>
		{/if}
	</ScrollArea>

	<!-- create channel button (only show if in a server) -->
	{#if serverId}
		<div class="p-3 border-t border-slate-700">
			<Button 
				variant="outline" 
				class="w-full"
				onclick={() => showCreateDialog = true}
			>
				+ create channel
			</Button>
		</div>
	{/if}

	<!-- create channel dialog -->
	{#if showCreateDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-slate-800 p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4">create channel</h3>
				{#if error}
					<div class="text-red-400 text-sm mb-3">{error}</div>
				{/if}
				<Input
					type="text"
					placeholder="channel name"
					bind:value={newChannelName}
					class="mb-4"
				/>
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showCreateDialog = false}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleCreateChannel} disabled={!newChannelName.trim()}>
						create
					</Button>
				</div>
			</div>
		</div>
	{/if}
</div>
