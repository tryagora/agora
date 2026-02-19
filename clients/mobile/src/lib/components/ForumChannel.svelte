<script lang="ts">
	// forum channel — thread grid view with thread-level chat
	// each thread is a Matrix room linked via m.space.child on the forum channel room

	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	interface Thread {
		room_id: string;
		title: string;
		author: string;
		created_at: number | null;
		reply_count: number | null;
		pinned: boolean;
	}

	interface Message {
		room_id: string;
		sender: string;
		content: string;
		timestamp?: number;
		event_id?: string;
	}

	interface Props {
		forumChannelId: string;
		forumChannelName: string;
		accessToken: string;
		userId: string;
		apiUrl?: string;
	}

	let { forumChannelId, forumChannelName, accessToken, userId, apiUrl = 'http://localhost:3000' }: Props = $props();

	const API_URL = $derived(apiUrl);

	let threads = $state<Thread[]>([]);
	let loadingThreads = $state(true);
	let threadError = $state('');

	// active thread view
	let activeThread = $state<Thread | null>(null);
	let threadMessages = $state<Message[]>([]);
	let loadingMessages = $state(false);
	let newReply = $state('');
	let sendingReply = $state(false);
	let messagesContainer: HTMLDivElement | undefined;

	// new thread dialog
	let showNewThread = $state(false);
	let newThreadTitle = $state('');
	let newThreadBody = $state('');
	let creatingThread = $state(false);
	let createError = $state('');

	async function loadThreads() {
		loadingThreads = true;
		threadError = '';
		try {
			const res = await fetch(
				`${API_URL}/servers/forum/threads?access_token=${accessToken}&forum_channel_id=${encodeURIComponent(forumChannelId)}`
			);
			if (res.ok) {
				const data = await res.json();
				threads = data.threads || [];
			} else {
				threadError = 'failed to load threads';
			}
		} catch {
			threadError = 'network error';
		} finally {
			loadingThreads = false;
		}
	}

	async function openThread(thread: Thread) {
		activeThread = thread;
		loadingMessages = true;
		threadMessages = [];
		try {
			// use the generic sync/messages endpoint to load messages in the thread room
			const res = await fetch(`${API_URL}/rooms/messages?access_token=${accessToken}&room_id=${encodeURIComponent(thread.room_id)}`);
			if (res.ok) {
				const data = await res.json();
				threadMessages = data.messages || [];
			}
		} catch {
			// non-fatal — thread might just be empty
		} finally {
			loadingMessages = false;
		}
	}

	async function sendReply() {
		if (!newReply.trim() || !activeThread) return;
		sendingReply = true;
		try {
			const res = await fetch(`${API_URL}/rooms/send`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: activeThread.room_id,
					content: newReply.trim(),
				}),
			});
			if (res.ok) {
				newReply = '';
				// optimistically add message
				threadMessages = [...threadMessages, {
					room_id: activeThread.room_id,
					sender: userId,
					content: newReply,
					timestamp: Date.now(),
				}];
				// scroll to bottom
				setTimeout(() => {
					if (messagesContainer) messagesContainer.scrollTop = messagesContainer.scrollHeight;
				}, 50);
			}
		} catch {
			// non-fatal
		} finally {
			sendingReply = false;
		}
	}

	async function createThread() {
		if (!newThreadTitle.trim() || !newThreadBody.trim()) return;
		creatingThread = true;
		createError = '';
		try {
			const res = await fetch(`${API_URL}/servers/forum/thread`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					forum_channel_id: forumChannelId,
					title: newThreadTitle.trim(),
					body: newThreadBody.trim(),
					author: userId,
				}),
			});
			if (res.ok) {
				showNewThread = false;
				newThreadTitle = '';
				newThreadBody = '';
				await loadThreads();
			} else {
				createError = 'failed to create thread';
			}
		} catch {
			createError = 'network error';
		} finally {
			creatingThread = false;
		}
	}

	function formatTime(ms: number | null): string {
		if (!ms) return '';
		const d = new Date(ms);
		return d.toLocaleDateString();
	}

	function authorShort(uid: string): string {
		return uid.split(':')[0].replace('@', '');
	}

	$effect(() => {
		const _id = forumChannelId;
		loadThreads();
	});

	// auto-scroll when messages arrive
	$effect(() => {
		const _len = threadMessages.length;
		if (messagesContainer) {
			requestAnimationFrame(() => {
				if (messagesContainer) messagesContainer.scrollTop = messagesContainer.scrollHeight;
			});
		}
	});
</script>

