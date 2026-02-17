<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';

	interface Message {
		room_id: string;
		sender: string;
		content: string;
		timestamp?: number;
	}

	interface Props {
		userId: string;
		accessToken: string;
		onLogout: () => void;
	}

	let { userId, accessToken, onLogout }: Props = $props();

	let messages = $state<Message[]>([]);
	let newMessage = $state('');
	let roomId = $state('');
	let nextBatch = $state('');
	let loading = $state(false);
	let error = $state('');

	const API_URL = 'http://localhost:3000';

	async function sync() {
		try {
			const params = new URLSearchParams({ access_token: accessToken });
			if (nextBatch) {
				params.append('since', nextBatch);
			}
			
			const response = await fetch(`${API_URL}/sync?${params}`);
			if (response.ok) {
				const data = await response.json();
				nextBatch = data.next_batch;
				if (data.messages && data.messages.length > 0) {
					messages = [...messages, ...data.messages];
				}
			}
		} catch (e) {
			console.error('sync failed:', e);
		}
	}

	async function sendMessage() {
		if (!newMessage.trim() || !roomId.trim()) return;
		
		loading = true;
		error = '';
		
		try {
			messages = [...messages, {
				room_id: roomId,
				sender: userId,
				content: newMessage,
				timestamp: Date.now()
			}];
			newMessage = '';
		} catch (e) {
			error = 'failed to send message';
		} finally {
			loading = false;
		}
	}

	// poll for new messages every 5 seconds
	$effect(() => {
		const interval = setInterval(sync, 5000);
		sync(); // initial sync
		return () => clearInterval(interval);
	});

	function formatTimestamp(ts?: number): string {
		if (!ts) return '';
		return new Date(ts).toLocaleTimeString();
	}
</script>

<div class="flex h-screen max-h-screen gap-4 p-4">
	<!-- sidebar -->
	<Card class="w-64 flex-shrink-0">
		<CardHeader>
			<CardTitle class="text-lg">{userId}</CardTitle>
			<CardDescription>connected</CardDescription>
		</CardHeader>
		<CardContent>
			<Button variant="outline" class="w-full" onclick={onLogout}>
				logout
			</Button>
		</CardContent>
	</Card>

	<!-- chat area -->
	<Card class="flex-1 flex flex-col min-h-0">
		<CardHeader class="flex-shrink-0">
			<CardTitle>chat</CardTitle>
			<CardDescription>
				{messages.length} messages
				{#if nextBatch}
					• synced
				{/if}
			</CardDescription>
		</CardHeader>
		
		<CardContent class="flex-1 flex flex-col min-h-0 space-y-4">
			<!-- room selector -->
			<div class="flex-shrink-0">
				<Input
					type="text"
					placeholder="room id (e.g., !room:localhost)"
					bind:value={roomId}
				/>
			</div>

			<!-- messages -->
			<div class="flex-1 border rounded-md p-4 overflow-y-auto min-h-0 space-y-3">
				{#if messages.length === 0}
					<p class="text-muted-foreground text-center">no messages yet</p>
				{:else}
					{#each messages as message (message.timestamp)}
						<div class="space-y-1">
							<div class="flex items-center gap-2">
								<span class="font-semibold text-sm">{message.sender}</span>
								<span class="text-xs text-muted-foreground">
									{formatTimestamp(message.timestamp)}
								</span>
							</div>
							<p class="text-sm">{message.content}</p>
						</div>
					{/each}
				{/if}
			</div>

			<!-- message input -->
			{#if error}
				<div class="text-sm text-red-400 flex-shrink-0">{error}</div>
			{/if}
			
			<div class="flex gap-2 flex-shrink-0">
				<Input
					type="text"
					placeholder="type a message..."
					bind:value={newMessage}
					onkeydown={(e) => e.key === 'Enter' && sendMessage()}
					disabled={loading}
					class="flex-1"
				/>
				<Button 
					onclick={sendMessage}
					disabled={loading || !newMessage.trim()}
				>
					send
				</Button>
			</div>
		</CardContent>
	</Card>
</div>
