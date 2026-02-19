<script lang="ts">
	// raid alert — shown to every member of a server when someone triggers a raid.
	// full-screen animated overlay with countdown bar and shake animation.
	// dismissed automatically when countdown hits 0, or manually.

	interface Props {
		raiderName: string;
		message: string;
		countdown: number; // seconds
		onDismiss: () => void;
	}

	let { raiderName, message, countdown, onDismiss }: Props = $props();

	// use a local copy so changes to the prop don't reset the timer mid-countdown
	const totalSeconds = countdown;
	let remaining = $state(totalSeconds);
	let progress = $state(100); // percentage for the countdown bar

	// web audio api — raid klaxon: three short descending blares
	function playRaidSound() {
		try {
			const ctx = new AudioContext();
			const blares = [880, 660, 440];
			blares.forEach((freq, i) => {
				const osc = ctx.createOscillator();
				const gain = ctx.createGain();
				osc.type = 'sawtooth';
				osc.frequency.value = freq;
				gain.gain.setValueAtTime(0.3, ctx.currentTime + i * 0.22);
				gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + i * 0.22 + 0.2);
				osc.connect(gain);
				gain.connect(ctx.destination);
				osc.start(ctx.currentTime + i * 0.22);
				osc.stop(ctx.currentTime + i * 0.22 + 0.21);
			});
			setTimeout(() => ctx.close(), 1500);
		} catch { /* audio blocked */ }
	}

	$effect(() => {
		playRaidSound();

		const tick = setInterval(() => {
			remaining -= 1;
			progress = (remaining / totalSeconds) * 100;
			if (remaining <= 0) {
				clearInterval(tick);
				onDismiss();
			}
		}, 1000);

		return () => clearInterval(tick);
	});
</script>

<!-- full-screen raid overlay -->
<div class="fixed inset-0 z-[60] flex items-center justify-center overflow-hidden">
	<!-- animated red pulsing background -->
	<div class="absolute inset-0 bg-red-950/90 backdrop-blur-sm animate-pulse" style="animation-duration:0.5s"></div>

	<!-- border sweep animation via CSS gradient -->
	<div class="absolute inset-0 border-4 border-red-500/80 pointer-events-none" style="box-shadow: inset 0 0 80px rgba(239,68,68,0.4), 0 0 80px rgba(239,68,68,0.3);"></div>

	<!-- content card -->
	<div class="relative z-10 flex flex-col items-center gap-6 px-8 py-10 max-w-md w-full text-center"
		style="animation: raidShake 0.15s ease-in-out infinite alternate;"
	>
		<!-- RAID siren icon — two red triangles -->
		<div class="flex gap-3">
			<div class="w-12 h-12 text-red-400 animate-bounce" style="animation-delay:0ms">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
					<path d="M12 2L2 19.5h20L12 2zm0 3.5l7.5 13H4.5L12 5.5z"/>
					<path d="M11 10h2v5h-2zm0 6h2v2h-2z" fill="currentColor"/>
				</svg>
			</div>
			<div class="w-12 h-12 text-red-400 animate-bounce" style="animation-delay:150ms">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
					<path d="M12 2L2 19.5h20L12 2zm0 3.5l7.5 13H4.5L12 5.5z"/>
					<path d="M11 10h2v5h-2zm0 6h2v2h-2z" fill="currentColor"/>
				</svg>
			</div>
			<div class="w-12 h-12 text-red-400 animate-bounce" style="animation-delay:300ms">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
					<path d="M12 2L2 19.5h20L12 2zm0 3.5l7.5 13H4.5L12 5.5z"/>
					<path d="M11 10h2v5h-2zm0 6h2v2h-2z" fill="currentColor"/>
				</svg>
			</div>
		</div>

		<!-- raid text -->
		<div>
			<p class="text-6xl font-black text-red-400 tracking-widest uppercase"
				style="text-shadow: 0 0 30px rgba(239,68,68,0.8); animation: raidShake 0.1s ease-in-out infinite alternate;">
				RAID
			</p>
			<p class="text-2xl font-bold text-white mt-2">{raiderName}</p>
			{#if message && message !== 'RAID!'}
				<p class="text-lg text-red-300 mt-1 italic">"{message}"</p>
			{/if}
		</div>

		<!-- countdown -->
		<div class="w-full">
			<p class="text-4xl font-mono font-black text-white tabular-nums">{remaining}</p>
			<div class="mt-3 w-full h-3 bg-red-950 rounded-full overflow-hidden border border-red-800">
				<div
					class="h-full bg-red-500 rounded-full transition-all duration-1000 ease-linear"
					style="width: {progress}%"
				></div>
			</div>
			<p class="text-xs text-red-400 mt-1">seconds until raid</p>
		</div>

		<!-- dismiss button -->
		<button
			class="text-xs text-red-400/60 hover:text-red-400 transition-colors underline underline-offset-2"
			onclick={onDismiss}
		>
			dismiss
		</button>
	</div>
</div>

<style>
	@keyframes raidShake {
		from { transform: translateX(-2px) rotate(-0.5deg); }
		to   { transform: translateX(2px)  rotate(0.5deg); }
	}
</style>