<div class="flex flex-col h-full min-h-0">
	<!-- header -->
	<div class="h-14 border-b border-border flex items-center justify-between px-4 bg-card flex-shrink-0">
		<div class="flex items-center gap-2">
			{#if activeThread}
				<button
					class="text-muted-foreground hover:text-card-foreground transition-colors"
					onclick={() => activeThread = null}
					type="button"
					aria-label="back to thread list"
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
					</svg>
				</button>
			{/if}
			<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
			</svg>
			<span class="font-semibold text-card-foreground">
				{activeThread ? activeThread.title : forumChannelName}
			</span>
			{#if activeThread}
				<span class="text-xs text-muted-foreground">by {authorShort(activeThread.author)}</span>
			{/if}
		</div>
		{#if !activeThread}
			<Button size="sm" onclick={() => showNewThread = true}>
				+ new post
			</Button>
		{/if}
	</div>

	<!-- thread grid -->
	{#if !activeThread}
		<div class="flex-1 overflow-y-auto p-4">
			{#if loadingThreads}
				<div class="text-center text-muted-foreground py-8">loading threads...</div>
			{:else if threadError}
				<div class="text-destructive text-sm text-center py-4">{threadError}</div>
			{:else if threads.length === 0}
				<div class="text-center py-16">
					<svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto text-muted-foreground mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
					</svg>
					<p class="text-muted-foreground text-sm">no posts yet</p>
					<button
						class="mt-3 text-primary text-sm hover:underline"
						onclick={() => showNewThread = true}
						type="button"
					>create the first post</button>
				</div>
			{:else}
				<div class="grid grid-cols-1 gap-3 max-w-2xl">
					{#each threads as thread (thread.room_id)}
						<button
							class="text-left bg-card rounded-lg border border-border hover:border-primary/50 hover:bg-card/80 transition-colors p-4"
							onclick={() => openThread(thread)}
						>
							<div class="flex items-start gap-3">
								<!-- pinned indicator -->
								{#if thread.pinned}
									<span class="text-xs text-primary flex-shrink-0 mt-0.5">📌</span>
								{/if}
								<div class="flex-1 min-w-0">
									<p class="font-semibold text-card-foreground truncate">{thread.title}</p>
									<div class="flex items-center gap-3 mt-1">
										<span class="text-xs text-muted-foreground">by {authorShort(thread.author)}</span>
										{#if thread.created_at}
											<span class="text-xs text-muted-foreground">{formatTime(thread.created_at)}</span>
										{/if}
										{#if thread.reply_count !== null}
											<span class="text-xs text-muted-foreground ml-auto">{thread.reply_count} replies</span>
										{/if}
									</div>
								</div>
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<!-- thread chat view -->
		<div class="flex-1 overflow-y-auto p-4 space-y-3 min-h-0" bind:this={messagesContainer}>
			{#if loadingMessages}
				<div class="text-center text-muted-foreground py-8">loading...</div>
			{:else if threadMessages.length === 0}
				<p class="text-center text-muted-foreground py-8">no replies yet — be the first!</p>
			{:else}
				{#each threadMessages as msg, i (msg.event_id || `local-${i}`)}
					<div class="space-y-0.5">
						<div class="flex items-center gap-2">
							<span class="text-sm font-semibold text-card-foreground">{authorShort(msg.sender)}</span>
							{#if msg.timestamp}
								<span class="text-xs text-muted-foreground">{new Date(msg.timestamp).toLocaleTimeString()}</span>
							{/if}
						</div>
						<p class="text-sm text-muted-foreground">{msg.content}</p>
					</div>
				{/each}
			{/if}
		</div>

		<!-- reply box -->
		<div class="border-t border-border bg-card p-3 flex gap-2 flex-shrink-0">
			<Input
				type="text"
				placeholder="reply to this thread..."
				bind:value={newReply}
				onkeydown={(e) => e.key === 'Enter' && sendReply()}
				disabled={sendingReply}
				class="flex-1 bg-muted border-input"
			/>
			<Button onclick={sendReply} disabled={!newReply.trim() || sendingReply} size="sm">
				reply
			</Button>
		</div>
	{/if}
</div>

<!-- new thread dialog -->
{#if showNewThread}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
		<div class="bg-card rounded-lg p-6 w-96 space-y-4">
			<div class="flex items-center justify-between">
				<h3 class="font-semibold text-card-foreground">new post in {forumChannelName}</h3>
				<button
					class="text-muted-foreground hover:text-card-foreground"
					onclick={() => showNewThread = false}
					type="button"
					aria-label="close"
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>

			{#if createError}
				<div class="text-destructive text-sm">{createError}</div>
			{/if}

			<div class="space-y-1.5">
				<label class="text-xs font-semibold text-muted-foreground uppercase">post title</label>
				<Input type="text" placeholder="what's on your mind?" bind:value={newThreadTitle} class="bg-muted border-input" />
			</div>
			<div class="space-y-1.5">
				<label class="text-xs font-semibold text-muted-foreground uppercase">content</label>
				<textarea
					class="w-full rounded-md border border-input bg-muted p-2 text-sm text-card-foreground resize-none h-24 focus:outline-none focus:ring-1 focus:ring-ring"
					placeholder="write your post..."
					bind:value={newThreadBody}
				></textarea>
			</div>

			<div class="flex gap-2">
				<Button variant="outline" class="flex-1" onclick={() => showNewThread = false} disabled={creatingThread}>
					cancel
				</Button>
				<Button class="flex-1" onclick={createThread} disabled={!newThreadTitle.trim() || !newThreadBody.trim() || creatingThread}>
					{creatingThread ? 'posting...' : 'post'}
				</Button>
			</div>
		</div>
	</div>
{/if}
