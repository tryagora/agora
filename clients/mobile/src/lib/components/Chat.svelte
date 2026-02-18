<script lang="ts">
	import ServerList from './ServerList.svelte';
	import ChannelList from './ChannelList.svelte';
	import DmList from './DmList.svelte';
	import FriendsList from './FriendsList.svelte';
	import MemberList from './MemberList.svelte';
	import ServerManage from './ServerManage.svelte';
	import SettingsModal from './SettingsModal.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import {
		isPermissionGranted,
		requestPermission,
		sendNotification
	} from '@tauri-apps/plugin-notification';


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
		apiUrl?: string;
		onLogout: () => void;
	}

	let { userId, accessToken, apiUrl = 'http://localhost:3000', onLogout }: Props = $props();

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
	
	// DM state
	let showCreateDmDialog = $state(false);
	let dmUserId = $state('');
	// start in home/DM mode so the friends page shows immediately on login
	let isDmMode = $state(true);
	
	// Server management state
	let showServerManage = $state(false);
	let isAdmin = $state(false);
	let selectedServerName = $state<string | null>(null);
	
	// DM refresh trigger
	let dmRefreshTrigger = $state(0);
	
	// Server refresh trigger
	let serverRefreshTrigger = $state(0);

	// settings modal
	let showSettings = $state(false);

	// friends page — shown in main area when in DM mode with no channel selected
	let showFriendsPage = $state(true);

	// use the server url configured during onboarding (or default)
	const API_URL = $derived(apiUrl);

	// tracks event_ids we have already seen — used to detect genuinely new messages
	const seenEventIds = new Set<string>();
	// set to true after the first sync completes so we don't spam notifications
	// for every historical message that loads on startup
	let initialSyncDone = $state(false);

	async function maybeNotify(msg: Message) {
		// never notify for your own messages
		if (msg.sender === userId) return;

		const senderShort = msg.sender.replace(/^@/, '').split(':')[0];
		const body = msg.content.length > 100 ? msg.content.slice(0, 97) + '...' : msg.content;

		try {
			// try tauri notification plugin first (works in desktop builds)
			let granted = await isPermissionGranted();
			if (!granted) {
				const perm = await requestPermission();
				granted = perm === 'granted';
			}
			if (granted) {
				sendNotification({ title: senderShort, body });
				return;
			}
		} catch {
			// not running inside tauri — fall through to web notifications
		}

		// web notifications fallback (browser / dev mode)
		if (!document.hidden) return; // only notify when window is hidden in browser
		if (!('Notification' in window) || Notification.permission !== 'granted') return;
		try {
			new Notification(senderShort, { body, tag: msg.event_id || undefined });
		} catch {
			// non-fatal
		}
	}

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
					const newMessages = data.messages.filter(
						(m: Message) => !m.event_id || !seenEventIds.has(m.event_id)
					);
					if (newMessages.length > 0) {
						// record all new ids before appending
						for (const m of newMessages) {
							if (m.event_id) seenEventIds.add(m.event_id);
						}
						messages = [...messages, ...newMessages];
						// fire notifications only after the initial history load
						if (initialSyncDone) {
							for (const m of newMessages) {
								await maybeNotify(m);
							}
						}
					}
				}
				// mark first sync complete so subsequent syncs can fire notifications
				initialSyncDone = true;
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
				serverRefreshTrigger++; // trigger server list reload
			} else {
				error = 'failed to create server';
			}
		} catch (e) {
			error = 'network error';
		}
	}



	function handleSelectChannel(channelId: string, channelName?: string) {
		selectedChannelId = channelId;
		selectedChannelName = channelName || null;
		if (!selectedServerId) {
			// selecting a dm — stay in DM mode but leave friends page
			isDmMode = true;
			showFriendsPage = false;
		} else {
			isDmMode = false;
		}
	}

	async function handleSelectServer(serverId: string, serverName?: string) {
		selectedServerId = serverId || null;
		selectedServerName = serverName || null;
		selectedChannelId = null;
		isDmMode = !serverId; // DM mode when no server selected
		showFriendsPage = !serverId; // show friends page when going home
		
		// Check admin status if selecting a server
		if (serverId) {
			await checkAdminStatus(serverId);
		} else {
			isAdmin = false;
		}
	}

	function handleOpenFriendDm(roomId: string, friendId: string) {
		selectedChannelId = roomId;
		// use the short username as the display name
		selectedChannelName = friendId.replace(/^@/, '').split(':')[0];
		isDmMode = true;
		showFriendsPage = false;
		dmRefreshTrigger++; // make DM list reload so the new conversation shows up
	}

	async function handleLeaveServer() {
		selectedServerId = null;
		selectedServerName = null;
		selectedChannelId = null;
		isDmMode = true;
		isAdmin = false;
		showServerManage = false;
		dmRefreshTrigger++; // Trigger DM list refresh
	}
	
	async function checkAdminStatus(serverId: string) {
		try {
			// Get room state to check power levels
			const response = await fetch(`${API_URL}/rooms/state?access_token=${accessToken}&room_id=${encodeURIComponent(serverId)}`);
			if (response.ok) {
				const state = await response.json();
				// Find power level event
				const powerLevelEvent = state.events?.find((e: any) => e.type === 'm.room.power_levels');
				if (powerLevelEvent && powerLevelEvent.content) {
					const userPower = powerLevelEvent.content.users?.[userId] || powerLevelEvent.content.users_default || 0;
					// Admin typically has power level 100, moderator 50
					isAdmin = userPower >= 50;
				} else {
					// No power levels set, creator is admin
					isAdmin = true;
				}
			}
		} catch (e) {
			console.error('failed to check admin status:', e);
			isAdmin = false;
		}
	}

	async function handleCreateDm() {
		if (!dmUserId.trim()) return;
		
		loading = true;
		error = '';
		
		try {
			// Create a direct message room
			const createResponse = await fetch(`${API_URL}/rooms/create`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					name: dmUserId,
					is_space: false
				})
			});
			
			if (createResponse.ok) {
				const roomData = await createResponse.json();
				
				// Invite the user to the room
				await fetch(`${API_URL}/rooms/invite`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						access_token: accessToken,
						room_id: roomData.room_id,
						user_id: dmUserId.includes(':') ? dmUserId : `@${dmUserId}:localhost`
					})
				});
				
				const displayName = dmUserId.trim().replace(/^@/, '').split(':')[0];
				showCreateDmDialog = false;
				dmUserId = '';
				handleSelectChannel(roomData.room_id, displayName);
			} else {
				error = 'failed to create dm';
			}
		} catch (e) {
			error = 'network error';
		} finally {
			loading = false;
		}
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
	// only show messages when a channel is selected
	let channelMessages = $derived(
		selectedChannelId 
			? messages.filter(m => m.room_id === selectedChannelId)
			: []
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

<div class="flex h-screen w-screen bg-background">
	<!-- server list -->
	<ServerList
		{accessToken}
		{selectedServerId}
		onSelectServer={handleSelectServer}
		onCreateServer={() => showCreateServerDialog = true}
		onJoinServer={() => {}}
		onManageServer={() => showServerManage = true}
		onLeaveServer={handleLeaveServer}
		refreshTrigger={serverRefreshTrigger}
	/>

	<!-- channel list or DM list -->
	{#if isDmMode}
		<DmList
			{accessToken}
			selectedDmId={selectedChannelId}
			onSelectDm={handleSelectChannel}
			onCreateDm={() => showCreateDmDialog = true}
			onShowFriends={() => { showFriendsPage = true; selectedChannelId = null; }}
			refreshTrigger={dmRefreshTrigger}
			{userId}
			{apiUrl}
			onOpenSettings={() => showSettings = true}
		/>
	{:else}
		<ChannelList
			{accessToken}
			serverId={selectedServerId}
			{selectedChannelId}
			onSelectChannel={handleSelectChannel}
			{isAdmin}
			{userId}
			{apiUrl}
			onOpenSettings={() => showSettings = true}
		/>
	{/if}

	<!-- chat area + member list -->
	<div class="flex-1 flex min-h-0">
	{#if isDmMode && showFriendsPage}
		<!-- friends page fills the entire remaining space -->
		<div class="flex-1 flex flex-col min-h-0 bg-background">
			<!-- header -->
			<div class="h-14 border-b border-border flex items-center justify-between px-4 bg-card">
				<span class="font-semibold text-card-foreground">friends</span>
				<div class="flex items-center gap-4">
					<span class="text-sm text-muted-foreground">{userId}</span>
					<Button variant="outline" size="sm" onclick={onLogout}>logout</Button>
				</div>
			</div>
			<div class="flex-1 min-h-0 overflow-hidden">
				<FriendsList
					{accessToken}
					{userId}
					onOpenDm={handleOpenFriendDm}
				/>
			</div>
		</div>
	{:else}
	<div class="flex-1 flex flex-col min-h-0">
		<!-- header -->
		<div class="h-14 border-b border-border flex items-center justify-between px-4 bg-card">
			<div>
			{#if selectedChannelId}
				{#if isDmMode}
					<span class="font-semibold text-card-foreground">{selectedChannelName || 'direct message'}</span>
				{:else}
					<span class="text-muted-foreground">#</span>
					<span class="font-semibold text-card-foreground ml-1">{selectedChannelName || 'channel'}</span>
				{/if}
			{:else}
				<span class="text-muted-foreground">{isDmMode ? 'select a conversation' : 'select a channel'}</span>
			{/if}
			</div>
			<div class="flex items-center gap-4">
				<span class="text-sm text-muted-foreground">{userId}</span>
				<Button variant="outline" size="sm" onclick={onLogout}>
					logout
				</Button>
			</div>
		</div>

		<!-- messages -->
		<div class="flex-1 overflow-y-auto p-4 space-y-3 min-h-0" bind:this={messagesContainer}>
			{#if channelMessages.length === 0}
				<p class="text-muted-foreground text-center mt-8">
					{selectedChannelId ? 'no messages yet' : 'select a channel to view messages'}
				</p>
			{:else}
				{#each channelMessages as message, i (message.event_id || `local-${i}-${message.timestamp}`)}
					<div class="space-y-1">
						<div class="flex items-center gap-2">
							<span class="font-semibold text-sm text-card-foreground">{message.sender}</span>
							<span class="text-xs text-muted-foreground">
								{formatTimestamp(message.timestamp)}
							</span>
						</div>
						<p class="text-sm text-muted-foreground">{message.content}</p>
					</div>
				{/each}
			{/if}
		</div>

		<!-- message input -->
		<div class="p-4 border-t border-border bg-card">
			{#if error}
				<div class="text-sm text-destructive mb-2">{error}</div>
			{/if}
			
			<div class="flex gap-2">
				<Input
					type="text"
					placeholder={selectedChannelId ? "type a message..." : "select a channel first"}
					bind:value={newMessage}
					onkeydown={(e) => e.key === 'Enter' && sendMessage()}
					disabled={loading || !selectedChannelId}
					class="flex-1 bg-muted border-input"
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
			selfUserId={userId}
			channelId={selectedChannelId}
		/>
	{/if}
	{/if}
	</div>

	<!-- create server dialog -->
	{#if showCreateServerDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4 text-card-foreground">create server</h3>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
				{/if}
				<Input
					type="text"
					placeholder="server name"
					bind:value={newServerName}
					class="mb-4 bg-muted border-input"
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

	<!-- create DM dialog -->
	{#if showCreateDmDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4 text-card-foreground">new direct message</h3>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
				{/if}
				<Input
					type="text"
					placeholder="@username:localhost"
					bind:value={dmUserId}
					class="mb-4 bg-muted border-input"
				/>
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showCreateDmDialog = false}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleCreateDm} disabled={!dmUserId.trim()}>
						start chat
					</Button>
				</div>
			</div>
		</div>
	{/if}

	<!-- server management dialog -->
	{#if showServerManage}
		<ServerManage
			serverId={selectedServerId}
			serverName={selectedServerName}
			{accessToken}
			{userId}
			onClose={() => showServerManage = false}
			onLeaveServer={handleLeaveServer}
		/>
	{/if}

	<!-- settings modal -->
	{#if showSettings}
		<SettingsModal
			{userId}
			{accessToken}
			{apiUrl}
			onClose={() => showSettings = false}
		/>
	{/if}
</div>
