<script lang="ts">
	// server settings panel — tabbed: overview, roles, members, invites
	// replaces the old thin ServerManage component

	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import RolesEditor from './RolesEditor.svelte';

	interface Member {
		user_id: string;
		display_name: string | null;
		power_level: number;
	}

	interface InviteInfo {
		alias: string;
		vanity_slug: string | null;
		server_name: string;
		member_count: number;
	}

	interface Props {
		serverId: string;
		serverName: string | null;
		accessToken: string;
		userId: string;
		onClose: () => void;
		onLeaveServer?: () => void;
		onNameChanged?: (newName: string) => void;
	}

	let { serverId, serverName, accessToken, userId, onClose, onLeaveServer, onNameChanged }: Props = $props();

	const API_URL = 'http://localhost:3000';

	type Tab = 'overview' | 'roles' | 'members' | 'invites';
	let activeTab = $state<Tab>('overview');

	// overview tab
	let editName = $state(serverName || '');
	let editDescription = $state('');
	let editVanitySlug = $state('');
	let savingMeta = $state(false);
	let metaError = $state('');
	let metaSuccess = $state('');

	// members tab
	let members = $state<Member[]>([]);
	let loadingMembers = $state(false);

	// invites tab
	let inviteInfo = $state<InviteInfo | null>(null);
	let loadingInvite = $state(false);
	let copied = $state(false);

	// danger zone
	let showLeaveConfirm = $state(false);
	let leavingServer = $state(false);
	let dangerError = $state('');
	// true if the current user is the server owner (power level 100)
	let isOwner = $state(false);

	async function checkOwner() {
		try {
			const res = await fetch(`${API_URL}/rooms/permissions?access_token=${accessToken}&room_id=${encodeURIComponent(serverId)}`);
			if (res.ok) {
				const data = await res.json();
				const myPower = data.users?.[userId] ?? data.users_default ?? 0;
				isOwner = myPower >= 100;
			}
		} catch { /* non-fatal */ }
	}

	async function handleDeleteServer() {
		leavingServer = true;
		dangerError = '';
		try {
			const res = await fetch(`${API_URL}/rooms/delete_server`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, room_id: serverId, user_id: userId }),
			});
			if (res.ok) {
				onLeaveServer?.();
				onClose();
			} else {
				dangerError = 'failed to delete server';
			}
		} catch {
			dangerError = 'network error';
		} finally {
			leavingServer = false;
		}
	}

	async function loadMeta() {
		try {
			const res = await fetch(`${API_URL}/servers/meta?access_token=${accessToken}&server_id=${encodeURIComponent(serverId)}`);
			if (res.ok) {
				const data = await res.json();
				if (data.name) editName = data.name;
				editDescription = data.description || '';
				editVanitySlug = data.vanity_slug || '';
			}
		} catch { /* non-fatal */ }
	}

	async function saveMeta() {
		savingMeta = true;
		metaError = '';
		metaSuccess = '';
		try {
			const res = await fetch(`${API_URL}/servers/meta`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					server_id: serverId,
					name: editName.trim() || undefined,
					description: editDescription || undefined,
					vanity_slug: editVanitySlug || undefined,
				}),
			});
			if (res.ok) {
				metaSuccess = 'saved';
				if (editName.trim()) onNameChanged?.(editName.trim());
				setTimeout(() => { metaSuccess = ''; }, 2000);
			} else {
				metaError = 'failed to save';
			}
		} catch {
			metaError = 'network error';
		} finally {
			savingMeta = false;
		}
	}

	async function loadMembers() {
		loadingMembers = true;
		try {
			const res = await fetch(`${API_URL}/rooms/members?access_token=${accessToken}&room_id=${encodeURIComponent(serverId)}`);
			if (res.ok) {
				const data = await res.json();
				members = (data.members || []).map((m: any) => ({
					user_id: m.user_id,
					display_name: m.display_name,
					power_level: m.power_level ?? 0,
				}));
			}
		} catch { /* non-fatal */ }
		loadingMembers = false;
	}

	async function loadInvite() {
		loadingInvite = true;
		try {
			const res = await fetch(`${API_URL}/servers/invite?access_token=${accessToken}&server_id=${encodeURIComponent(serverId)}`);
			if (res.ok) {
				inviteInfo = await res.json();
			}
		} catch { /* non-fatal */ }
		loadingInvite = false;
	}

	async function handleLeaveServer() {
		leavingServer = true;
		dangerError = '';
		try {
			const res = await fetch(`${API_URL}/rooms/leave`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ access_token: accessToken, room_id: serverId }),
			});
			if (res.ok) {
				onLeaveServer?.();
				onClose();
			} else {
				dangerError = 'failed to leave server';
			}
		} catch {
			dangerError = 'network error';
		} finally {
			leavingServer = false;
		}
	}

	function copyInvite() {
		if (!inviteInfo) return;
		const text = inviteInfo.vanity_slug
			? `agora.gg/${inviteInfo.vanity_slug}`
			: inviteInfo.alias;
		navigator.clipboard.writeText(text).catch(() => {});
		copied = true;
		setTimeout(() => { copied = false; }, 2000);
	}

	// load data when tab changes
	$effect(() => {
		if (activeTab === 'overview') loadMeta();
		if (activeTab === 'members') loadMembers();
		if (activeTab === 'invites') loadInvite();
	});

	// check owner status once on mount
	$effect(() => {
		checkOwner();
	});

	const TAB_LABELS: [Tab, string][] = [
		['overview', 'overview'],
		['roles', 'roles'],
		['members', 'members'],
		['invites', 'invite'],
	];
