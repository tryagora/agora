<script lang="ts">
	// vibe rooms — shared ambient sound that all voice channel participants hear together.
	// one person changes it, everyone in the channel sees the change on their next poll.
	// all audio is synthesized via web audio api — no asset files, no cdn.

	interface Props {
		roomId: string;
		accessToken: string;
		userId: string;
		apiUrl?: string;
		// current vibe set by whoever owns the room state
		currentVibe?: string;
		setBy?: string;
		onVibeChange?: (vibe: string) => void;
	}

	let {
		roomId,
		accessToken,
		userId,
		apiUrl = 'http://localhost:3000',
		currentVibe = 'none',
		setBy = undefined,
		onVibeChange,
	}: Props = $props();

	type VibeId = 'none' | 'rain' | 'lofi' | 'campfire' | 'space';

	const vibes: { id: VibeId; label: string; emoji: string; desc: string }[] = [
		{ id: 'none',     label: 'off',      emoji: '🔇', desc: 'no ambient sound' },
		{ id: 'rain',     label: 'rain',     emoji: '🌧️', desc: 'heavy rain on glass' },
		{ id: 'lofi',     label: 'lo-fi',    emoji: '🎵', desc: 'chill chord drone' },
		{ id: 'campfire', label: 'campfire', emoji: '🔥', desc: 'crackling fire' },
		{ id: 'space',    label: 'space',    emoji: '🌌', desc: 'deep space ambience' },
	];

	// audio engine — one AudioContext shared for the whole session
	let audioCtx: AudioContext | null = null;
	// currently playing nodes — stop these when switching vibes
	let activeNodes: AudioNode[] = [];
	// master gain so we can fade in/out
	let masterGain: GainNode | null = null;

	function getAudioCtx(): AudioContext {
		if (!audioCtx) {
			audioCtx = new AudioContext();
			masterGain = audioCtx.createGain();
			masterGain.gain.setValueAtTime(0, audioCtx.currentTime);
			masterGain.connect(audioCtx.destination);
		}
		return audioCtx;
	}

	function stopAll() {
		for (const node of activeNodes) {
			try {
				(node as OscillatorNode | AudioBufferSourceNode).stop?.();
				node.disconnect();
			} catch { /* already stopped */ }
		}
		activeNodes = [];
	}

	// fade master gain to target over 2 seconds
	function fadeTo(target: number) {
		if (!masterGain || !audioCtx) return;
		masterGain.gain.linearRampToValueAtTime(target, audioCtx.currentTime + 2);
	}

	// ── ambient generators ────────────────────────────────────────────────────

	function playRain(ctx: AudioContext, dest: AudioNode) {
		// heavy rain = white noise filtered through a bandpass + a low rumble
		const bufSize = ctx.sampleRate * 4; // 4-second looping buffer
		const buf = ctx.createBuffer(1, bufSize, ctx.sampleRate);
		const data = buf.getChannelData(0);
		for (let i = 0; i < bufSize; i++) data[i] = (Math.random() * 2 - 1);

		const src = ctx.createBufferSource();
		src.buffer = buf;
		src.loop = true;

		// bandpass gives the hissy "rain on glass" character
		const bp = ctx.createBiquadFilter();
		bp.type = 'bandpass';
		bp.frequency.value = 3000;
		bp.Q.value = 0.5;

		// low shelf adds the rumble of heavy downpour
		const low = ctx.createBiquadFilter();
		low.type = 'lowshelf';
		low.frequency.value = 200;
		low.gain.value = 8;

		src.connect(bp);
		bp.connect(low);
		low.connect(dest);
		src.start();
		activeNodes.push(src, bp, low);

		// second layer: slightly different noise for the "patter" texture
		const buf2 = ctx.createBuffer(1, bufSize, ctx.sampleRate);
		const d2 = buf2.getChannelData(0);
		for (let i = 0; i < bufSize; i++) d2[i] = (Math.random() * 2 - 1);
		const src2 = ctx.createBufferSource();
		src2.buffer = buf2;
		src2.loop = true;
		const hp = ctx.createBiquadFilter();
		hp.type = 'highpass';
		hp.frequency.value = 6000;
		hp.Q.value = 0.3;
		const g2 = ctx.createGain();
		g2.gain.value = 0.4;
		src2.connect(hp); hp.connect(g2); g2.connect(dest);
		src2.start();
		activeNodes.push(src2, hp, g2);
	}

	function playLofi(ctx: AudioContext, dest: AudioNode) {
		// lo-fi = three-note chord drone (Am7 = A, C, E, G) with slight wobble
		// plus a filtered vinyl crackle layer
		const chord = [220, 261.63, 329.63, 392]; // Am7

		for (const freq of chord) {
			const osc = ctx.createOscillator();
			osc.type = 'sawtooth';
			osc.frequency.value = freq;

			// slow vibrato via lfo
			const lfo = ctx.createOscillator();
			lfo.type = 'sine';
			lfo.frequency.value = 0.3 + Math.random() * 0.2; // slightly detuned per voice
			const lfoGain = ctx.createGain();
			lfoGain.gain.value = 2;
			lfo.connect(lfoGain);
			lfoGain.connect(osc.frequency);

			// lowpass filter to make it muffled like a cassette tape
			const lp = ctx.createBiquadFilter();
			lp.type = 'lowpass';
			lp.frequency.value = 800;
			lp.Q.value = 0.8;

			const g = ctx.createGain();
			g.gain.value = 0.08; // keep individual voices quiet

			osc.connect(lp); lp.connect(g); g.connect(dest);
			lfo.start(); osc.start();
			activeNodes.push(osc, lfo, lfoGain, lp, g);
		}

		// vinyl crackle = very sparse noise bursts
		const crackleBuf = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
		const cd = crackleBuf.getChannelData(0);
		for (let i = 0; i < cd.length; i++) {
			// most samples silent, occasional loud click
			cd[i] = Math.random() < 0.001 ? (Math.random() * 2 - 1) : 0;
		}
		const crackle = ctx.createBufferSource();
		crackle.buffer = crackleBuf;
		crackle.loop = true;
		const cg = ctx.createGain();
		cg.gain.value = 0.15;
		crackle.connect(cg); cg.connect(dest);
		crackle.start();
		activeNodes.push(crackle, cg);
	}

	function playCampfire(ctx: AudioContext, dest: AudioNode) {
		// campfire = low-frequency noise shaped to crackle with random pops
		const bufSize = ctx.sampleRate * 3;
		const buf = ctx.createBuffer(1, bufSize, ctx.sampleRate);
		const data = buf.getChannelData(0);

		// brownian-ish noise (integrate white noise) for the base rumble
		let last = 0;
		for (let i = 0; i < bufSize; i++) {
			last = (last + (Math.random() * 2 - 1) * 0.15) * 0.98;
			data[i] = last;
		}

		const base = ctx.createBufferSource();
		base.buffer = buf;
		base.loop = true;

		const lp = ctx.createBiquadFilter();
		lp.type = 'lowpass';
		lp.frequency.value = 500;

		const g = ctx.createGain();
		g.gain.value = 1.5;

		base.connect(lp); lp.connect(g); g.connect(dest);
		base.start();
		activeNodes.push(base, lp, g);

		// random crackle pops — scheduled as individual impulses
		function schedulePops() {
			if (!audioCtx) return;
			const now = audioCtx.currentTime;
			// schedule the next 2 seconds of pops
			for (let t = 0; t < 2; t += Math.random() * 0.4 + 0.05) {
				const popBuf = ctx.createBuffer(1, 512, ctx.sampleRate);
				const pd = popBuf.getChannelData(0);
				for (let i = 0; i < 512; i++) {
					// exponential decay — sounds like a wood pop
					pd[i] = (Math.random() * 2 - 1) * Math.exp(-i / 40);
				}
				const pop = ctx.createBufferSource();
				pop.buffer = popBuf;
				const pg = ctx.createGain();
				pg.gain.value = Math.random() * 0.6 + 0.1;
				pop.connect(pg); pg.connect(dest);
				pop.start(now + t);
				activeNodes.push(pop, pg);
			}
		}

		schedulePops();
		// re-schedule every 1.8s so pops continue
		const popTimer = setInterval(schedulePops, 1800);
		// store timer so we can cancel it on vibe change
		activePopTimer = popTimer;
	}

	function playSpace(ctx: AudioContext, dest: AudioNode) {
		// space = very slow, deep pad built from detuned sine waves with long attack
		const freqs = [55, 82.4, 110, 138.6]; // low A, E, A, C# — a major chord very low

		for (const freq of freqs) {
			const osc = ctx.createOscillator();
			osc.type = 'sine';
			osc.frequency.value = freq + (Math.random() - 0.5) * 1.5; // slight detune

			// ultra-slow lfo for the "breathing" feel
			const lfo = ctx.createOscillator();
			lfo.type = 'sine';
			lfo.frequency.value = 0.05 + Math.random() * 0.03;
			const lfoGain = ctx.createGain();
			lfoGain.gain.value = freq * 0.02;
			lfo.connect(lfoGain);
			lfoGain.connect(osc.frequency);

			const g = ctx.createGain();
			// slow attack so it swells in
			g.gain.setValueAtTime(0, ctx.currentTime);
			g.gain.linearRampToValueAtTime(0.12, ctx.currentTime + 4);

			osc.connect(g); g.connect(dest);
			lfo.start(); osc.start();
			activeNodes.push(osc, lfo, lfoGain, g);
		}

		// a very subtle high shimmer from high overtones
		const shimmer = ctx.createOscillator();
		shimmer.type = 'sine';
		shimmer.frequency.value = 4400;
		const sg = ctx.createGain();
		sg.gain.setValueAtTime(0, ctx.currentTime);
		sg.gain.linearRampToValueAtTime(0.008, ctx.currentTime + 6);
		shimmer.connect(sg); sg.connect(dest);
		shimmer.start();
		activeNodes.push(shimmer, sg);
	}

	let activePopTimer: ReturnType<typeof setInterval> | null = null;

	function applyVibe(vibe: string) {
		// always fade out first, then switch
		if (activePopTimer) { clearInterval(activePopTimer); activePopTimer = null; }

		if (vibe === 'none') {
			fadeTo(0);
			setTimeout(stopAll, 2100);
			return;
		}

		const ctx = getAudioCtx();
		// resume if browser suspended it (autoplay policy)
		if (ctx.state === 'suspended') ctx.resume();

		fadeTo(0);
		setTimeout(() => {
			stopAll();
			if (!masterGain) return;
			switch (vibe as VibeId) {
				case 'rain':     playRain(ctx, masterGain);     break;
				case 'lofi':     playLofi(ctx, masterGain);     break;
				case 'campfire': playCampfire(ctx, masterGain); break;
				case 'space':    playSpace(ctx, masterGain);    break;
			}
			fadeTo(1);
		}, 800); // brief silence between vibes
	}

	// react to prop changes from the parent (sync loop updated currentVibe)
	$effect(() => {
		applyVibe(currentVibe);
		return () => {
			// cleanup when component unmounts (voice channel disconnected)
			if (activePopTimer) clearInterval(activePopTimer);
			fadeTo(0);
			setTimeout(() => { stopAll(); audioCtx?.close(); audioCtx = null; }, 800);
		};
	});

	let setting = $state(false);
	let settingError = $state('');

	async function setVibe(vibeId: VibeId) {
		setting = true;
		settingError = '';
		try {
			const res = await fetch(`${apiUrl}/voice/vibe`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					access_token: accessToken,
					room_id: roomId,
					vibe: vibeId,
					user_id: userId,
				}),
			});
			if (res.ok) {
				onVibeChange?.(vibeId);
			} else {
				settingError = 'failed to set vibe';
			}
		} catch {
			settingError = 'network error';
		} finally {
			setting = false;
		}
	}

	const setByShort = $derived(setBy ? setBy.split(':')[0].replace('@', '') : null);
