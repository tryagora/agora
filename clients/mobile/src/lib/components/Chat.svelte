<script lang="ts">
	import ServerList from './ServerList.svelte';
	import ChannelList from './ChannelList.svelte';
	import DmList from './DmList.svelte';
	import FriendsList from './FriendsList.svelte';
	import MemberList from './MemberList.svelte';
	import ServerSettings from './ServerSettings.svelte';
	import CreateServerWizard from './CreateServerWizard.svelte';
	import ForumChannel from './ForumChannel.svelte';
	import SettingsModal from './SettingsModal.svelte';
	import VoiceChannel from './VoiceChannel.svelte';
	import IncomingCall from './IncomingCall.svelte';
	import RaidAlert from './RaidAlert.svelte';
	import HypeTrain from './HypeTrain.svelte';
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
	let showCreateServerWizard = $state(false);
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

	// voice channel state — when set, the main area shows VoiceChannel instead of chat
	let activeVoiceChannelId = $state<string | null>(null);
	let activeVoiceChannelName = $state<string | null>(null);
	// forum channel state — when set, the main area shows ForumChannel
	let activeForumChannelId = $state<string | null>(null);
	let activeForumChannelName = $state<string | null>(null);
	// dm call state — inline call panel above messages
	let showDmCall = $state(false);

	// ── call signaling ─────────────────────────────────────────────────────────
	// outgoing call state (we are the caller)
	let outgoingCallId = $state<string | null>(null);
	let outgoingCallRoomId = $state<string | null>(null);
	let outgoingCallTimeout: ReturnType<typeof setTimeout> | null = null;

	// incoming call state (we are the callee)
	let incomingCall = $state<{
		callId: string;
		roomId: string;
		callerName: string;
		callerId: string;
	} | null>(null);

	// call events we have already processed — prevents re-triggering on re-sync
	const seenCallIds = new Set<string>();

	// ── raid alert state ──────────────────────────────────────────────────────
	let activeRaid = $state<{
		raiderName: string;
		message: string;
		countdown: number;
	} | null>(null);
	const seenRaidIds = new Set<string>();

	// ── hype train state ──────────────────────────────────────────────────────
	// timestamps of messages in the current channel, kept rolling for hype detection
	let channelMessageTimestamps = $state<number[]>([]);
	let hypeActive = $state(false);

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

					// split: signaling events vs regular messages
					const isMsgType = (m: any, t: string) =>
						m.content_type === t || (m.content && typeof m.content === 'object' && (m.content as any).msgtype === t);

					const callEvents = newMessages.filter((m: any) => isMsgType(m, 'agora.call'));
					const raidEvents = newMessages.filter((m: any) => isMsgType(m, 'agora.raid'));
					const textMessages = newMessages.filter((m: any) => !isMsgType(m, 'agora.call') && !isMsgType(m, 'agora.raid'));

					// process signaling events immediately (before storing messages)
					if (initialSyncDone) {
						for (const evt of callEvents) {
							handleCallEvent({
								...evt,
								call_id: evt.call_id,
								action: evt.action,
								from: evt.from,
								display_name: evt.display_name,
							});
						}
						for (const evt of raidEvents) {
							handleRaidEvent(evt);
						}
					}

					// update hype train timestamps for current channel
					const now = Date.now();
					const incomingForChannel = textMessages.filter((m: any) => m.room_id === selectedChannelId);
					if (incomingForChannel.length > 0) {
						const fresh = incomingForChannel.map((m: any) => m.timestamp ?? now);
						// keep last 30 seconds of timestamps to avoid unbounded growth
						const cutoff = now - 30_000;
						channelMessageTimestamps = [...channelMessageTimestamps.filter(t => t > cutoff), ...fresh];
					}

					// cap total messages to 500 to prevent unbounded memory growth
					// — old messages are evicted from the front, keeping the newest
					const combined = [...messages, ...textMessages];
					messages = combined.length > 500 ? combined.slice(combined.length - 500) : combined;
						// fire notifications only after the initial history load
						if (initialSyncDone) {
							for (const m of textMessages) {
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

	function handleServerCreated(serverId: string, serverName: string) {
		showCreateServerWizard = false;
		serverRefreshTrigger++;
		// auto-select the new server
		handleSelectServer(serverId, serverName);
	}



	function handleSelectChannel(channelId: string, channelName?: string) {
		selectedChannelId = channelId;
		selectedChannelName = channelName || null;
		// reset hype train when switching channels
		channelMessageTimestamps = [];
		hypeActive = false;
		// clear any active voice/forum channel
		activeVoiceChannelId = null;
		activeVoiceChannelName = null;
		activeForumChannelId = null;
		activeForumChannelName = null;
		showDmCall = false;
		if (!selectedServerId) {
			// selecting a dm — stay in DM mode but leave friends page
			isDmMode = true;
			showFriendsPage = false;
		} else {
			isDmMode = false;
		}
	}

	function handleSelectVoiceChannel(channelId: string, channelName?: string) {
		activeVoiceChannelId = channelId;
		activeVoiceChannelName = channelName || null;
		// clear text/forum channel selection — voice takes over the main area
		selectedChannelId = null;
		selectedChannelName = null;
		activeForumChannelId = null;
		activeForumChannelName = null;
		isDmMode = false;
		showFriendsPage = false;
	}

	function handleSelectForumChannel(channelId: string, channelName?: string) {
		activeForumChannelId = channelId;
		activeForumChannelName = channelName || null;
		// clear other channel types
		selectedChannelId = null;
		selectedChannelName = null;
		activeVoiceChannelId = null;
		activeVoiceChannelName = null;
		isDmMode = false;
		showFriendsPage = false;
	}

	async function handleSelectServer(serverId: string, serverName?: string) {
		selectedServerId = serverId || null;
		selectedServerName = serverName || null;
		selectedChannelId = null;
		activeVoiceChannelId = null;
		activeVoiceChannelName = null;
		activeForumChannelId = null;
		activeForumChannelName = null;
		showDmCall = false;
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

	// ── raid helpers ──────────────────────────────────────────────────────────

	async function startRaid(message?: string, countdown?: number) {
		if (!selectedChannelId) return;
		const raiderName = userId.split(':')[0].replace('@', '');
		try {
			await fetch(`${API_URL}/rooms/raid`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: selectedChannelId,
					raider_id: userId,
					raider_name: raiderName,
					message: message ?? 'RAID!',
					countdown: countdown ?? 5,
				}),
			});
		} catch (e) {
			console.error('failed to send raid:', e);
		}
	}

	function handleRaidEvent(evt: any) {
		const eventId = evt.event_id || (evt.raider_id + evt.timestamp);
		if (!eventId || seenRaidIds.has(eventId)) return;
		seenRaidIds.add(eventId);
		activeRaid = {
			raiderName: evt.raider_name || 'someone',
			message: evt.message || 'RAID!',
			countdown: evt.countdown ?? 5,
		};
	}

	// ── call signaling helpers ────────────────────────────────────────────────

	async function sendCallEvent(roomId: string, action: string, callId: string) {
		try {
			await fetch(`${API_URL}/voice/call`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: roomId,
					action,
					call_id: callId,
					from_user_id: userId,
					display_name: userId.split(':')[0].replace('@', ''),
				}),
			});
		} catch (e) {
			console.error('failed to send call event:', e);
		}
	}

	// start an outgoing DM voice call
	async function startDmCall() {
		const roomId = selectedChannelId;
		if (!roomId || !isDmMode) return;

		// if a call is already active (accepted), just show it inline
		if (showDmCall) { showDmCall = false; return; }

		const callId = crypto.randomUUID();
		outgoingCallId = callId;
		outgoingCallRoomId = roomId;

		await sendCallEvent(roomId, 'ring', callId);

		// notify caller via desktop if hidden
		try {
			if (await isPermissionGranted()) {
				sendNotification({ title: 'calling...', body: `waiting for ${selectedChannelName || 'them'} to answer` });
			}
		} catch { /* not in tauri */ }

		// auto-cancel after 30 seconds if no answer
		outgoingCallTimeout = setTimeout(async () => {
			if (outgoingCallId === callId) {
				await sendCallEvent(roomId, 'cancel', callId);
				outgoingCallId = null;
				outgoingCallRoomId = null;
			}
		}, 30_000);
	}

	async function cancelOutgoingCall() {
		if (!outgoingCallId || !outgoingCallRoomId) return;
		if (outgoingCallTimeout) { clearTimeout(outgoingCallTimeout); outgoingCallTimeout = null; }
		await sendCallEvent(outgoingCallRoomId, 'cancel', outgoingCallId);
		outgoingCallId = null;
		outgoingCallRoomId = null;
	}

	async function acceptIncomingCall() {
		if (!incomingCall) return;
		const { callId, roomId } = incomingCall;
		incomingCall = null;
		await sendCallEvent(roomId, 'accept', callId);
		// navigate to the DM room and open the voice panel
		handleSelectChannel(roomId, selectedChannelName || undefined);
		showDmCall = true;
	}

	async function declineIncomingCall() {
		if (!incomingCall) return;
		const { callId, roomId } = incomingCall;
		incomingCall = null;
		await sendCallEvent(roomId, 'cancel', callId);
	}

	// process a call event message received via sync
	function handleCallEvent(msg: Message & { call_id?: string; action?: string; from?: string; display_name?: string }) {
		const { call_id, action, from, room_id } = msg;
		if (!call_id || !action || !from) return;
		// skip our own call events
		if (from === userId) {
			// if we sent a 'cancel' or the callee sent 'accept', clear outgoing call
			if (action === 'cancel' && outgoingCallId === call_id) {
				if (outgoingCallTimeout) { clearTimeout(outgoingCallTimeout); outgoingCallTimeout = null; }
				outgoingCallId = null;
				outgoingCallRoomId = null;
			}
			if (action === 'accept' && outgoingCallId === call_id) {
				if (outgoingCallTimeout) { clearTimeout(outgoingCallTimeout); outgoingCallTimeout = null; }
				outgoingCallId = null;
				outgoingCallRoomId = null;
				// the other side accepted — open our voice panel
				showDmCall = true;
			}
			return;
		}
		// dedup — don't show the same call event twice
		if (seenCallIds.has(call_id + action)) return;
		seenCallIds.add(call_id + action);

		if (action === 'ring') {
			// someone is calling us
			const callerName = (msg.display_name || from).split(':')[0].replace('@', '');
			incomingCall = { callId: call_id, roomId: room_id, callerName, callerId: from };
			// desktop notification so they hear it even when window is hidden
			try {
				isPermissionGranted().then(granted => {
					if (granted) sendNotification({ title: 'incoming call', body: `${callerName} is calling you` });
				});
			} catch { /* not in tauri */ }
		} else if (action === 'cancel' || action === 'accept') {
			// call was cancelled by caller, or already accepted — dismiss incoming
			if (incomingCall?.callId === call_id) {
				incomingCall = null;
			}
		}
	}

	// adaptive sync polling:
	// - 5s when the window is visible
	// - 30s when hidden (minimized to tray) — notifications still arrive via WS
	// this cuts idle CPU and network traffic significantly
	$effect(() => {
		let interval: ReturnType<typeof setInterval>;

		function startInterval() {
			clearInterval(interval);
			const delay = document.hidden ? 30_000 : 5_000;
			interval = setInterval(sync, delay);
		}

		function onVisibilityChange() {
			startInterval(); // restart with new delay when visibility changes
			if (!document.hidden) sync(); // immediate sync when window regains focus
		}

		document.addEventListener('visibilitychange', onVisibilityChange);
		sync(); // initial sync
		startInterval();

		return () => {
			clearInterval(interval);
			document.removeEventListener('visibilitychange', onVisibilityChange);
		};
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
		onCreateServer={() => showCreateServerWizard = true}
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
			onSelectVoiceChannel={handleSelectVoiceChannel}
			onSelectForumChannel={handleSelectForumChannel}
			{isAdmin}
			{userId}
			{apiUrl}
			onOpenSettings={() => showSettings = true}
			{activeVoiceChannelId}
			{activeVoiceChannelName}
			onDisconnectVoice={() => { activeVoiceChannelId = null; activeVoiceChannelName = null; }}
		/>
	{/if}

	<!-- chat area + member list -->
	<div class="flex-1 flex min-h-0">
	{#if activeForumChannelId}
		<!-- forum channel fills the entire main area -->
		<div class="flex-1 flex flex-col min-h-0 bg-background">
			<ForumChannel
				forumChannelId={activeForumChannelId}
				forumChannelName={activeForumChannelName || 'forum'}
				{userId}
				{accessToken}
				{apiUrl}
			/>
		</div>
	{:else if activeVoiceChannelId}
		<!-- voice channel fills the entire main area -->
		<div class="flex-1 flex flex-col min-h-0 bg-background">
			<div class="h-14 border-b border-border flex items-center justify-between px-4 bg-card">
				<div class="flex items-center gap-2">
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12M9.172 9.172a4 4 0 105.656 5.656" />
					</svg>
					<span class="font-semibold text-card-foreground">{activeVoiceChannelName || 'voice channel'}</span>
				</div>
				<div class="flex items-center gap-4">
					<span class="text-sm text-muted-foreground">{userId}</span>
					<Button variant="outline" size="sm" onclick={onLogout}>logout</Button>
				</div>
			</div>
			<div class="flex-1 min-h-0 max-w-sm mx-auto w-full py-4 px-4">
				<VoiceChannel
					roomId={activeVoiceChannelId}
					roomName={activeVoiceChannelName || 'voice channel'}
					{userId}
					{accessToken}
					{apiUrl}
					onDisconnect={() => { activeVoiceChannelId = null; activeVoiceChannelName = null; }}
				/>
			</div>
		</div>
	{:else if isDmMode && showFriendsPage}
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
			<div class="flex items-center gap-3">
				<!-- raid button — server channels only, admin only -->
				{#if selectedChannelId && !isDmMode && isAdmin}
					<button
						class="px-2 h-7 rounded text-xs font-black tracking-widest uppercase bg-red-900/40 text-red-400 hover:bg-red-900/70 border border-red-800/50 transition-colors"
						onclick={() => startRaid()}
						title="trigger a raid alert for everyone in this channel"
						aria-label="raid"
					>
						⚔ RAID
					</button>
				{/if}
				<!-- voice call button — only show in DM rooms -->
				{#if selectedChannelId && isDmMode}
					<button
						class={`w-8 h-8 rounded-full flex items-center justify-center transition-colors ${showDmCall || outgoingCallId ? 'bg-green-500 text-white' : 'bg-muted text-muted-foreground hover:bg-muted/80'}`}
						onclick={startDmCall}
						title={showDmCall ? 'end call' : 'start voice call'}
						aria-label={showDmCall ? 'end voice call' : 'start voice call'}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z" />
						</svg>
					</button>
				{/if}
				<span class="text-sm text-muted-foreground">{userId}</span>
				<Button variant="outline" size="sm" onclick={onLogout}>
					logout
				</Button>
			</div>
		</div>

		<!-- dm voice call panel — shown above messages when call is active -->
		{#if showDmCall && selectedChannelId && isDmMode}
			<div class="h-48 border-b border-border bg-card/50 flex-shrink-0">
				<VoiceChannel
					roomId={selectedChannelId}
					roomName={selectedChannelName || 'call'}
					{userId}
					{accessToken}
					{apiUrl}
					onDisconnect={() => showDmCall = false}
				/>
			</div>
		{/if}

		<!-- hype train banner — appears above messages when chat is going fast -->
		<HypeTrain
			recentTimestamps={channelMessageTimestamps}
			onHypeChange={(active) => { hypeActive = active; }}
		/>

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

	<!-- raid alert overlay — shown to everyone when a raid is triggered -->
	{#if activeRaid}
		<RaidAlert
			raiderName={activeRaid.raiderName}
			message={activeRaid.message}
			countdown={activeRaid.countdown}
			onDismiss={() => { activeRaid = null; }}
		/>
	{/if}

	<!-- outgoing call overlay — shown while waiting for the other side to pick up -->
	{#if outgoingCallId && outgoingCallRoomId}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
			<div class="bg-card rounded-2xl shadow-2xl w-72 overflow-hidden">
				<div class="bg-primary/10 px-6 pt-8 pb-6 flex flex-col items-center gap-3">
					<div class="relative">
						<div class="w-20 h-20 rounded-full bg-primary flex items-center justify-center text-primary-foreground text-3xl font-bold">
							{((selectedChannelName || '?')[0]).toUpperCase()}
						</div>
						<div class="absolute inset-0 rounded-full border-2 border-primary animate-ping opacity-40"></div>
					</div>
					<div class="text-center">
						<p class="font-semibold text-card-foreground text-lg">{selectedChannelName || 'user'}</p>
						<p class="text-sm text-muted-foreground">calling...</p>
					</div>
				</div>
				<div class="flex border-t border-border">
					<button
						class="flex-1 flex flex-col items-center gap-1.5 py-4 hover:bg-destructive/10 transition-colors"
						onclick={cancelOutgoingCall}
						aria-label="cancel call"
					>
						<div class="w-10 h-10 rounded-full bg-destructive flex items-center justify-center">
							<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.517l2.257-1.128a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
							</svg>
						</div>
						<span class="text-xs text-muted-foreground">cancel</span>
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- incoming call overlay -->
	{#if incomingCall}
		<IncomingCall
			callerName={incomingCall.callerName}
			callId={incomingCall.callId}
			roomId={incomingCall.roomId}
			onAccept={acceptIncomingCall}
			onDecline={declineIncomingCall}
		/>
	{/if}

	<!-- create server wizard -->
	{#if showCreateServerWizard}
		<CreateServerWizard
			{accessToken}
			onCreated={handleServerCreated}
			onClose={() => showCreateServerWizard = false}
		/>
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

	<!-- server settings dialog -->
	{#if showServerManage && selectedServerId}
		<ServerSettings
			serverId={selectedServerId}
			serverName={selectedServerName}
			{accessToken}
			{userId}
			onClose={() => showServerManage = false}
			onLeaveServer={handleLeaveServer}
			onNameChanged={(name) => { selectedServerName = name; }}
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
