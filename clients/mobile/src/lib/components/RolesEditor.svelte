<script lang="ts">
	// roles editor — list, create, edit, and delete roles for a server
	// each role maps to a Matrix power level and carries a flat permission flags object

	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	interface RolePermissions {
		send_messages: boolean;
		manage_channels: boolean;
		manage_roles: boolean;
		kick_members: boolean;
		ban_members: boolean;
		mention_everyone: boolean;
		manage_server: boolean;
		administrator: boolean;
	}

	interface Role {
		id: string;
		name: string;
		color: string;
		hoist: boolean;
		mentionable: boolean;
		permissions: RolePermissions;
		power_level: number;
	}

	interface Props {
		serverId: string;
		accessToken: string;
		onClose: () => void;
	}

	let { serverId, accessToken, onClose }: Props = $props();

	const API_URL = 'http://localhost:3000';

	let roles = $state<Role[]>([]);
	let selectedRole = $state<Role | null>(null);
	let loading = $state(false);
	let saving = $state(false);
	let error = $state('');

	// editing state
	let editName = $state('');
	let editColor = $state('#5865f2');
	let editHoist = $state(false);
	let editMentionable = $state(false);
	let editPowerLevel = $state(0);
	let editPerms = $state<RolePermissions>({
		send_messages: true,
		manage_channels: false,
		manage_roles: false,
		kick_members: false,
		ban_members: false,
		mention_everyone: false,
		manage_server: false,
		administrator: false,
	});

	const PERM_LABELS: [keyof RolePermissions, string][] = [
		['send_messages', 'send messages'],
		['manage_channels', 'manage channels'],
		['manage_roles', 'manage roles'],
		['kick_members', 'kick members'],
		['ban_members', 'ban members'],
		['mention_everyone', 'mention @everyone'],
		['manage_server', 'manage server'],
		['administrator', 'administrator (all)'],
	];

	const PRESET_COLORS = ['#5865f2', '#57f287', '#fee75c', '#eb459e', '#ed4245', '#ffffff', '#99aab5'];

	async function loadRoles() {
		loading = true;
		error = '';
		try {
			const res = await fetch(`${API_URL}/servers/roles?access_token=${accessToken}&server_id=${encodeURIComponent(serverId)}`);
			if (res.ok) {
				const data = await res.json();
				roles = data.roles || [];
			}
		} catch {
			error = 'failed to load roles';
		} finally {
			loading = false;
		}
	}

	function selectRole(role: Role) {
		selectedRole = role;
		editName = role.name;
		editColor = role.color;
		editHoist = role.hoist;
		editMentionable = role.mentionable;
		editPowerLevel = role.power_level;
		editPerms = { ...role.permissions };
	}

	function createNewRole() {
		const newRole: Role = {
			id: crypto.randomUUID(),
			name: 'new role',
			color: '#99aab5',
			hoist: false,
			mentionable: false,
			power_level: 0,
			permissions: {
				send_messages: true,
				manage_channels: false,
				manage_roles: false,
				kick_members: false,
				ban_members: false,
				mention_everyone: false,
				manage_server: false,
				administrator: false,
			},
		};
		roles = [...roles, newRole];
		selectRole(newRole);
	}

	async function saveRole() {
		if (!selectedRole || !editName.trim()) return;
		saving = true;
		error = '';

		const updated: Role = {
			...selectedRole,
			name: editName.trim(),
			color: editColor,
			hoist: editHoist,
			mentionable: editMentionable,
			power_level: editPowerLevel,
			permissions: { ...editPerms },
		};

		const updatedRoles = roles.map(r => r.id === updated.id ? updated : r);

		try {
			const res = await fetch(`${API_URL}/servers/roles`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					server_id: serverId,
					roles: updatedRoles,
				}),
			});
			if (res.ok) {
				roles = updatedRoles;
				selectedRole = updated;
			} else {
				error = 'failed to save role';
			}
		} catch {
			error = 'network error';
		} finally {
			saving = false;
		}
	}

	async function deleteRole(role: Role) {
		const updatedRoles = roles.filter(r => r.id !== role.id);
		try {
			const res = await fetch(`${API_URL}/servers/roles`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					server_id: serverId,
					roles: updatedRoles,
				}),
			});
			if (res.ok) {
				roles = updatedRoles;
				if (selectedRole?.id === role.id) selectedRole = null;
			} else {
				error = 'failed to delete role';
			}
		} catch {
			error = 'network error';
		}
	}

	$effect(() => {
		loadRoles();
	});
</script>

