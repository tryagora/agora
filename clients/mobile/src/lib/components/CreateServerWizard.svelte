<script lang="ts">
	// create server wizard — multi-step: template → name → preview → create
	// templates pre-populate channels so new servers feel alive immediately

	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	interface Template {
		id: string;
		label: string;
		description: string;
		icon: string;
		// channels to create after server is ready
		channels: { name: string; type: 'text' | 'voice' | 'forum'; category?: string }[];
	}

	const TEMPLATES: Template[] = [
		{
			id: 'gaming',
			label: 'gaming',
			description: 'a place for your gaming crew',
			icon: '🎮',
			channels: [
				{ name: 'general', type: 'text', category: 'text channels' },
				{ name: 'looking-for-group', type: 'text', category: 'text channels' },
				{ name: 'game-night', type: 'voice', category: 'voice channels' },
				{ name: 'chill', type: 'voice', category: 'voice channels' },
			],
		},
		{
			id: 'friends',
			label: 'friends & family',
			description: 'hang out with the people you care about',
			icon: '👥',
			channels: [
				{ name: 'general', type: 'text' },
				{ name: 'vibes', type: 'text' },
				{ name: 'hang', type: 'voice' },
			],
		},
		{
			id: 'study',
			label: 'study group',
			description: 'focused space for studying together',
			icon: '📚',
			channels: [
				{ name: 'announcements', type: 'text', category: 'info' },
				{ name: 'resources', type: 'forum', category: 'info' },
				{ name: 'general-study', type: 'text', category: 'study' },
				{ name: 'pomodoro', type: 'voice', category: 'study' },
			],
		},
		{
			id: 'club',
			label: 'school club',
			description: 'organize your club activities',
			icon: '🎓',
			channels: [
				{ name: 'announcements', type: 'text', category: 'info' },
				{ name: 'general', type: 'text', category: 'members' },
				{ name: 'events', type: 'forum', category: 'members' },
				{ name: 'meeting-room', type: 'voice', category: 'members' },
			],
		},
		{
			id: 'local',
			label: 'local community',
			description: 'connect with people around you',
			icon: '🏘️',
			channels: [
				{ name: 'welcome', type: 'text' },
				{ name: 'announcements', type: 'text' },
				{ name: 'events-board', type: 'forum' },
				{ name: 'town-hall', type: 'voice' },
			],
		},
		{
			id: 'custom',
			label: 'custom',
			description: 'start from scratch',
			icon: '✨',
			channels: [
				{ name: 'general', type: 'text' },
			],
		},
	];

	interface Props {
		accessToken: string;
		onCreated: (serverId: string, serverName: string) => void;
		onClose: () => void;
	}

	let { accessToken, onCreated, onClose }: Props = $props();

	const API_URL = 'http://localhost:3000';

	let step = $state<'template' | 'name' | 'creating'>('template');
	let selectedTemplate = $state<Template | null>(null);
	let serverName = $state('');
	let error = $state('');
	let creating = $state(false);

	function selectTemplate(t: Template) {
		selectedTemplate = t;
		// pre-fill name if empty
		if (!serverName) serverName = t.id === 'custom' ? '' : t.label;
		step = 'name';
	}

	async function handleCreate() {
		if (!serverName.trim() || !selectedTemplate) return;
		creating = true;
		error = '';

		try {
			// 1. create the server (space)
			const res = await fetch(`${API_URL}/rooms/create`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					name: serverName.trim(),
					is_space: true,
				}),
			});

			if (!res.ok) {
				error = 'failed to create server';
				creating = false;
				return;
			}

			const { room_id: serverId } = await res.json();

			// 2. tag it with the template in server meta
			await fetch(`${API_URL}/servers/meta`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					server_id: serverId,
					name: serverName.trim(),
					template: selectedTemplate.id,
				}),
			});

			// 3. create categories first (unique category names in template)
			const categoryNames = [...new Set(
				selectedTemplate.channels
					.map(c => c.category)
					.filter((c): c is string => !!c)
			)];

			const categoryIds: Record<string, string> = {};
			for (const catName of categoryNames) {
				const catRes = await fetch(`${API_URL}/rooms/category/create`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						access_token: accessToken,
						name: catName,
						parent_space_id: serverId,
					}),
				});
				if (catRes.ok) {
					const catData = await catRes.json();
					categoryIds[catName] = catData.room_id;
				}
			}

			// 4. create channels
			for (const ch of selectedTemplate.channels) {
				const parentId = ch.category && categoryIds[ch.category]
					? categoryIds[ch.category]
					: serverId;

				await fetch(`${API_URL}/rooms/create`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						access_token: accessToken,
						name: ch.name,
						is_space: false,
						parent_space_id: parentId,
						channel_type: ch.type,
					}),
				});
			}

			onCreated(serverId, serverName.trim());
		} catch (e) {
			error = 'network error';
			creating = false;
		}
	}
