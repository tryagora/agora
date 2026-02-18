<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { ScrollArea } from '$lib/components/ui/scroll-area';

	interface Member {
		user_id: string;
		display_name: string | null;
		power_level: number;
	}

	interface Props {
		serverId: string | null;
		serverName: string | null;
		accessToken: string;
		userId: string;
		onClose: () => void;
		onLeaveServer?: () => void;
	}

	let { serverId, serverName, accessToken, userId, onClose, onLeaveServer }: Props = $props();
	
	let newServerName = $state('');
	let members = $state<Member[]>([]);
	let permissions = $state<Record<string, number>>({});
	let defaultPower = $state(0);
	let loading = $state(false);
	let error = $state('');
	let success = $state('');
	let showLeaveConfirm = $state(false);
	let showPermissionsEditor = $state(false);
	let selectedMember = $state<Member | null>(null);
	let newPowerLevel = $state(0);

	const API_URL = 'http://localhost:3000';

	// Update when serverName changes
	$effect(() => {
		newServerName = serverName || '';
	});

	// Load permissions when dialog opens
	$effect(() => {
		if (serverId) {
			loadPermissions();
			loadMembers();
		}
	});

	async function loadPermissions() {
		if (!serverId) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/permissions?access_token=${accessToken}&room_id=${encodeURIComponent(serverId)}`);
			if (response.ok) {
				const data = await response.json();
				permissions = data.users || {};
				defaultPower = data.users_default || 0;
			}
		} catch (e) {
			console.error('failed to load permissions:', e);
		}
	}

	async function loadMembers() {
		if (!serverId) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/members?access_token=${accessToken}&room_id=${encodeURIComponent(serverId)}`);
			if (response.ok) {
				const data = await response.json();
				members = (data.members || []).map((m: any) => ({
					user_id: m.user_id,
					display_name: m.display_name,
					power_level: permissions[m.user_id] || defaultPower
				}));
			}
		} catch (e) {
			console.error('failed to load members:', e);
		}
	}

	async function handleUpdateName() {
		if (!newServerName.trim() || !serverId) return;
		
		loading = true;
		error = '';
		success = '';
		
		try {
			// This would need a backend endpoint
			success = 'server name updated';
		} catch (e) {
			error = 'failed to update';
		} finally {
			loading = false;
		}
	}

	async function handleUploadImage() {
		// Placeholder for image upload
		success = 'image upload coming soon';
	}

	async function handleLeaveServer() {
		if (!serverId) return;
		
		try {
			const response = await fetch(`${API_URL}/rooms/leave`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: serverId
				})
			});
			
			if (response.ok) {
				showLeaveConfirm = false;
				onLeaveServer?.();
				onClose();
			} else {
				error = 'failed to leave server';
			}
		} catch (e) {
			error = 'network error';
		}
	}

	async function handleUpdatePermissions() {
		if (!serverId || !selectedMember) return;
		
		loading = true;
		error = '';
		success = '';
		
		try {
			const response = await fetch(`${API_URL}/rooms/permissions`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: serverId,
					user_id: selectedMember.user_id,
					power_level: newPowerLevel
				})
			});
			
			if (response.ok) {
				success = 'permissions updated';
				showPermissionsEditor = false;
				selectedMember = null;
				await loadPermissions();
				await loadMembers();
			} else {
				error = 'failed to update permissions';
			}
		} catch (e) {
			error = 'network error';
		} finally {
			loading = false;
		}
	}

	function openPermissionsEditor(member: Member) {
		selectedMember = member;
		newPowerLevel = member.power_level;
		showPermissionsEditor = true;
	}

	function getPowerLevelName(level: number): string {
		if (level >= 100) return 'Owner';
		if (level >= 50) return 'Admin';
		if (level >= 25) return 'Moderator';
		return 'Member';
	}

	function isCurrentUserAdmin(): boolean {
		const myPower = permissions[userId] || defaultPower;
		return myPower >= 50;
	}
</script>

