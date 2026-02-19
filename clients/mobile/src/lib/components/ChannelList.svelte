<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';
	import UserPanel from './UserPanel.svelte';

	interface Channel {
		room_id: string;
		name: string | null;
		topic: string | null;
		is_space: boolean;
		channel_type?: string | null;
		parent_id?: string;
	}

	interface Category {
		room_id: string;
		name: string | null;
		children: Channel[];
		isOpen: boolean;
	}

	interface Props {
		accessToken: string;
		serverId: string | null;
		selectedChannelId: string | null;
		onSelectChannel: (channelId: string, channelName?: string) => void;
		onSelectVoiceChannel?: (channelId: string, channelName?: string) => void;
		onSelectForumChannel?: (channelId: string, channelName?: string) => void;
		isAdmin?: boolean;
		userId?: string;
		apiUrl?: string;
		onOpenSettings?: () => void;
		// voice bar — passed down from Chat so the sidebar can show "connected to voice"
		activeVoiceChannelId?: string | null;
		activeVoiceChannelName?: string | null;
		onDisconnectVoice?: () => void;
	}

	let { accessToken, serverId, selectedChannelId, onSelectChannel, onSelectVoiceChannel, onSelectForumChannel, isAdmin = false, userId = '', apiUrl = 'http://localhost:3000', onOpenSettings, activeVoiceChannelId = null, activeVoiceChannelName = null, onDisconnectVoice }: Props = $props();

	let channels = $state<Channel[]>([]);
	let categories = $state<Category[]>([]);
	let uncategorizedChannels = $state<Channel[]>([]);
	let loading = $state(false);
	let error = $state('');
	let showCreateChannelDialog = $state(false);
	let showCreateCategoryDialog = $state(false);
	let showDeleteConfirm = $state(false);
	let newChannelName = $state('');
	let newChannelType = $state<'text' | 'voice' | 'forum'>('text');
	let newCategoryName = $state('');
	let channelToDelete = $state<Channel | null>(null);
	let selectedCategoryId = $state<string | null>(null);

	// maps room_id → list of participant identities currently in voice
	let voiceParticipants = $state<Map<string, string[]>>(new Map());

	const API_URL = 'http://localhost:3000';

	async function loadChannels() {
		if (!serverId) {
			channels = [];
			categories = [];
			uncategorizedChannels = [];
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
				const allChildren: Channel[] = data.children || [];
				
				// separate categories (spaces) from channels
				const categorySpaces = allChildren.filter((c: Channel) => c.is_space);
				const roomChannels = allChildren.filter((c: Channel) => !c.is_space);
				
				// load channels for each category
				const categoryList: Category[] = [];
				for (const cat of categorySpaces) {
					const catParams = new URLSearchParams({
						access_token: accessToken,
						space_id: cat.room_id
					});
					const catResponse = await fetch(`${API_URL}/rooms/children?${catParams}`);
					let catChildren: Channel[] = [];
					if (catResponse.ok) {
						const catData = await catResponse.json();
						catChildren = (catData.children || []).filter((c: Channel) => !c.is_space);
					}
					
					categoryList.push({
						room_id: cat.room_id,
						name: cat.name,
						children: catChildren,
						isOpen: true
					});
				}
				
				categories = categoryList;
				// channels that aren't in any category
				uncategorizedChannels = roomChannels.filter((ch: Channel) => {
					return !categoryList.some((cat: Category) => 
						cat.children.some((c: Channel) => c.room_id === ch.room_id)
					);
				});
				channels = roomChannels;
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
					parent_space_id: selectedCategoryId || serverId,
					channel_type: newChannelType,
				})
			});
			
			if (response.ok) {
				showCreateChannelDialog = false;
				newChannelName = '';
				newChannelType = 'text';
				selectedCategoryId = null;
				await loadChannels();
			} else {
				error = 'failed to create channel';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	async function handleCreateCategory() {
		if (!newCategoryName.trim() || !serverId) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/category/create`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					name: newCategoryName,
					parent_space_id: serverId
				})
			});
			
			if (response.ok) {
				showCreateCategoryDialog = false;
				newCategoryName = '';
				await loadChannels();
			} else {
				error = 'failed to create category';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	async function handleDeleteChannel() {
		if (!channelToDelete) return;
		
		try {
			// First remove from parent space if it's a channel in a category or server
			if (serverId) {
				await fetch(`${API_URL}/rooms/remove_child`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						access_token: accessToken,
						space_id: serverId,
						child_room_id: channelToDelete.room_id
					})
				});
			}
			
			const response = await fetch(`${API_URL}/rooms/delete`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: channelToDelete.room_id
				})
			});
			
			if (response.ok) {
				showDeleteConfirm = false;
				// Remove from local state immediately
				if (channelToDelete.is_space) {
					// It's a category
					categories = categories.filter(c => c.room_id !== channelToDelete!.room_id);
				} else {
					// It's a channel - remove from all categories and uncategorized
					categories = categories.map(cat => ({
						...cat,
						children: cat.children.filter(c => c.room_id !== channelToDelete!.room_id)
					}));
					uncategorizedChannels = uncategorizedChannels.filter(c => c.room_id !== channelToDelete!.room_id);
				}
				channelToDelete = null;
				await loadChannels();
			} else {
				error = 'failed to delete channel';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	function toggleCategory(category: Category) {
		category.isOpen = !category.isOpen;
		categories = [...categories]; // trigger reactivity
	}

	function openCreateChannelDialog(categoryId?: string) {
		selectedCategoryId = categoryId || null;
		newChannelName = '';
		newChannelType = 'text';
		showCreateChannelDialog = true;
	}

	function handleChannelClick(channel: Channel) {
		if (channel.channel_type === 'voice' && onSelectVoiceChannel) {
			onSelectVoiceChannel(channel.room_id, channel.name || undefined);
		} else if (channel.channel_type === 'forum' && onSelectForumChannel) {
			onSelectForumChannel(channel.room_id, channel.name || undefined);
		} else {
			onSelectChannel(channel.room_id, channel.name || undefined);
		}
	}

	function confirmDeleteChannel(channel: Channel) {
		channelToDelete = channel;
		showDeleteConfirm = true;
	}

	// load channels when server changes
	$effect(() => {
		const _id = serverId;
		loadChannels();
	});

	// poll voice participants every 5s for all voice channels in the current server
	async function pollVoiceParticipants() {
		const allVoiceChannels: Channel[] = [
			...uncategorizedChannels.filter(c => c.channel_type === 'voice'),
			...categories.flatMap(cat => cat.children.filter(c => c.channel_type === 'voice')),
		];
		if (allVoiceChannels.length === 0) return;

		const updated = new Map<string, string[]>(voiceParticipants);
		await Promise.all(allVoiceChannels.map(async (ch) => {
			try {
				const params = new URLSearchParams({ room_name: ch.room_id });
				const res = await fetch(`${API_URL}/voice/participants?${params}`);
				if (res.ok) {
					const data = await res.json();
					updated.set(ch.room_id, data.participants || []);
				}
			} catch {
				// ignore — livekit might not be running
			}
		}));
		voiceParticipants = updated;
	}

	$effect(() => {
		// only poll when we have channels loaded
		const _channels = uncategorizedChannels.length + categories.length;
		pollVoiceParticipants();
		const interval = setInterval(pollVoiceParticipants, 5000);
		return () => clearInterval(interval);
	});

	function isVoiceChannel(channel: Channel): boolean {
		return channel.channel_type === 'voice';
	}

	function isForumChannel(channel: Channel): boolean {
		return channel.channel_type === 'forum';
	}
</script>

<div class="flex flex-col h-full w-60 bg-card border-r border-border">
	<!-- header -->
	<div class="p-4 border-b border-border flex items-center justify-between">
		<h2 class="font-semibold text-card-foreground">
			{serverId ? 'channels' : 'dms'}
		</h2>
		{#if serverId && isAdmin}
			<button
				class="w-6 h-6 rounded-full bg-muted hover:bg-accent flex items-center justify-center transition-colors"
				onclick={() => showCreateCategoryDialog = true}
				title="create category"
				aria-label="create category"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
				</svg>
			</button>
		{/if}
	</div>

	<!-- channel list -->
	<ScrollArea class="flex-1 p-2">
		{#if loading}
			<div class="text-center text-muted-foreground text-sm py-4">loading...</div>
		{:else if serverId && (categories.length === 0 && uncategorizedChannels.length === 0)}
			<div class="text-center text-muted-foreground text-sm py-4">
				no channels
				{#if isAdmin}
					<button
						class="block mx-auto mt-2 text-primary hover:underline text-xs"
						onclick={() => openCreateChannelDialog()}
					>
						+ create channel
					</button>
				{/if}
			</div>
		{:else if !serverId}
			<div class="text-center text-muted-foreground text-sm py-4">
				select a server
			</div>
		{:else}
			<div class="space-y-2">
				<!-- Categories -->
				{#each categories as category (category.room_id)}
					<div class="space-y-1">
						<button
							class="w-full flex items-center justify-between px-2 py-1 text-xs font-semibold text-muted-foreground hover:text-card-foreground uppercase tracking-wider"
							onclick={() => toggleCategory(category)}
						>
							<span class="truncate">{category.name || 'category'}</span>
							<span class="transform transition-transform" class:rotate-90={category.isOpen}>
								<svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
								</svg>
							</span>
						</button>
						{#if category.isOpen}
							<div class="space-y-1">
								{#each category.children as channel (channel.room_id)}
									<div class="group flex items-center flex-wrap">
										<button
											class="flex-1 text-left px-3 py-2 rounded text-muted-foreground hover:bg-muted hover:text-card-foreground transition-colors flex items-center gap-2"
											class:bg-muted={selectedChannelId === channel.room_id}
											class:text-card-foreground={selectedChannelId === channel.room_id}
											onclick={() => handleChannelClick(channel)}
										>
									{#if isVoiceChannel(channel)}
											<!-- speaker icon for voice channels -->
											<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
												<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12M9.172 9.172a4 4 0 105.656 5.656" />
											</svg>
										{:else if isForumChannel(channel)}
											<!-- forum / post icon -->
											<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
												<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
											</svg>
										{:else}
											<span class="text-muted-foreground text-xs font-mono">#</span>
										{/if}
										<span class="truncate">{channel.name || 'unnamed'}</span>
										{#if isVoiceChannel(channel) && (voiceParticipants.get(channel.room_id) ?? []).length > 0}
												<span class="ml-auto text-xs text-green-500 flex-shrink-0">{(voiceParticipants.get(channel.room_id) ?? []).length}</span>
											{/if}
										</button>
										{#if isAdmin}
											<button
												class="opacity-0 group-hover:opacity-100 px-2 py-2 text-muted-foreground hover:text-destructive transition-opacity"
												onclick={() => confirmDeleteChannel(channel)}
												title="delete channel"
												aria-label="delete channel"
											>
												<svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
													<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
												</svg>
											</button>
										{/if}
										<!-- voice participant list below channel row -->
										{#if isVoiceChannel(channel)}
											{@const vp = voiceParticipants.get(channel.room_id) ?? []}
											{#each vp as identity (identity)}
												<div class="w-full flex items-center gap-2 pl-8 pr-3 py-0.5">
													<div class="w-4 h-4 rounded-full bg-muted flex items-center justify-center text-xs text-muted-foreground flex-shrink-0">
														{(identity.split(':')[0].replace('@', '')[0] || '?').toUpperCase()}
													</div>
													<span class="text-xs text-muted-foreground truncate">{identity.split(':')[0].replace('@', '')}</span>
													<span class="ml-auto w-1.5 h-1.5 rounded-full bg-green-500 flex-shrink-0"></span>
												</div>
											{/each}
										{/if}
									</div>
								{/each}
								{#if isAdmin}
									<button
										class="w-full text-left px-3 py-1 rounded text-muted-foreground hover:text-card-foreground hover:bg-muted transition-colors flex items-center gap-2 text-sm"
										onclick={() => openCreateChannelDialog(category.room_id)}
									>
										<span class="text-muted-foreground">+</span>
										<span class="truncate">add channel</span>
									</button>
								{/if}
							</div>
						{/if}
					</div>
				{/each}

				<!-- Uncategorized channels -->
				{#if uncategorizedChannels.length > 0}
					<div class="space-y-1">
						<span class="block px-2 py-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
							channels
						</span>
						{#each uncategorizedChannels as channel (channel.room_id)}
							<div class="group flex items-center flex-wrap">
								<button
									class="flex-1 text-left px-3 py-2 rounded text-muted-foreground hover:bg-muted hover:text-card-foreground transition-colors flex items-center gap-2"
									class:bg-muted={selectedChannelId === channel.room_id}
									class:text-card-foreground={selectedChannelId === channel.room_id}
									onclick={() => handleChannelClick(channel)}
								>
								{#if isVoiceChannel(channel)}
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12M9.172 9.172a4 4 0 105.656 5.656" />
									</svg>
								{:else if isForumChannel(channel)}
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
									</svg>
								{:else}
									<span class="text-muted-foreground text-xs font-mono">#</span>
								{/if}
								<span class="truncate">{channel.name || 'unnamed'}</span>
								{#if isVoiceChannel(channel) && (voiceParticipants.get(channel.room_id) ?? []).length > 0}
										<span class="ml-auto text-xs text-green-500 flex-shrink-0">{(voiceParticipants.get(channel.room_id) ?? []).length}</span>
									{/if}
								</button>
								{#if isAdmin}
									<button
										class="opacity-0 group-hover:opacity-100 px-2 py-2 text-muted-foreground hover:text-destructive transition-opacity"
										onclick={() => confirmDeleteChannel(channel)}
										title="delete channel"
										aria-label="delete channel"
									>
										<svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
										</svg>
									</button>
								{/if}
								<!-- voice participant list below channel row -->
								{#if isVoiceChannel(channel)}
									{@const vp = voiceParticipants.get(channel.room_id) ?? []}
									{#each vp as identity (identity)}
										<div class="w-full flex items-center gap-2 pl-8 pr-3 py-0.5">
											<div class="w-4 h-4 rounded-full bg-muted flex items-center justify-center text-xs text-muted-foreground flex-shrink-0">
												{(identity.split(':')[0].replace('@', '')[0] || '?').toUpperCase()}
											</div>
											<span class="text-xs text-muted-foreground truncate">{identity.split(':')[0].replace('@', '')}</span>
											<span class="ml-auto w-1.5 h-1.5 rounded-full bg-green-500 flex-shrink-0"></span>
										</div>
									{/each}
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</ScrollArea>

	<!-- create channel button (only show if in a server and user is admin) -->
	{#if serverId && isAdmin}
		<div class="p-3 border-t border-border">
			<Button 
				variant="outline" 
				class="w-full"
				onclick={() => openCreateChannelDialog()}
			>
				+ create channel
			</Button>
		</div>
	{/if}

	<!-- create channel dialog -->
	{#if showCreateChannelDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4 text-card-foreground">create channel</h3>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
				{/if}
			<!-- channel type selector -->
				<div class="mb-4 flex gap-2">
					<button
						class={`flex-1 flex flex-col items-center gap-1 p-3 rounded border-2 transition-colors ${newChannelType === 'text' ? 'border-primary bg-primary/5' : 'border-border'}`}
						onclick={() => newChannelType = 'text'}
					>
						<span class="text-lg font-mono text-muted-foreground">#</span>
						<span class="text-xs text-card-foreground">text</span>
					</button>
					<button
						class={`flex-1 flex flex-col items-center gap-1 p-3 rounded border-2 transition-colors ${newChannelType === 'voice' ? 'border-primary bg-primary/5' : 'border-border'}`}
						onclick={() => newChannelType = 'voice'}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12M9.172 9.172a4 4 0 105.656 5.656" />
						</svg>
						<span class="text-xs text-card-foreground">voice</span>
					</button>
					<button
						class={`flex-1 flex flex-col items-center gap-1 p-3 rounded border-2 transition-colors ${newChannelType === 'forum' ? 'border-primary bg-primary/5' : 'border-border'}`}
						onclick={() => newChannelType = 'forum'}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
						</svg>
						<span class="text-xs text-card-foreground">forum</span>
					</button>
				</div>
				<Input
					type="text"
					placeholder="channel name"
					bind:value={newChannelName}
					class="mb-4"
				/>
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showCreateChannelDialog = false}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleCreateChannel} disabled={!newChannelName.trim()}>
						create
					</Button>
				</div>
			</div>
		</div>
	{/if}

	<!-- create category dialog -->
	{#if showCreateCategoryDialog}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-4 text-card-foreground">create category</h3>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
				{/if}
				<Input
					type="text"
					placeholder="category name"
					bind:value={newCategoryName}
					class="mb-4"
				/>
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showCreateCategoryDialog = false}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleCreateCategory} disabled={!newCategoryName.trim()}>
						create
					</Button>
				</div>
			</div>
		</div>
	{/if}

	<!-- delete confirmation dialog -->
	{#if showDeleteConfirm && channelToDelete}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
			<div class="bg-card p-6 rounded-lg w-80">
				<h3 class="text-lg font-semibold mb-2 text-card-foreground">delete channel?</h3>
				<p class="text-muted-foreground text-sm mb-4">
					are you sure you want to delete "{channelToDelete.name || 'unnamed'}"? this action cannot be undone.
				</p>
				{#if error}
					<div class="text-destructive text-sm mb-3">{error}</div>
				{/if}
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => { showDeleteConfirm = false; channelToDelete = null; }}>
						cancel
					</Button>
					<Button variant="destructive" class="flex-1" onclick={handleDeleteChannel}>
						delete
					</Button>
				</div>
			</div>
		</div>
	{/if}

	<!-- persistent voice bar — shown at bottom of sidebar when connected to a voice channel -->
	{#if activeVoiceChannelId}
		<div class="border-t border-border bg-green-500/10 px-3 py-2 flex items-center gap-2">
			<!-- animated green indicator -->
			<div class="relative flex-shrink-0">
				<div class="w-2.5 h-2.5 rounded-full bg-green-500"></div>
				<div class="absolute inset-0 rounded-full bg-green-500 animate-ping opacity-60"></div>
			</div>
			<div class="flex-1 min-w-0">
				<p class="text-xs font-semibold text-green-400 truncate">voice connected</p>
				<p class="text-xs text-muted-foreground truncate">{activeVoiceChannelName || 'voice channel'}</p>
			</div>
			{#if onDisconnectVoice}
				<button
					class="flex-shrink-0 w-6 h-6 rounded flex items-center justify-center text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
					onclick={onDisconnectVoice}
					title="disconnect from voice"
					aria-label="disconnect from voice"
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.517l2.257-1.128a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
					</svg>
				</button>
			{/if}
		</div>
	{/if}

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
