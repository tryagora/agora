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
		onSelectServer: (serverId: string, serverName?: string) => void;
		onCreateServer: () => void;
		onJoinServer: () => void;
		onManageServer: () => void;
		onLeaveServer?: () => void;
		refreshTrigger?: number;
	}

	let { accessToken, selectedServerId, onSelectServer, onCreateServer, onJoinServer, onManageServer, onLeaveServer, refreshTrigger = 0 }: Props = $props();

	let servers = $state<Server[]>([]);
	let loading = $state(true);
	let error = $state('');
	let showJoinDialog = $state(false);
	let joinRoomId = $state('');
	let showLeaveConfirm = $state(false);

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

	// load servers on mount and when refreshTrigger changes
	$effect(() => {
		const _trigger = refreshTrigger;
		loadServers();
	});

	function getServerInitials(name: string | null): string {
		if (!name) return '?';
		return name.split(' ').map(w => w[0]).join('').toUpperCase().slice(0, 2);
	}

	async function handleLeaveServer() {
		if (!selectedServerId) return;
		
		try {
			// backend handles leaving all child rooms before leaving the space itself
			const response = await fetch(`${API_URL}/rooms/leave`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: selectedServerId
				})
			});
			
			if (response.ok) {
				showLeaveConfirm = false;
				onSelectServer(''); // go back to home
				onLeaveServer?.();
				await loadServers();
			} else {
				error = 'failed to leave server';
			}
		} catch (e) {
			error = 'network error';
		}
	}
</script>

<div class="flex flex-col h-full w-20 bg-background border-r border-border py-3">
	<!-- home button -->
	<button
		class="w-12 h-12 mx-auto rounded-full bg-muted hover:bg-muted/80 flex items-center justify-center mb-3 transition-all"
		class:ring-2={selectedServerId === null}
		class:ring-primary={selectedServerId === null}
		onclick={() => onSelectServer('')}
		aria-label="home"
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-primary-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
		</svg>
	</button>

	<div class="w-8 h-[2px] bg-border mx-auto mb-3 rounded-full"></div>

	<!-- server list -->
	<ScrollArea class="flex-1 px-2">
		<div class="space-y-2">
			{#if loading}
				<div class="text-center text-muted-foreground text-xs">loading...</div>
			{:else if servers.length === 0}
				<div class="text-center text-muted-foreground text-xs">no servers</div>
			{:else}
				{#each servers as server (server.room_id)}
					<div class="flex flex-col items-center gap-1">
						<button
							class="w-12 h-12 rounded-full flex items-center justify-center text-primary-foreground font-bold text-sm transition-all hover:rounded-2xl"
							class:bg-primary={selectedServerId === server.room_id}
							class:bg-secondary={selectedServerId !== server.room_id}
							class:ring-2={selectedServerId === server.room_id}
							class:ring-primary={selectedServerId === server.room_id}
							title={server.name || 'unnamed server'}
							onclick={() => onSelectServer(server.room_id, server.name ?? undefined)}
							aria-label={server.name || 'unnamed server'}
						>
							{getServerInitials(server.name)}
						</button>
						{#if selectedServerId === server.room_id}
							<div class="flex flex-col gap-1">
								<button
									class="w-6 h-6 rounded-full bg-muted hover:bg-accent border border-border flex items-center justify-center transition-colors"
									onclick={() => onManageServer()}
									title="manage server"
									aria-label="manage server"
									type="button"
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
									</svg>
								</button>
								<button
									class="w-6 h-6 rounded-full bg-muted hover:bg-destructive hover:text-destructive-foreground border border-border flex items-center justify-center transition-colors"
									onclick={() => showLeaveConfirm = true}
									title="leave server"
									aria-label="leave server"
									type="button"
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
									</svg>
								</button>
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	</ScrollArea>

	<div class="w-8 h-[2px] bg-border mx-auto my-3 rounded-full"></div>

	<!-- add server button -->
	<button
		class="w-12 h-12 mx-auto rounded-full bg-secondary hover:bg-primary hover:text-primary-foreground text-secondary-foreground flex items-center justify-center transition-all mb-2"
		onclick={onCreateServer}
		title="create server"
	>
		<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
		</svg>
	</button>

	<!-- join server button -->
	<button
		class="w-12 h-12 mx-auto rounded-full bg-secondary hover:bg-primary hover:text-primary-foreground text-secondary-foreground flex items-center justify-center transition-all"
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
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4 text-card-foreground">join server</h3>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
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

	<!-- leave confirmation dialog -->
	{#if showLeaveConfirm}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-2 text-card-foreground">leave server?</h3>
				<p class="text-muted-foreground text-sm mb-4">are you sure you want to leave this server? you can rejoin later if invited.</p>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
				{/if}
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showLeaveConfirm = false}>
						cancel
					</Button>
					<Button variant="destructive" class="flex-1" onclick={handleLeaveServer}>
						leave
					</Button>
				</div>
			</div>
		</div>
	{/if}
</div>
