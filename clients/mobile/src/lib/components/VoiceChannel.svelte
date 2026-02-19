<script lang="ts">
	import {
		Room,
		RoomEvent,
		LocalParticipant,
		RemoteParticipant,
		Track,
		type Participant,
	} from 'livekit-client';
	import VibeRoom from './VibeRoom.svelte';

	interface Props {
		roomId: string;
		roomName: string;
		userId: string;
		accessToken: string;
		apiUrl?: string;
		onDisconnect?: () => void;
		onParticipantsChange?: (participants: ParticipantInfo[]) => void;
	}

	let {
		roomId,
		roomName,
		userId,
		accessToken,
		apiUrl = 'http://localhost:3000',
		onDisconnect,
		onParticipantsChange,
	}: Props = $props();

	type ParticipantInfo = {
		identity: string;
		displayName: string;
		isSpeaking: boolean;
		isMuted: boolean;
		isLocal: boolean;
	};

	let room: Room | null = $state(null);
	let participants = $state<ParticipantInfo[]>([]);
	let muted = $state(false);
	let connecting = $state(false);
	let error = $state('');

	// vibe room state — synced from matrix state every 5s
	let currentVibe = $state('none');
	let vibeSetBy = $state<string | undefined>(undefined);

	async function pollVibe() {
		try {
			const params = new URLSearchParams({ access_token: accessToken, room_id: roomId });
			const res = await fetch(`${apiUrl}/voice/vibe?${params}`);
			if (res.ok) {
				const data = await res.json();
				currentVibe = data.vibe ?? 'none';
				vibeSetBy = data.set_by ?? undefined;
			}
		} catch {
			// livekit or backend unreachable — leave vibe unchanged
		}
	}

	$effect(() => {
		// poll vibe immediately and every 5s while connected
		const _room = room; // reactive dependency
		if (!_room) return;
		pollVibe();
		const interval = setInterval(pollVibe, 5000);
		return () => clearInterval(interval);
	});

	function participantToInfo(p: Participant, isLocal: boolean): ParticipantInfo {
		// check if the participant has any published audio track that is muted
		let isMuted = true;
		for (const pub of p.trackPublications.values()) {
			if (pub.kind === Track.Kind.Audio) {
				isMuted = pub.isMuted;
				break;
			}
		}
		return {
			identity: p.identity,
			displayName: p.name || p.identity,
			isSpeaking: p.isSpeaking,
			isMuted,
			isLocal,
		};
	}

	function rebuildParticipants() {
		if (!room) return;
		const list: ParticipantInfo[] = [];
		list.push(participantToInfo(room.localParticipant, true));
		for (const p of room.remoteParticipants.values()) {
			list.push(participantToInfo(p, false));
		}
		participants = list;
		onParticipantsChange?.(list);
	}

	async function connect() {
		connecting = true;
		error = '';

		try {
			// get display name from identity (strip @...localhost)
			const displayName = userId.split(':')[0].replace('@', '');

			const res = await fetch(`${apiUrl}/voice/token`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: roomId,
					user_id: userId,
					display_name: displayName,
				}),
			});

			if (!res.ok) {
				error = 'failed to get voice token';
				return;
			}

			const { token, livekit_url } = await res.json();

			const r = new Room({
				adaptiveStream: true,
				disconnectOnPageLeave: true,
			});

			// wire up event listeners before connecting
			r.on(RoomEvent.ParticipantConnected, () => rebuildParticipants());
			r.on(RoomEvent.ParticipantDisconnected, () => rebuildParticipants());
			r.on(RoomEvent.ActiveSpeakersChanged, () => rebuildParticipants());
			r.on(RoomEvent.TrackMuted, () => rebuildParticipants());
			r.on(RoomEvent.TrackUnmuted, () => rebuildParticipants());
			r.on(RoomEvent.LocalTrackPublished, () => rebuildParticipants());
			r.on(RoomEvent.Disconnected, () => {
				room = null;
				participants = [];
				onDisconnect?.();
			});

			await r.connect(livekit_url, token);

			// connected — show the room immediately before waiting for mic
			room = r;
			connecting = false;
			rebuildParticipants();

			// enable microphone separately — getUserMedia can hang on permission prompt
			// catch errors here so a denied/missing mic doesn't kill the whole connection
			try {
				await r.localParticipant.setMicrophoneEnabled(true);
				rebuildParticipants();
			} catch (micErr) {
				// mic failed (permission denied, no device, etc.) — stay connected but muted
				muted = true;
				error = `mic unavailable: ${micErr}`;
				// clear the error after 4s so it doesn't linger
				setTimeout(() => { error = ''; }, 4000);
			}
		} catch (e) {
			error = `connection failed: ${e}`;
		} finally {
			// ensure connecting is cleared even if something unexpected throws
			connecting = false;
		}
	}

	async function disconnect() {
		if (room) {
			await room.disconnect();
		}
		room = null;
		participants = [];
		onDisconnect?.();
	}

	async function toggleMute() {
		if (!room) return;
		const newMuted = !muted;
		await room.localParticipant.setMicrophoneEnabled(!newMuted);
		muted = newMuted;
		rebuildParticipants();
	}

	// connect immediately when component mounts
	$effect(() => {
		connect();
		return () => {
			// cleanup on unmount
			room?.disconnect();
		};
	});