</script>

<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
	<div class="bg-card rounded-xl shadow-2xl w-[480px] max-h-[80vh] overflow-y-auto">
		<!-- header -->
		<div class="flex items-center justify-between px-6 pt-6 pb-4 border-b border-border">
			<div>
				<h2 class="text-lg font-semibold text-card-foreground">create a server</h2>
				<p class="text-xs text-muted-foreground mt-0.5">
					{#if step === 'template'}
						pick a template to get started
					{:else}
						give your server a name
					{/if}
				</p>
			</div>
			<button
				class="w-8 h-8 rounded-full flex items-center justify-center text-muted-foreground hover:text-card-foreground hover:bg-muted transition-colors"
				onclick={onClose}
				aria-label="close"
				type="button"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</button>
		</div>

		<!-- step: template picker -->
		{#if step === 'template'}
			<div class="p-6 grid grid-cols-2 gap-3">
				{#each TEMPLATES as t (t.id)}
					<button
						class="text-left p-4 rounded-lg border-2 border-border hover:border-primary hover:bg-primary/5 transition-colors"
						onclick={() => selectTemplate(t)}
					>
						<span class="text-2xl">{t.icon}</span>
						<p class="text-sm font-semibold text-card-foreground mt-2">{t.label}</p>
						<p class="text-xs text-muted-foreground mt-0.5">{t.description}</p>
					</button>
				{/each}
			</div>
		{/if}

		<!-- step: name + preview -->
		{#if step === 'name' && selectedTemplate}
			<div class="p-6 space-y-5">
				<!-- template badge -->
				<div class="flex items-center gap-2 text-sm text-muted-foreground">
					<button
						class="hover:text-card-foreground transition-colors"
						onclick={() => step = 'template'}
						type="button"
					>
						← back
					</button>
					<span>·</span>
					<span>{selectedTemplate.icon} {selectedTemplate.label}</span>
				</div>

				<!-- name input -->
				<div class="space-y-2">
					<label for="server-name-input" class="text-sm font-medium text-card-foreground">
						server name
					</label>
					<Input
						id="server-name-input"
						type="text"
						placeholder="my awesome server"
						bind:value={serverName}
						class="bg-muted border-input"
					/>
				</div>

				<!-- channels preview -->
				<div class="space-y-2">
					<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">channels that will be created</p>
					<div class="bg-muted rounded-lg p-3 space-y-1 max-h-40 overflow-y-auto">
						{#each selectedTemplate.channels as ch (ch.name + (ch.category || ''))}
							<div class="flex items-center gap-2 text-sm text-muted-foreground">
								{#if ch.type === 'voice'}
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M12 6v12M9.172 9.172a4 4 0 105.656 5.656" />
									</svg>
								{:else if ch.type === 'forum'}
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
									</svg>
								{:else}
									<span class="text-xs font-mono">#</span>
								{/if}
								<span class="truncate">{ch.name}</span>
								{#if ch.category}
									<span class="ml-auto text-xs opacity-50 flex-shrink-0">{ch.category}</span>
								{/if}
							</div>
						{/each}
					</div>
				</div>

				{#if error}
					<div class="text-destructive text-sm">{error}</div>
				{/if}

				<div class="flex gap-2 pt-1">
					<Button variant="outline" class="flex-1" onclick={onClose} disabled={creating}>
						cancel
					</Button>
					<Button class="flex-1" onclick={handleCreate} disabled={!serverName.trim() || creating}>
						{creating ? 'creating...' : 'create server'}
					</Button>
				</div>
			</div>
		{/if}
	</div>
</div>
