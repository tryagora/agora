<script lang="ts">
	// hype train — appears when a channel is going fast (≥5 messages in 10s).
	// purely client-side, no backend. just a fun visual energy meter.

	interface Props {
		// message timestamps (ms) for the current channel — parent keeps this updated
		recentTimestamps: number[];
		// callback so parent can update a css class on the message list
		onHypeChange?: (active: boolean) => void;
	}

	let { recentTimestamps, onHypeChange }: Props = $props();

	const WINDOW_MS = 10_000; // 10 second sliding window
	const HYPE_THRESHOLD = 5;  // messages needed to trigger hype
	const COOL_DOWN_MS = 8_000; // quiet time before hype ends

	let hypeActive = $state(false);
	let hypeLevel = $state(0); // 0–100 energy meter
	let lastMessageAt = $state(0);
	let cooldownTimer: ReturnType<typeof setTimeout> | null = null;

	// recompute hype whenever recentTimestamps changes
	$effect(() => {
		const now = Date.now();
		const windowStart = now - WINDOW_MS;
		const inWindow = recentTimestamps.filter(t => t >= windowStart).length;

		// hype level is how many messages above threshold (capped at 20 → 100%)
		const raw = Math.max(0, inWindow - HYPE_THRESHOLD + 1);
		hypeLevel = Math.min(100, (raw / 15) * 100);

		if (inWindow >= HYPE_THRESHOLD) {
			if (!hypeActive) {
				hypeActive = true;
				onHypeChange?.(true);
				playHypeSound();
			}
			lastMessageAt = now;
			// reset the cooldown every time a new message arrives
			if (cooldownTimer) clearTimeout(cooldownTimer);
			cooldownTimer = setTimeout(() => {
				hypeActive = false;
				hypeLevel = 0;
				onHypeChange?.(false);
			}, COOL_DOWN_MS);
		}
	});

	// ascending chime when hype starts
	function playHypeSound() {
		try {
			const ctx = new AudioContext();
			[523, 659, 784, 1047].forEach((freq, i) => {
				const osc = ctx.createOscillator();
				const gain = ctx.createGain();
				osc.type = 'triangle';
				osc.frequency.value = freq;
				gain.gain.setValueAtTime(0.15, ctx.currentTime + i * 0.08);
				gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + i * 0.08 + 0.15);
				osc.connect(gain);
				gain.connect(ctx.destination);
				osc.start(ctx.currentTime + i * 0.08);
				osc.stop(ctx.currentTime + i * 0.08 + 0.16);
			});
			setTimeout(() => ctx.close(), 800);
		} catch { /* audio blocked */ }
	}

	// emoji burst pool for the floating particles
	const fireEmojis = ['🔥', '⚡', '💥', '🎉', '🚀'];
	let particles = $state<{ id: number; emoji: string; x: number; delay: number }[]>([]);
	let particleId = 0;

	$effect(() => {
		if (!hypeActive) { particles = []; return; }

		// add a new floating particle every 600ms
		const iv = setInterval(() => {
			const id = particleId++;
			particles = [
				...particles.slice(-8), // keep last 8 max
				{
					id,
					emoji: fireEmojis[Math.floor(Math.random() * fireEmojis.length)],
					x: Math.random() * 80 + 10, // 10%–90% horizontal
					delay: Math.random() * 0.3,
				}
			];
		}, 600);

		return () => clearInterval(iv);
	});
</script>

{#if hypeActive}
	<!-- hype train banner — sticks to the top of the message area -->
	<div class="relative overflow-hidden border-b border-orange-500/30 bg-gradient-to-r from-orange-950/80 via-red-950/80 to-orange-950/80 flex-shrink-0">
		<!-- floating emoji particles -->
		{#each particles as p (p.id)}
			<span
				class="absolute bottom-0 text-xl pointer-events-none select-none"
				style="left:{p.x}%; animation: hypeFloat 1.8s ease-out {p.delay}s forwards;"
			>
				{p.emoji}
			</span>
		{/each}

		<!-- content row -->
		<div class="relative z-10 flex items-center gap-3 px-4 py-2">
			<!-- animated fire icon -->
			<span class="text-2xl animate-bounce" style="animation-duration:0.4s">🔥</span>

			<div class="flex-1 min-w-0">
				<p class="text-xs font-black text-orange-300 uppercase tracking-widest">hype train</p>
				<p class="text-xs text-orange-400/80">chat is going crazy right now</p>
			</div>

			<!-- energy bar -->
			<div class="w-24 flex flex-col items-end gap-0.5">
				<p class="text-[10px] text-orange-400/60 font-mono">energy</p>
				<div class="w-full h-2 bg-orange-950 rounded-full overflow-hidden border border-orange-800/50">
					<div
						class="h-full rounded-full transition-all duration-500"
						style="width:{hypeLevel}%; background: linear-gradient(90deg, #f97316, #ef4444);"
					></div>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	@keyframes hypeFloat {
		0%   { transform: translateY(0)   scale(1);   opacity: 1; }
		100% { transform: translateY(-60px) scale(1.4); opacity: 0; }
	}
</style>
