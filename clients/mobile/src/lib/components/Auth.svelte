<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import Grainient from './Grainient.svelte';

	interface Props {
		apiUrl?: string;
		onAuthSuccess: (userId: string, accessToken: string) => void;
	}

	let { apiUrl = 'http://localhost:3000', onAuthSuccess }: Props = $props();

	let username = $state('');
	let password = $state('');
	let error = $state('');
	let success = $state('');
	let loading = $state(false);

	// use the configured api url — reactive so if it changes the requests update
	const API_URL = $derived(apiUrl);

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

<!-- full-screen container with animated WebGL gradient background -->
<div class="relative h-full w-full flex items-center justify-center p-4">
	<!-- animated background layer -->
	<div class="absolute inset-0 z-0">
		<Grainient
			color1="#5c4033"
			color2="#2d1b0e"
			color3="#8b6f47"
			timeSpeed={0.18}
			warpStrength={0.8}
			warpFrequency={4.0}
			warpAmplitude={60.0}
			rotationAmount={300.0}
			grainAmount={0.08}
			contrast={1.3}
			saturation={0.9}
			zoom={0.85}
		/>
	</div>

	<!-- auth card on top -->
	<div class="relative z-10 w-full max-w-md">
		<Card class="w-full bg-card/80 backdrop-blur-sm border-border/60 shadow-2xl">
			<CardHeader>
				<CardTitle class="text-2xl text-card-foreground">agora</CardTitle>
				<CardDescription class="text-muted-foreground">sign in or create an account</CardDescription>
			</CardHeader>
			<CardContent class="space-y-4">
				{#if error}
					<div class="rounded-md bg-destructive/10 border border-destructive/20 p-3 text-sm text-destructive">
						{error}
					</div>
				{/if}
				
				{#if success}
					<div class="rounded-md bg-primary/10 border border-primary/20 p-3 text-sm text-primary">
						{success}
					</div>
				{/if}
				
				<div class="space-y-2">
					<label for="username" class="text-sm font-medium text-card-foreground">username</label>
					<Input
						id="username"
						type="text"
						placeholder="enter username"
						bind:value={username}
						disabled={loading}
						class="bg-muted border-input text-foreground placeholder:text-muted-foreground"
					/>
				</div>
				
				<div class="space-y-2">
					<label for="password" class="text-sm font-medium text-card-foreground">password</label>
					<Input
						id="password"
						type="password"
						placeholder="enter password"
						bind:value={password}
						disabled={loading}
						class="bg-muted border-input text-foreground placeholder:text-muted-foreground"
					/>
				</div>
				
				<div class="flex gap-2 pt-2">
					<Button 
						variant="outline" 
						class="flex-1 border-border text-card-foreground hover:bg-muted"
						onclick={register}
						disabled={loading || !username || !password}
					>
						{loading ? 'loading...' : 'register'}
					</Button>
					<Button 
						class="flex-1 bg-primary text-primary-foreground hover:bg-primary/90"
						onclick={login}
						disabled={loading || !username || !password}
					>
						{loading ? 'loading...' : 'login'}
					</Button>
				</div>
			</CardContent>
		</Card>
	</div>
</div>