{#if serverId}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
		<div class="bg-card p-6 rounded-lg w-96 max-h-[80vh] overflow-y-auto" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between mb-6">
				<h3 class="text-lg font-semibold text-card-foreground">manage server</h3>
			<button 
				class="text-muted-foreground hover:text-card-foreground"
				onclick={onClose}
				aria-label="close"
				type="button"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</button>
			</div>

			{#if error}
				<div class="text-destructive text-sm mb-3">{error}</div>
			{/if}
			
			{#if success}
				<div class="text-primary text-sm mb-3">{success}</div>
			{/if}

			<div class="space-y-4">
			<!-- Server Image -->
			<div class="space-y-2">
				<span class="text-sm font-medium text-card-foreground">server image</span>
				<div class="flex items-center gap-4">
					<div class="w-16 h-16 rounded-full bg-secondary flex items-center justify-center text-xl font-bold text-secondary-foreground">
						{serverName?.[0]?.toUpperCase() || '?'}
					</div>
					<Button variant="outline" onclick={handleUploadImage} disabled={loading}>
						upload image
					</Button>
				</div>
				<p class="text-xs text-muted-foreground">custom server images coming soon</p>
			</div>

			<!-- Server Name -->
			<div class="space-y-2">
				<label for="server-name" class="text-sm font-medium text-card-foreground">server name</label>
				<Input
					id="server-name"
					type="text"
					bind:value={newServerName}
					class="bg-muted border-input"
				/>
					<Button 
						variant="outline" 
						class="w-full"
						onclick={handleUpdateName}
						disabled={loading || !newServerName.trim()}
					>
						update name
					</Button>
				</div>

				<!-- Permissions Section -->
				{#if members.length > 0}
					<div class="space-y-2 pt-4 border-t border-border">
						<h4 class="text-sm font-medium text-card-foreground">members & permissions</h4>
						<ScrollArea class="h-40 border rounded-md">
							<div class="p-2 space-y-1">
								{#each members as member}
									<div class="flex items-center justify-between p-2 rounded hover:bg-muted">
										<div class="flex flex-col">
											<span class="text-sm text-card-foreground">{member.display_name || member.user_id}</span>
											<span class="text-xs text-muted-foreground">{getPowerLevelName(member.power_level)} ({member.power_level})</span>
										</div>
										{#if isCurrentUserAdmin() && member.user_id !== userId}
											<Button 
												variant="ghost" 
												size="sm"
												onclick={() => openPermissionsEditor(member)}
											>
												edit
											</Button>
										{/if}
									</div>
								{/each}
							</div>
						</ScrollArea>
					</div>
				{/if}

				<!-- Danger Zone -->
				<div class="pt-4 border-t border-border">
					<h4 class="text-sm font-medium text-destructive mb-2">danger zone</h4>
					<Button 
						variant="outline" 
						class="w-full border-destructive text-destructive hover:bg-destructive/10"
						onclick={() => showLeaveConfirm = true}
					>
						leave server
					</Button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Leave confirmation dialog -->
{#if showLeaveConfirm}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]">
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

<!-- Permissions editor dialog -->
{#if showPermissionsEditor && selectedMember}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]">
		<div class="bg-card p-6 rounded-lg w-80">
			<h3 class="text-lg font-semibold mb-2 text-card-foreground">edit permissions</h3>
			<p class="text-muted-foreground text-sm mb-4">
				set power level for {selectedMember.display_name || selectedMember.user_id}
			</p>
			
			<div class="space-y-3 mb-4">
				<label class="block text-sm font-medium text-card-foreground">power level</label>
				<Input
					type="number"
					bind:value={newPowerLevel}
					min="0"
					max="100"
					class="mb-2"
				/>
				<div class="flex gap-2">
					<Button variant="outline" size="sm" onclick={() => newPowerLevel = 0}>member (0)</Button>
					<Button variant="outline" size="sm" onclick={() => newPowerLevel = 25}>moderator (25)</Button>
					<Button variant="outline" size="sm" onclick={() => newPowerLevel = 50}>admin (50)</Button>
				</div>
			</div>

			{#if error}
				<div class="text-destructive text-sm mb-3">{error}</div>
			{/if}

			<div class="flex gap-2">
				<Button variant="outline" class="flex-1" onclick={() => { showPermissionsEditor = false; selectedMember = null; }}>
					cancel
				</Button>
				<Button class="flex-1" onclick={handleUpdatePermissions} disabled={loading}>
					save
				</Button>
			</div>
		</div>
	</div>
{/if}