</script>

<div class="flex flex-col h-full">
	<!-- header -->
	<div class="p-3 border-b border-border flex items-center gap-2">
		<!-- speaker icon -->
		<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-green-500 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12M9.172 9.172a4 4 0 105.656 5.656" />
		</svg>
		<span class="text-sm font-semibold text-card-foreground truncate">{roomName}</span>
		{#if connecting}
			<span class="ml-auto text-xs text-muted-foreground">connecting...</span>
		{:else if room}
			<span class="ml-auto text-xs text-green-500">connected</span>
		{/if}
	</div>

	{#if error}
		<div class="p-3 text-destructive text-sm">{error}</div>
	{/if}

	<!-- participant list -->
	<div class="flex-1 overflow-y-auto p-2 space-y-1">
		{#if connecting}
			<div class="text-center text-muted-foreground text-xs py-4">joining voice...</div>
		{:else if participants.length === 0}
			<div class="text-center text-muted-foreground text-xs py-4">no one here yet</div>
		{:else}
			{#each participants as p (p.identity)}
				<div
					class={`flex items-center gap-2 px-2 py-1.5 rounded transition-colors ${p.isSpeaking ? 'bg-green-500/10' : ''}`}
				>
					<!-- avatar placeholder -->
					<div
						class="w-7 h-7 rounded-full bg-muted flex items-center justify-center flex-shrink-0 text-xs font-semibold text-muted-foreground border-2 transition-colors"
						class:border-green-500={p.isSpeaking}
						class:border-transparent={!p.isSpeaking}
					>
						{(p.displayName[0] || '?').toUpperCase()}
					</div>

					<span class="text-sm text-card-foreground truncate flex-1">{p.displayName}{#if p.isLocal} (you){/if}</span>

					<!-- mic icon: muted vs active -->
					{#if p.isMuted}
						<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-destructive flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
							<!-- slash line -->
							<line x1="3" y1="3" x2="21" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
						</svg>
					{:else}
						<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
						</svg>
					{/if}
				</div>
			{/each}
		{/if}
	</div>

	<!-- vibe room picker — shown once connected -->
	{#if room}
		<div class="px-2 pb-2">
			<VibeRoom
				{roomId}
				{accessToken}
				{userId}
				{apiUrl}
				currentVibe={currentVibe}
				setBy={vibeSetBy}
				onVibeChange={(v) => { currentVibe = v; }}
			/>
		</div>
	{/if}

	<!-- controls bar -->
	{#if room}
		<div class="p-3 border-t border-border flex items-center gap-2">
			<!-- mute/unmute button -->
			<button
				class={`flex items-center justify-center w-8 h-8 rounded-full transition-colors ${muted ? 'bg-destructive text-destructive-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}`}
				onclick={toggleMute}
				title={muted ? 'unmute' : 'mute'}
				aria-label={muted ? 'unmute microphone' : 'mute microphone'}
			>
				{#if muted}
					<!-- mic-off -->
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
						<line x1="3" y1="3" x2="21" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
					</svg>
				{:else}
					<!-- mic-on -->
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
					</svg>
				{/if}
			</button>

			<span class="text-xs text-muted-foreground flex-1 truncate">
				{muted ? 'muted' : 'voice active'}
			</span>

			<!-- disconnect button -->
			<button
				class="flex items-center justify-center w-8 h-8 rounded-full bg-destructive/10 hover:bg-destructive/20 text-destructive transition-colors"
				onclick={disconnect}
				title="disconnect from voice"
				aria-label="disconnect from voice channel"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.517l2.257-1.128a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
				</svg>
			</button>
		</div>
	{/if}
</div>
