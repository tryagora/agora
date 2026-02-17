<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';

	interface Props {
		onAuthSuccess: (userId: string, accessToken: string) => void;
	}

	let { onAuthSuccess }: Props = $props();

	let username = $state('');
	let password = $state('');
	let error = $state('');
	let success = $state('');
	let loading = $state(false);

	const API_URL = 'http://localhost:3000';

	async function register() {
		loading = true;
		error = '';
		success = '';
		
		try {
			const response = await fetch(`${API_URL}/register`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ username, password })
			});
			
			if (response.ok) {
				const data = await response.json();
				success = `registered! user: ${data.user_id}`;
				// auto login after registration
				onAuthSuccess(data.user_id, data.access_token);
			} else {
				error = 'registration failed';
			}
		} catch (e) {
			error = 'network error - is the api running?';
		} finally {
			loading = false;
		}
	}

	async function login() {
		loading = true;
		error = '';
		success = '';
		
		try {
			const response = await fetch(`${API_URL}/login`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ username, password })
			});
			
			if (response.ok) {
				const data = await response.json();
				success = `logged in! welcome ${data.user_id}`;
				onAuthSuccess(data.user_id, data.access_token);
			} else {
				error = 'login failed';
			}
		} catch (e) {
			error = 'network error - is the api running?';
		} finally {
			loading = false;
		}
	}
</script>

<Card class="w-full max-w-md">
	<CardHeader>
		<CardTitle class="text-2xl">agora</CardTitle>
		<CardDescription>sign in or create an account</CardDescription>
	</CardHeader>
	<CardContent class="space-y-4">
		{#if error}
			<div class="rounded-md bg-red-500/10 border border-red-500/20 p-3 text-sm text-red-400">
				{error}
			</div>
		{/if}
		
		{#if success}
			<div class="rounded-md bg-green-500/10 border border-green-500/20 p-3 text-sm text-green-400">
				{success}
			</div>
		{/if}
		
		<div class="space-y-2">
			<label for="username" class="text-sm font-medium">username</label>
			<Input
				id="username"
				type="text"
				placeholder="enter username"
				bind:value={username}
				disabled={loading}
			/>
		</div>
		
		<div class="space-y-2">
			<label for="password" class="text-sm font-medium">password</label>
			<Input
				id="password"
				type="password"
				placeholder="enter password"
				bind:value={password}
				disabled={loading}
			/>
		</div>
		
		<div class="flex gap-2 pt-2">
			<Button 
				variant="outline" 
				class="flex-1"
				onclick={register}
				disabled={loading || !username || !password}
			>
				{loading ? 'loading...' : 'register'}
			</Button>
			<Button 
				class="flex-1"
				onclick={login}
				disabled={loading || !username || !password}
			>
				{loading ? 'loading...' : 'login'}
			</Button>
		</div>
	</CardContent>
</Card>
