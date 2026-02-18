<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import Grainient from './Grainient.svelte';

	interface Props {
		onComplete: (serverUrl: string) => void;
	}

	let { onComplete }: Props = $props();

	// steps: welcome, notifications, server, done
	let step = $state<'welcome' | 'notifications' | 'server' | 'done'>('welcome');
	let serverUrl = $state('http://localhost:3000');
	let notifGranted = $state(false);
	let notifDenied = $state(false);

	async function requestNotifications() {
		if (!('Notification' in window)) {
			// browser doesn't support notifications — skip silently
			notifDenied = true;
			return;
		}
		const perm = await Notification.requestPermission();
		if (perm === 'granted') {
			notifGranted = true;
		} else {
			notifDenied = true;
		}
		step = 'server';
	}

	function skipNotifications() {
		notifDenied = true;
		step = 'server';
	}

	function finish() {
		// mark onboarding done so we never show it again
		localStorage.setItem('agora_onboarding_done', '1');
		// persist server url preference
		localStorage.setItem('agora_server_url', serverUrl.trim() || 'http://localhost:3000');
		onComplete(serverUrl.trim() || 'http://localhost:3000');
	}
</script>

<div class="relative h-full w-full flex items-center justify-center p-4">
	<!-- same animated background as auth -->
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

	<div class="relative z-10 w-full max-w-md">
		<div class="bg-card/80 backdrop-blur-sm border border-border/60 rounded-xl shadow-2xl p-8 space-y-6">

			<!-- step: welcome -->
			{#if step === 'welcome'}
				<div class="space-y-3 text-center">
					<h1 class="text-3xl font-bold text-card-foreground">welcome to agora</h1>
					<p class="text-muted-foreground text-sm leading-relaxed">
						a free, open-source, federated place to talk.<br />
						let's get you set up in a few quick steps.
					</p>
				</div>
				<div class="pt-2">
					<Button
						class="w-full bg-primary text-primary-foreground hover:bg-primary/90"
						onclick={() => step = 'notifications'}
					>
						get started
					</Button>
				</div>

			<!-- step: notifications -->
			{:else if step === 'notifications'}
				<div class="space-y-3 text-center">
					<div class="text-4xl">🔔</div>
					<h2 class="text-xl font-semibold text-card-foreground">stay in the loop</h2>
					<p class="text-muted-foreground text-sm leading-relaxed">
						agora can notify you when new messages arrive, even when the window is in the background.
					</p>
				</div>
				<div class="space-y-2">
					<Button
						class="w-full bg-primary text-primary-foreground hover:bg-primary/90"
						onclick={requestNotifications}
					>
						enable notifications
					</Button>
					<Button
						variant="outline"
						class="w-full border-border text-card-foreground hover:bg-muted"
						onclick={skipNotifications}
					>
						skip for now
					</Button>
				</div>

			<!-- step: server URL -->
			{:else if step === 'server'}
				<div class="space-y-3">
					<div class="text-center">
						<h2 class="text-xl font-semibold text-card-foreground">server address</h2>
						<p class="text-muted-foreground text-sm mt-1">
							where is your agora API running?
						</p>
					</div>

					{#if notifGranted}
						<div class="rounded-md bg-primary/10 border border-primary/20 p-2 text-xs text-primary text-center">
							notifications enabled
						</div>
					{:else if notifDenied}
						<div class="rounded-md bg-muted border border-border p-2 text-xs text-muted-foreground text-center">
							notifications disabled — you can enable them in browser settings later
						</div>
					{/if}

					<div class="space-y-1">
						<label for="server-url" class="text-sm font-medium text-card-foreground">api url</label>
						<Input
							id="server-url"
							type="url"
							placeholder="http://localhost:3000"
							bind:value={serverUrl}
							class="bg-muted border-input text-foreground placeholder:text-muted-foreground"
						/>
						<p class="text-xs text-muted-foreground">
							leave as-is if running locally
						</p>
					</div>
				</div>
				<Button
					class="w-full bg-primary text-primary-foreground hover:bg-primary/90"
					onclick={() => step = 'done'}
					disabled={!serverUrl.trim()}
				>
					continue
				</Button>

			<!-- step: done -->
			{:else if step === 'done'}
				<div class="space-y-3 text-center">
					<div class="text-4xl">✓</div>
					<h2 class="text-xl font-semibold text-card-foreground">all set!</h2>
					<p class="text-muted-foreground text-sm leading-relaxed">
						agora is ready to use.<br />
						create an account or sign in to get started.
					</p>
				</div>
				<Button
					class="w-full bg-primary text-primary-foreground hover:bg-primary/90"
					onclick={finish}
				>
					go to sign in
				</Button>
			{/if}

			<!-- step indicator dots -->
			<div class="flex justify-center gap-2 pt-2">
				{#each ['welcome', 'notifications', 'server', 'done'] as s}
					<div class="h-1.5 rounded-full transition-all duration-300 {step === s ? 'w-6 bg-primary' : 'w-1.5 bg-muted-foreground/30'}"></div>
				{/each}
			</div>
		</div>
	</div>
</div>
