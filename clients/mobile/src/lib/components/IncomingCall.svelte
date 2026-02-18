<script lang="ts">
	interface Props {
		callerName: string;
		callId: string;
		roomId: string;
		onAccept: () => void;
		onDecline: () => void;
	}

	let { callerName, callId, roomId, onAccept, onDecline }: Props = $props();

	// auto-decline after 30 seconds if not answered
	$effect(() => {
		const timer = setTimeout(() => {
			onDecline();
		}, 30_000);

		// play a simple beep pattern using the Web Audio API for the ringtone
		// falls back silently if audio isn't available
		let audioCtx: AudioContext | null = null;
		let ringInterval: ReturnType<typeof setInterval> | null = null;

		try {
			audioCtx = new AudioContext();

			function playBeep() {
				if (!audioCtx) return;
				// two-tone ring like a phone
				[880, 1100].forEach((freq, i) => {
					const osc = audioCtx!.createOscillator();
					const gain = audioCtx!.createGain();
					osc.connect(gain);
					gain.connect(audioCtx!.destination);
					osc.frequency.value = freq;
					osc.type = 'sine';
					gain.gain.setValueAtTime(0.15, audioCtx!.currentTime + i * 0.15);
					gain.gain.exponentialRampToValueAtTime(0.001, audioCtx!.currentTime + i * 0.15 + 0.12);
					osc.start(audioCtx!.currentTime + i * 0.15);
					osc.stop(audioCtx!.currentTime + i * 0.15 + 0.13);
				});
			}

			playBeep();
			ringInterval = setInterval(playBeep, 1800);
		} catch {
			// audio not available — silent ring
		}

		return () => {
			clearTimeout(timer);
			if (ringInterval) clearInterval(ringInterval);
			audioCtx?.close();
		};
	});
</script>

<!-- full-screen overlay with blur backdrop -->
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
	<div class="bg-card rounded-2xl shadow-2xl w-72 overflow-hidden">
		<!-- avatar + name header -->
		<div class="bg-primary/10 px-6 pt-8 pb-6 flex flex-col items-center gap-3">
			<!-- animated ringing avatar -->
			<div class="relative">
				<div class="w-20 h-20 rounded-full bg-primary flex items-center justify-center text-primary-foreground text-3xl font-bold">
					{(callerName[0] || '?').toUpperCase()}
				</div>
				<!-- pulsing ring animation -->
				<div class="absolute inset-0 rounded-full border-2 border-primary animate-ping opacity-40"></div>
			</div>
			<div class="text-center">
				<p class="font-semibold text-card-foreground text-lg">{callerName}</p>
				<p class="text-sm text-muted-foreground">incoming voice call</p>
			</div>
		</div>

		<!-- accept / decline buttons -->
		<div class="flex border-t border-border">
			<!-- decline -->
			<button
				class="flex-1 flex flex-col items-center gap-1.5 py-4 hover:bg-destructive/10 transition-colors group"
				onclick={onDecline}
				aria-label="decline call"
			>
				<div class="w-10 h-10 rounded-full bg-destructive flex items-center justify-center">
					<!-- phone-off icon -->
					<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.517l2.257-1.128a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
					</svg>
				</div>
				<span class="text-xs text-muted-foreground">decline</span>
			</button>

			<div class="w-px bg-border"></div>

			<!-- accept -->
			<button
				class="flex-1 flex flex-col items-center gap-1.5 py-4 hover:bg-green-500/10 transition-colors group"
				onclick={onAccept}
				aria-label="accept call"
			>
				<div class="w-10 h-10 rounded-full bg-green-500 flex items-center justify-center">
					<!-- phone icon -->
					<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z" />
					</svg>
				</div>
				<span class="text-xs text-muted-foreground">accept</span>
			</button>
		</div>
	</div>
</div>
