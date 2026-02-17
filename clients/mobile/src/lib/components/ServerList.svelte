<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';

	interface Server {
		room_id: string;
		name: string | null;
		topic: string | null;
		is_space: boolean;
		member_count: number | null;
	}

	interface Props {
		accessToken: string;
		selectedServerId: string | null;
		onSelectServer: (serverId: string) => void;
		onCreateServer: () => void;
		onJoinServer: () => void;
	}

	let { accessToken, selectedServerId, onSelectServer, onCreateServer, onJoinServer }: Props = $props();

	let servers = $state<Server[]>([]);
	let loading = $state(true);
	let error = $state('');
	let showJoinDialog = $state(false);
	let joinRoomId = $state('');

	const API_URL = 'http://localhost:3000';

	async function loadServers() {
		try {
			loading = true;
			error = '';
			
			const response = await fetch(`${API_URL}/rooms?access_token=${accessToken}`);
			if (response.ok) {
				const data = await response.json();
				servers = data.rooms.filter((r: Server) => r.is_space);
			} else {
				error = 'failed to load servers';
			}
		} catch (e) {
			error = 'network error';
		} finally {
			loading = false;
		}
	}

	async function handleJoinServer() {
		if (!joinRoomId.trim()) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/join`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id_or_alias: joinRoomId
				})
			});
			
			if (response.ok) {
				showJoinDialog = false;
				joinRoomId = '';
				await loadServers();
			} else {
				error = 'failed to join server';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	// load servers on mount
	$effect(() => {
		loadServers();
	});

	function getServerInitials(name: string | null): string {
		if (!name) return '?';
		return name.split(' ').map(w => w[0]).join('').toUpperCase().slice(0, 2);
	}
</script>

<div class="flex flex-col h-full w-20 bg-slate-900 border-r border-slate-800 py-3">
	<!-- home button -->
	<button
		class="w-12 h-12 mx-auto rounded-full bg-slate-700 hover:bg-slate-600 flex items-center justify-center mb-3 transition-all"
		class:ring-2={selectedServerId === null}
		class:ring-indigo-500={selectedServerId === null}
		onclick={() => onSelectServer('')}
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
		</svg>
	</button>

	<div class="w-8 h-[2px] bg-slate-700 mx-auto mb-3 rounded-full"></div>

	<!-- server list -->
	<ScrollArea class="flex-1 px-2">
		<div class="space-y-2">
			{#if loading}
				<div class="text-center text-slate-500 text-xs">loading...</div>
			{:else if servers.length === 0}
				<div class="text-center text-slate-500 text-xs">no servers</div>
			{:else}
				{#each servers as server (server.room_id)}
					<button
						class="w-12 h-12 mx-auto rounded-full flex items-center justify-center text-white font-bold text-sm transition-all hover:rounded-2xl"
						class:bg-indigo-600={selectedServerId === server.room_id}
						class:bg-slate-700={selectedServerId !== server.room_id}
						class:ring-2={selectedServerId === server.room_id}
						class:ring-white={selectedServerId === server.room_id}
						title={server.name || 'unnamed server'}
						onclick={() => onSelectServer(server.room_id)}
					>
						{getServerInitials(server.name)}
					</button>
				{/each}
			{/if}
		</div>
	</ScrollArea>

	<div class="w-8 h-[2px] bg-slate-700 mx-auto my-3 rounded-full"></div>

	<!-- add server button -->
	<button
		class="w-12 h-12 mx-auto rounded-full bg-slate-800 hover:bg-green-600 hover:text-white text-green-500 flex items-center justify-center transition-all mb-2"
		onclick={onCreateServer}
		title="create server"
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
		</svg>
	</button>

	<!-- join server button -->
	<button
		class="w-12 h-12 mx-auto rounded-full bg-slate-800 hover:bg-indigo-600 hover:text-white text-indigo-400 flex items-center justify-center transition-all"
		onclick={() => showJoinDialog = true}
		title="join server"
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
		</svg>
	</button>

	<!-- join dialog -->
	{#if showJoinDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-slate-800 p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4">join server</h3>
				{#if error}
					<div class="text-red-400 text-sm mb-3">{error}</div>
				{/if}
				<Input
					type="text"
					placeholder="room id or alias"
					bind:value={joinRoomId}
					class="mb-4"
				/>
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showJoinDialog = false}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleJoinServer} disabled={!joinRoomId.trim()}>
						join
					</Button>
				</div>
			</div>
		</div>
	{/if}
</div>