<div class="flex h-full gap-0 min-h-0">
	<!-- role list sidebar -->
	<div class="w-48 flex-shrink-0 border-r border-border flex flex-col bg-muted/30">
		<div class="p-3 border-b border-border flex items-center justify-between">
			<span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">roles</span>
			<button
				class="w-5 h-5 rounded flex items-center justify-center text-muted-foreground hover:text-card-foreground hover:bg-muted transition-colors text-sm"
				onclick={createNewRole}
				title="create role"
				aria-label="create role"
				type="button"
			>+</button>
		</div>
		<div class="flex-1 overflow-y-auto p-2 space-y-1">
			{#if loading}
				<div class="text-xs text-muted-foreground text-center py-4">loading...</div>
			{:else if roles.length === 0}
				<div class="text-xs text-muted-foreground text-center py-4">no roles yet</div>
			{:else}
				{#each roles as role (role.id)}
					<button
						class="w-full text-left px-3 py-2 rounded flex items-center gap-2 transition-colors"
						class:bg-muted={selectedRole?.id === role.id}
						class:text-card-foreground={selectedRole?.id === role.id}
						onclick={() => selectRole(role)}
					>
						<span class="w-2.5 h-2.5 rounded-full flex-shrink-0" style="background:{role.color}"></span>
						<span class="truncate text-sm">{role.name}</span>
					</button>
				{/each}
			{/if}
		</div>
	</div>

	<!-- role editor panel -->
	<div class="flex-1 overflow-y-auto p-5 space-y-5">
		{#if !selectedRole}
			<div class="text-muted-foreground text-sm text-center py-8">
				select a role to edit, or create one
			</div>
		{:else}
			<div class="flex items-center justify-between">
				<h3 class="font-semibold text-card-foreground">editing: {selectedRole.name}</h3>
				<button
					class="text-xs text-destructive hover:underline"
					onclick={() => deleteRole(selectedRole!)}
					type="button"
				>delete role</button>
			</div>

			{#if error}
				<div class="text-destructive text-sm">{error}</div>
			{/if}

			<!-- name -->
			<div class="space-y-1.5">
				<label class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">role name</label>
				<Input type="text" bind:value={editName} class="bg-muted border-input" />
			</div>

			<!-- color -->
			<div class="space-y-1.5">
				<label class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">color</label>
				<div class="flex items-center gap-2 flex-wrap">
					{#each PRESET_COLORS as c (c)}
						<button
							class="w-6 h-6 rounded-full border-2 transition-all"
							class:border-card-foreground={editColor === c}
							class:border-transparent={editColor !== c}
							style="background:{c}"
							onclick={() => editColor = c}
							aria-label={c}
							type="button"
						></button>
					{/each}
					<!-- custom color picker -->
					<input
						type="color"
						bind:value={editColor}
						class="w-6 h-6 rounded cursor-pointer border border-border bg-transparent"
						title="custom color"
					/>
				</div>
			</div>

			<!-- power level -->
			<div class="space-y-1.5">
				<label class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
					matrix power level <span class="text-muted-foreground normal-case font-normal">(0–100)</span>
				</label>
				<Input type="number" bind:value={editPowerLevel} min={0} max={100} class="bg-muted border-input w-24" />
				<p class="text-xs text-muted-foreground">higher = more permissions in Matrix. admin roles need 100.</p>
			</div>

			<!-- display options -->
			<div class="space-y-2">
				<label class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">display</label>
				<label class="flex items-center gap-2 cursor-pointer">
					<input type="checkbox" bind:checked={editHoist} class="rounded" />
					<span class="text-sm text-card-foreground">display role members separately</span>
				</label>
				<label class="flex items-center gap-2 cursor-pointer">
					<input type="checkbox" bind:checked={editMentionable} class="rounded" />
					<span class="text-sm text-card-foreground">allow anyone to mention this role</span>
				</label>
			</div>

			<!-- permissions -->
			<div class="space-y-2">
				<label class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">permissions</label>
				<div class="space-y-1.5">
					{#each PERM_LABELS as [key, label] (key)}
						<label class="flex items-center gap-2 cursor-pointer">
							<input
								type="checkbox"
								bind:checked={editPerms[key]}
								class="rounded"
								disabled={key !== 'administrator' && editPerms.administrator}
							/>
							<span class="text-sm text-card-foreground">{label}</span>
							{#if key === 'administrator'}
								<span class="ml-auto text-xs text-muted-foreground">overrides all</span>
							{/if}
						</label>
					{/each}
				</div>
			</div>

			<div class="flex gap-2 pt-2">
				<Button variant="outline" onclick={() => selectedRole = null}>cancel</Button>
				<Button onclick={saveRole} disabled={saving || !editName.trim()}>
					{saving ? 'saving...' : 'save role'}
				</Button>
			</div>
		{/if}
	</div>
</div>
