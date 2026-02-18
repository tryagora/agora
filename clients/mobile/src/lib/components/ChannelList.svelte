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
		isAdmin?: boolean;
		userId?: string;
		apiUrl?: string;
		onOpenSettings?: () => void;
	}

	let { accessToken, serverId, selectedChannelId, onSelectChannel, isAdmin = false, userId = '', apiUrl = 'http://localhost:3000', onOpenSettings }: Props = $props();

	let channels = $state<Channel[]>([]);
	let categories = $state<Category[]>([]);
	let uncategorizedChannels = $state<Channel[]>([]);
	let loading = $state(false);
	let error = $state('');
	let showCreateChannelDialog = $state(false);
	let showCreateCategoryDialog = $state(false);
	let showDeleteConfirm = $state(false);
	let newChannelName = $state('');
	let newCategoryName = $state('');
	let channelToDelete = $state<Channel | null>(null);
	let selectedCategoryId = $state<string | null>(null);

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
					parent_space_id: selectedCategoryId || serverId
				})
			});
			
			if (response.ok) {
				showCreateChannelDialog = false;
				newChannelName = '';
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
		showCreateChannelDialog = true;
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

	function getChannelIcon(): string {
		return '#';
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
									<div class="group flex items-center">
										<button
											class="flex-1 text-left px-3 py-2 rounded text-muted-foreground hover:bg-muted hover:text-card-foreground transition-colors flex items-center gap-2"
											class:bg-muted={selectedChannelId === channel.room_id}
											class:text-card-foreground={selectedChannelId === channel.room_id}
											onclick={() => onSelectChannel(channel.room_id, channel.name || undefined)}
										>
											<span class="text-muted-foreground">{getChannelIcon()}</span>
											<span class="truncate">{channel.name || 'unnamed'}</span>
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
							<div class="group flex items-center">
								<button
									class="flex-1 text-left px-3 py-2 rounded text-muted-foreground hover:bg-muted hover:text-card-foreground transition-colors flex items-center gap-2"
									class:bg-muted={selectedChannelId === channel.room_id}
									class:text-card-foreground={selectedChannelId === channel.room_id}
									onclick={() => onSelectChannel(channel.room_id, channel.name || undefined)}
								>
									<span class="text-muted-foreground">{getChannelIcon()}</span>
									<span class="truncate">{channel.name || 'unnamed'}</span>
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
