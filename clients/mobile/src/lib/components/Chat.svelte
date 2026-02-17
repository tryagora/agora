<script lang="ts">
	import ServerList from './ServerList.svelte';
	import ChannelList from './ChannelList.svelte';
	import MemberList from './MemberList.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';


	interface Message {
		room_id: string;
		sender: string;
		content: string;
		timestamp?: number;
		event_id?: string;
	}

	interface Props {
		userId: string;
		accessToken: string;
		onLogout: () => void;
	}

	let { userId, accessToken, onLogout }: Props = $props();

	let messages = $state<Message[]>([]);
	let newMessage = $state('');
	let selectedServerId = $state<string | null>(null);
	let selectedChannelId = $state<string | null>(null);
	let selectedChannelName = $state<string | null>(null);
	let nextBatch = $state('');
	let loading = $state(false);
	let error = $state('');
	let showCreateServerDialog = $state(false);
	let newServerName = $state('');
	let messagesContainer: HTMLDivElement;

	const API_URL = 'http://localhost:3000';

	async function sync() {
		try {
			const params = new URLSearchParams({ access_token: accessToken });
			if (nextBatch) {
				params.append('since', nextBatch);
			}
			
			const response = await fetch(`${API_URL}/sync?${params}`);
			if (response.ok) {
				const data = await response.json();
				nextBatch = data.next_batch;
				if (data.messages && data.messages.length > 0) {
					// deduplicate by event_id
					const existingIds = new Set(messages.map(m => m.event_id).filter(Boolean));
					const newMessages = data.messages.filter(
						(m: Message) => !m.event_id || !existingIds.has(m.event_id)
					);
					if (newMessages.length > 0) {
						messages = [...messages, ...newMessages];
					}
				}
			}
		} catch (e) {
			console.error('sync failed:', e);
		}
	}

	async function sendMessage() {
		if (!newMessage.trim() || !selectedChannelId) return;
		
		loading = true;
		error = '';
		
		try {
			const response = await fetch(`${API_URL}/rooms/send`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: selectedChannelId,
					content: newMessage
				})
			});
			
			if (response.ok) {
				newMessage = '';
				// message will appear on next sync cycle
			} else {
				error = 'failed to send message';
			}
		} catch (e) {
			error = 'failed to send message';
		} finally {
			loading = false;
		}
	}

	async function handleCreateServer() {
		if (!newServerName.trim()) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/create`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					name: newServerName,
					is_space: true
				})
			});
			
			if (response.ok) {
				showCreateServerDialog = false;
				newServerName = '';
			} else {
				error = 'failed to create server';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	function handleSelectServer(serverId: string) {
		selectedServerId = serverId || null;
		selectedChannelId = null;
	}

	function handleSelectChannel(channelId: string, channelName?: string) {
		selectedChannelId = channelId;
		selectedChannelName = channelName || null;
	}

	// poll for new messages every 5 seconds
	$effect(() => {
		const interval = setInterval(sync, 5000);
		sync(); // initial sync
		return () => clearInterval(interval);
	});

	function formatTimestamp(ts?: number): string {
		if (!ts) return '';
		return new Date(ts).toLocaleTimeString();
	}

	// filter messages for selected channel
	let channelMessages = $derived(
		selectedChannelId 
			? messages.filter(m => m.room_id === selectedChannelId)
			: messages
	);

	// auto-scroll to bottom when new messages arrive
	$effect(() => {
		const _len = channelMessages.length;
		if (messagesContainer) {
			requestAnimationFrame(() => {
				messagesContainer.scrollTop = messagesContainer.scrollHeight;
			});
		}
	});
</script>

<div class="flex h-screen w-screen bg-slate-900">
	<!-- server list -->
	<ServerList
		{accessToken}
		{selectedServerId}
		onSelectServer={handleSelectServer}
		onCreateServer={() => showCreateServerDialog = true}
		onJoinServer={() => {}}
	/>

	<!-- channel list -->
	<ChannelList
		{accessToken}
		serverId={selectedServerId}
		{selectedChannelId}
		onSelectChannel={handleSelectChannel}
	/>

	<!-- chat area + member list -->
	<div class="flex-1 flex min-h-0">
	<div class="flex-1 flex flex-col min-h-0">
		<!-- header -->
		<div class="h-14 border-b border-slate-700 flex items-center justify-between px-4 bg-slate-800">
			<div>
			{#if selectedChannelId}
				<span class="text-slate-500">#</span>
				<span class="font-semibold text-white ml-1">{selectedChannelName || 'channel'}</span>
				{:else}
					<span class="text-slate-400">select a channel</span>
				{/if}
			</div>
			<div class="flex items-center gap-4">
				<span class="text-sm text-slate-400">{userId}</span>
				<Button variant="outline" size="sm" onclick={onLogout}>
					logout
				</Button>
			</div>
		</div>

		<!-- messages -->
		<div class="flex-1 overflow-y-auto p-4 space-y-3 min-h-0" bind:this={messagesContainer}>
			{#if channelMessages.length === 0}
				<p class="text-slate-500 text-center mt-8">
					{selectedChannelId ? 'no messages yet' : 'select a channel to view messages'}
				</p>
			{:else}
				{#each channelMessages as message, i (message.event_id || `local-${i}-${message.timestamp}`)}
					<div class="space-y-1">
						<div class="flex items-center gap-2">
							<span class="font-semibold text-sm text-white">{message.sender}</span>
							<span class="text-xs text-slate-500">
								{formatTimestamp(message.timestamp)}
							</span>
						</div>
						<p class="text-sm text-slate-300">{message.content}</p>
					</div>
				{/each}
			{/if}
		</div>

		<!-- message input -->
		<div class="p-4 border-t border-slate-700 bg-slate-800">
			{#if error}
				<div class="text-sm text-red-400 mb-2">{error}</div>
			{/if}
			
			<div class="flex gap-2">
				<Input
					type="text"
					placeholder={selectedChannelId ? "type a message..." : "select a channel first"}
					bind:value={newMessage}
					onkeydown={(e) => e.key === 'Enter' && sendMessage()}
					disabled={loading || !selectedChannelId}
					class="flex-1 bg-slate-700 border-slate-600"
				/>
				<Button 
					onclick={sendMessage}
					disabled={loading || !newMessage.trim() || !selectedChannelId}
				>
					send
				</Button>
			</div>
		</div>
	</div>

	<!-- member list -->
	{#if selectedChannelId}
		<MemberList
			{accessToken}
			channelId={selectedChannelId}
		/>
	{/if}
	</div>

	<!-- create server dialog -->
	{#if showCreateServerDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-slate-800 p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4 text-white">create server</h3>
				{#if error}
					<div class="text-red-400 text-sm mb-3">{error}</div>
				{/if}
				<Input
					type="text"
					placeholder="server name"
					bind:value={newServerName}
					class="mb-4 bg-slate-700 border-slate-600"
				/>
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showCreateServerDialog = false}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleCreateServer} disabled={!newServerName.trim()}>
						create
					</Button>
				</div>
			</div>
		</div>
	{/if}
</div>