</script>

{#if serverId}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
		<div
			class="bg-card rounded-xl shadow-2xl w-[720px] h-[520px] flex overflow-hidden"
			onclick={(e) => e.stopPropagation()}
		>
			<!-- left sidebar: tabs + danger zone -->
			<div class="w-44 bg-muted/30 border-r border-border flex flex-col flex-shrink-0">
				<div class="p-4 border-b border-border">
					<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider truncate">{serverName || 'server'}</p>
				</div>

				<nav class="flex-1 p-2 space-y-0.5">
					{#each TAB_LABELS as [id, label] (id)}
						<button
							class="w-full text-left px-3 py-2 rounded text-sm transition-colors"
							class:bg-muted={activeTab === id}
							class:text-card-foreground={activeTab === id}
							class:text-muted-foreground={activeTab !== id}
							onclick={() => activeTab = id}
						>
							{label}
						</button>
					{/each}
				</nav>

				<div class="p-3 border-t border-border space-y-1">
					<button
						class="w-full text-left px-3 py-2 rounded text-sm text-destructive hover:bg-destructive/10 transition-colors"
						onclick={() => showLeaveConfirm = true}
					>
						{isOwner ? 'delete server' : 'leave server'}
					</button>
					<button
						class="w-full text-left px-3 py-2 rounded text-sm text-muted-foreground hover:bg-muted transition-colors"
						onclick={onClose}
					>
						close
					</button>
				</div>
			</div>

			<!-- right content area -->
			<div class="flex-1 flex flex-col min-w-0">
				<!-- tab header -->
				<div class="px-6 pt-5 pb-4 border-b border-border flex-shrink-0">
					<h2 class="text-lg font-semibold text-card-foreground capitalize">{activeTab}</h2>
				</div>

				<div class="flex-1 overflow-hidden">

					<!-- overview tab -->
					{#if activeTab === 'overview'}
						<ScrollArea class="h-full">
							<div class="p-6 space-y-5">
								{#if metaError}
									<div class="text-destructive text-sm">{metaError}</div>
								{/if}
								{#if metaSuccess}
									<div class="text-primary text-sm">{metaSuccess}</div>
								{/if}

								<div class="space-y-1.5">
									<label for="s-name" class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">server name</label>
									<Input id="s-name" type="text" bind:value={editName} class="bg-muted border-input" />
								</div>

								<div class="space-y-1.5">
									<label for="s-desc" class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">description</label>
									<textarea
										id="s-desc"
										class="w-full rounded-md border border-input bg-muted p-2 text-sm text-card-foreground resize-none h-20 focus:outline-none focus:ring-1 focus:ring-ring"
										placeholder="what is this server about?"
										bind:value={editDescription}
									></textarea>
								</div>

								<div class="space-y-1.5">
									<label for="s-slug" class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
										vanity slug <span class="normal-case font-normal text-muted-foreground">(agora.gg/your-slug)</span>
									</label>
									<Input
										id="s-slug"
										type="text"
										placeholder="my-server"
										bind:value={editVanitySlug}
										class="bg-muted border-input"
									/>
									<p class="text-xs text-muted-foreground">3–32 chars, letters, numbers, hyphens only</p>
								</div>

								<Button onclick={saveMeta} disabled={savingMeta}>
									{savingMeta ? 'saving...' : 'save changes'}
								</Button>
							</div>
						</ScrollArea>

					<!-- roles tab -->
					{:else if activeTab === 'roles'}
						<div class="h-full">
							<RolesEditor {serverId} {accessToken} onClose={() => {}} />
						</div>

					<!-- members tab -->
					{:else if activeTab === 'members'}
						<div class="h-full overflow-y-auto p-6">
							{#if loadingMembers}
								<div class="text-center text-muted-foreground py-8">loading...</div>
							{:else if members.length === 0}
								<div class="text-center text-muted-foreground py-8">no members found</div>
							{:else}
								<div class="space-y-2">
									{#each members as member (member.user_id)}
										<div class="flex items-center gap-3 p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors">
											<div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-sm font-semibold text-muted-foreground flex-shrink-0">
												{((member.display_name || member.user_id)[0] || '?').toUpperCase()}
											</div>
											<div class="flex-1 min-w-0">
												<p class="text-sm font-medium text-card-foreground truncate">
													{member.display_name || member.user_id.split(':')[0].replace('@', '')}
												</p>
												<p class="text-xs text-muted-foreground">{member.user_id}</p>
											</div>
											<span class="text-xs text-muted-foreground flex-shrink-0">
												{member.power_level >= 100 ? 'owner' : member.power_level >= 50 ? 'admin' : member.power_level >= 25 ? 'mod' : 'member'}
											</span>
										</div>
									{/each}
								</div>
							{/if}
						</div>

					<!-- invites tab -->
					{:else if activeTab === 'invites'}
						<div class="p-6 space-y-5">
							{#if loadingInvite}
								<div class="text-center text-muted-foreground py-8">loading...</div>
							{:else if inviteInfo}
								<div class="space-y-1.5">
									<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">server alias</p>
									<div class="flex items-center gap-2">
										<code class="flex-1 bg-muted rounded px-3 py-2 text-sm text-card-foreground font-mono">
											{inviteInfo.alias}
										</code>
										<Button size="sm" variant="outline" onclick={copyInvite}>
											{copied ? 'copied!' : 'copy'}
										</Button>
									</div>
								</div>

								{#if inviteInfo.vanity_slug}
									<div class="space-y-1.5">
										<p class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">vanity link</p>
										<div class="flex items-center gap-2">
											<code class="flex-1 bg-muted rounded px-3 py-2 text-sm text-card-foreground font-mono">
												agora.gg/{inviteInfo.vanity_slug}
											</code>
											<Button size="sm" variant="outline" onclick={copyInvite}>
												{copied ? 'copied!' : 'copy'}
											</Button>
										</div>
									</div>
								{/if}

								<div class="flex items-center gap-4 pt-2">
									<div class="text-center">
										<p class="text-2xl font-bold text-card-foreground">{inviteInfo.member_count}</p>
										<p class="text-xs text-muted-foreground">members</p>
									</div>
								</div>

								<p class="text-xs text-muted-foreground">
									share the alias or vanity link so others can join. they can paste it in the "join server" field.
								</p>
							{:else}
								<div class="text-center text-muted-foreground py-8">could not load invite info</div>
							{/if}
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>

	<!-- leave / delete confirmation -->
	{#if showLeaveConfirm}
		<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]">
			<div class="bg-card rounded-lg p-6 w-80 space-y-4">
				{#if isOwner}
					<h3 class="font-semibold text-card-foreground">delete server?</h3>
					<p class="text-muted-foreground text-sm">
						this will kick all members and permanently destroy the server. this cannot be undone.
					</p>
				{:else}
					<h3 class="font-semibold text-card-foreground">leave server?</h3>
					<p class="text-muted-foreground text-sm">are you sure? you can rejoin later with an invite.</p>
				{/if}
				{#if dangerError}
					<div class="text-destructive text-sm">{dangerError}</div>
				{/if}
				<div class="flex gap-2">
					<Button variant="outline" class="flex-1" onclick={() => showLeaveConfirm = false} disabled={leavingServer}>
						cancel
					</Button>
					<Button variant="destructive" class="flex-1" onclick={isOwner ? handleDeleteServer : handleLeaveServer} disabled={leavingServer}>
						{#if leavingServer}
							{isOwner ? 'deleting...' : 'leaving...'}
						{:else}
							{isOwner ? 'delete server' : 'leave'}
						{/if}
					</Button>
				</div>
			</div>
		</div>
	{/if}
{/if}