</script>

<!-- vibe picker — compact row that lives inside the voice channel panel -->
<div class="border border-border rounded-lg overflow-hidden bg-card/60">
	<!-- header -->
	<div class="flex items-center justify-between px-3 py-2 border-b border-border">
		<div class="flex items-center gap-1.5">
			<span class="text-sm font-semibold text-card-foreground">vibe</span>
			{#if currentVibe !== 'none' && setByShort}
				<span class="text-xs text-muted-foreground">— set by {setByShort}</span>
			{/if}
		</div>
		{#if settingError}
			<span class="text-xs text-destructive">{settingError}</span>
		{/if}
	</div>

	<!-- vibe grid -->
	<div class="grid grid-cols-5 divide-x divide-border">
		{#each vibes as v (v.id)}
			{@const active = currentVibe === v.id}
			<button
				class={`flex flex-col items-center gap-1 py-3 px-1 transition-colors relative
					${active
						? 'bg-primary/15 text-card-foreground'
						: 'text-muted-foreground hover:bg-muted hover:text-card-foreground'}`}
				onclick={() => setVibe(v.id)}
				disabled={setting}
				title={v.desc}
				aria-label={`set vibe: ${v.label}`}
				aria-pressed={active}
			>
				<!-- animated pulse ring when this vibe is active -->
				{#if active && v.id !== 'none'}
					<span class="absolute inset-0 rounded pointer-events-none">
						<span class="absolute inset-0 bg-primary/10 animate-pulse rounded"></span>
					</span>
				{/if}
				<span class="text-lg leading-none relative">{v.emoji}</span>
				<span class="text-[10px] leading-none font-medium relative">{v.label}</span>
			</button>
		{/each}
	</div>

	<!-- active vibe status bar — only shown when a vibe is playing -->
	{#if currentVibe !== 'none'}
		{@const v = vibes.find(x => x.id === currentVibe)}
		{#if v}
			<div class="flex items-center gap-2 px-3 py-1.5 border-t border-border bg-primary/5">
				<div class="flex gap-0.5 items-center">
					<!-- animated sound wave bars -->
					<span class="w-0.5 h-2 bg-primary rounded-full animate-bounce" style="animation-delay:0ms"></span>
					<span class="w-0.5 h-3 bg-primary rounded-full animate-bounce" style="animation-delay:150ms"></span>
					<span class="w-0.5 h-2 bg-primary rounded-full animate-bounce" style="animation-delay:300ms"></span>
					<span class="w-0.5 h-3 bg-primary rounded-full animate-bounce" style="animation-delay:150ms"></span>
					<span class="w-0.5 h-1.5 bg-primary rounded-full animate-bounce" style="animation-delay:0ms"></span>
				</div>
				<span class="text-xs text-primary font-medium">{v.emoji} {v.label} vibes</span>
				<span class="ml-auto text-xs text-muted-foreground">{v.desc}</span>
			</div>
		{/if}
	{/if}
</div>
