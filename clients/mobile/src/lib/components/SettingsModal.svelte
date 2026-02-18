<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	interface Props {
		userId: string;
		accessToken: string;
		apiUrl?: string;
		onClose: () => void;
	}

	let { userId, accessToken, apiUrl = 'http://localhost:3000', onClose }: Props = $props();

	type Tab = 'account' | 'appearance' | 'notifications' | 'about' | 'advanced';
	let activeTab = $state<Tab>('account');

	// account tab state
	let displayname = $state('');
	let statusMsg = $state('');
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let success = $state('');

	// notifications tab state
	let notifGranted = $state(false);
	let notifDenied = $state(false);

	// advanced tab state
	let acknowledgeRisk = $state(false);
	let resetStatus = $state('');

	const API_URL = apiUrl;

  function shortId(uid: string) {
    return uid.replace(/^@/, "").split(":")[0];
  }

  async function loadProfile() {
    loading = true;
    try {
      const res = await fetch(
        `${API_URL}/profile/get?access_token=${encodeURIComponent(accessToken)}&user_id=${encodeURIComponent(userId)}`,
      );
      if (res.ok) {
        const data = await res.json();
        displayname = data.displayname || shortId(userId);
      }
    } catch {
      error = "failed to load";
    }
    loading = false;
  }

  async function saveProfile() {
    saving = true;
    error = "";
    success = "";
    try {
      await fetch(`${API_URL}/profile/set`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          access_token: accessToken,
          user_id: userId,
          displayname: displayname.trim() || null,
        }),
      });
      await fetch(`${API_URL}/presence/set`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          access_token: accessToken,
          user_id: userId,
          presence: "online",
          status_msg: statusMsg.trim() || null,
        }),
      });
      success = "saved!";
    } catch {
      error = "failed to save";
    }
    saving = false;
  }

  async function checkNotifPerms() {
    if (!("Notification" in window)) return;
    notifGranted = Notification.permission === "granted";
    notifDenied = Notification.permission === "denied";
  }

  async function requestNotifs() {
    if (!('Notification' in window)) return;
    const p = await Notification.requestPermission();
    checkNotifPerms();
  }

  // advanced tab functions
  function clearOnboarding() {
    localStorage.removeItem('agora_onboarding_done');
    localStorage.removeItem('agora_server_url');
    resetStatus = 'onboarding cleared — reload to see it again';
  }

  function clearAllAppData() {
    localStorage.clear();
    resetStatus = 'all local data cleared — reload the app';
  }

  function reloadApp() {
    window.location.reload();
  }

  $effect(() => {
    loadProfile();
    checkNotifPerms();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- backdrop -->
<div
  class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
  role="dialog"
  aria-modal="true"
>
  <!-- settings panel: full screen minus padding -->
  <div
    class="bg-card rounded-lg w-[800px] h-[600px] max-w-[95vw] max-h-[90vh] flex overflow-hidden shadow-2xl"
    onclick={(e) => e.stopPropagation()}
  >
    <!-- sidebar -->
    <div class="w-56 bg-sidebar border-r border-sidebar-border flex flex-col">
      <div class="p-4 border-b border-sidebar-border">
        <h2 class="font-semibold text-sidebar-foreground">settings</h2>
      </div>
      <nav class="flex-1 p-2 space-y-0.5">
        <button
          class="w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left"
          class:bg-sidebar-accent={activeTab === "account"}
          class:text-sidebar-accent-foreground={activeTab === "account"}
          class:text-sidebar-foreground={activeTab !== "account"}
          onclick={() => (activeTab = "account")}
        >
          <svg
            class="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
            /></svg
          >
          my account
        </button>
        <button
          class="w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left"
          class:bg-sidebar-accent={activeTab === "appearance"}
          class:text-sidebar-accent-foreground={activeTab === "appearance"}
          class:text-sidebar-foreground={activeTab !== "appearance"}
          onclick={() => (activeTab = "appearance")}
        >
          <svg
            class="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01"
            /></svg
          >
          appearance
        </button>
        <button
          class="w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left"
          class:bg-sidebar-accent={activeTab === "notifications"}
          class:text-sidebar-accent-foreground={activeTab === "notifications"}
          class:text-sidebar-foreground={activeTab !== "notifications"}
          onclick={() => (activeTab = "notifications")}
        >
          <svg
            class="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
            /></svg
          >
          notifications
        </button>
        <button
          class="w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left"
          class:bg-sidebar-accent={activeTab === "about"}
          class:text-sidebar-accent-foreground={activeTab === "about"}
          class:text-sidebar-foreground={activeTab !== "about"}
          onclick={() => (activeTab = "about")}
        >
          <svg
            class="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            /></svg
          >
          about
        </button>
        <button
          class="w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left"
          class:bg-sidebar-accent={activeTab === "advanced"}
          class:text-sidebar-accent-foreground={activeTab === "advanced"}
          class:text-sidebar-foreground={activeTab !== "advanced"}
          onclick={() => (activeTab = "advanced")}
        >
          <svg
            class="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            /><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            /></svg
          >
          advanced
        </button>
      </nav>

      <!-- close button at bottom of sidebar -->
      <div class="p-2 border-t border-sidebar-border">
        <button
          class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded text-sm bg-destructive/10 text-destructive hover:bg-destructive/20 transition-colors"
          onclick={onClose}
        >
          <svg
            class="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            /></svg
          >
          close
        </button>
      </div>
    </div>

    <!-- content area -->
    <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
      <!-- header -->
      <div
        class="h-14 border-b border-border flex items-center px-6 flex-shrink-0"
      >
        <h3 class="font-semibold text-card-foreground capitalize">
          {activeTab === "account" ? "my account" : activeTab}
        </h3>
      </div>

      <!-- scrollable content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if activeTab === "account"}
          <div class="space-y-6 max-w-md">
            <div>
              <h4 class="text-sm font-medium text-card-foreground mb-3">
                profile
              </h4>
              <div class="space-y-3">
                <div>
                  <label
                    class="text-xs text-muted-foreground block mb-1"
                    for="set-name">display name</label
                  >
                  <Input
                    id="set-name"
                    bind:value={displayname}
                    class="bg-muted border-input"
                    placeholder="your name"
                  />
                </div>
                <div>
                  <label
                    class="text-xs text-muted-foreground block mb-1"
                    for="set-status">status message</label
                  >
                  <Input
                    id="set-status"
                    bind:value={statusMsg}
                    class="bg-muted border-input"
                    placeholder="what are you up to?"
                  />
                </div>
                <div class="flex gap-2">
                  <Button onclick={saveProfile} disabled={saving}>
                    {saving ? "saving..." : "save changes"}
                  </Button>
                </div>
                {#if error}<p class="text-xs text-destructive">{error}</p>{/if}
                {#if success}<p class="text-xs text-primary">{success}</p>{/if}
              </div>
            </div>

            <div class="pt-4 border-t border-border">
              <h4 class="text-sm font-medium text-card-foreground mb-2">
                account details
              </h4>
              <p class="text-sm text-muted-foreground">
                user id: <span class="font-mono text-xs">{userId}</span>
              </p>
            </div>
          </div>
        {:else if activeTab === "appearance"}
          <div class="space-y-6 max-w-md">
            <div>
              <h4 class="text-sm font-medium text-card-foreground mb-3">
                theme
              </h4>
              <div class="p-4 bg-muted rounded-lg border border-border">
                <div class="flex items-center gap-3">
                  <div
                    class="w-12 h-8 rounded bg-card border border-border flex items-center justify-center"
                  >
                    <svg
                      class="w-4 h-4 text-card-foreground"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      ><path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
                      /></svg
                    >
                  </div>
                  <div>
                    <p class="text-sm font-medium text-card-foreground">dark</p>
                    <p class="text-xs text-muted-foreground">
                      agora always uses dark theme
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        {:else if activeTab === "notifications"}
          <div class="space-y-6 max-w-md">
            <div>
              <h4 class="text-sm font-medium text-card-foreground mb-3">
                desktop notifications
              </h4>
              {#if notifGranted}
                <div
                  class="p-3 bg-primary/10 border border-primary/20 rounded-lg text-sm text-primary"
                >
                  notifications enabled — you'll be notified of new messages
                  when the app is in the background
                </div>
              {:else if notifDenied}
                <div class="p-3 bg-muted border border-border rounded-lg">
                  <p class="text-sm text-muted-foreground mb-2">
                    notifications blocked
                  </p>
                  <p class="text-xs text-muted-foreground">
                    enable notifications in your browser or system settings to
                    receive desktop notifications
                  </p>
                </div>
              {:else}
                <Button onclick={requestNotifs}>enable notifications</Button>
              {/if}
            </div>
          </div>
        {:else if activeTab === "about"}
          <div class="space-y-6 max-w-md">
            <div class="text-center py-8">
              <h3 class="text-2xl font-bold text-card-foreground mb-1">
                agora
              </h3>
              <p class="text-sm text-muted-foreground mb-4">version 0.1.0</p>
              <p class="text-sm text-muted-foreground max-w-xs mx-auto">
                a free, open-source, federated communication platform built on
                Matrix.
              </p>
            </div>
            <div class="pt-4 border-t border-border">
              <h4 class="text-sm font-medium text-card-foreground mb-2">
                built with ❤ by the agora team
              </h4>
            </div>
          </div>
        {:else if activeTab === "advanced"}
          <div class="space-y-6 max-w-md">
            <!-- warning -->
            <div class="p-4 bg-destructive/10 border border-destructive/20 rounded-lg">
              <p class="text-sm text-destructive font-medium mb-2">⚠️ danger zone</p>
              <p class="text-xs text-muted-foreground">
                these options are for debugging and testing. use with caution.
              </p>
            </div>

            <!-- acknowledgement checkbox -->
            <div class="flex items-start gap-3">
              <input
                type="checkbox"
                id="acknowledge"
                bind:checked={acknowledgeRisk}
                class="mt-1 w-4 h-4 rounded border-border bg-muted"
              />
              <label for="acknowledge" class="text-sm text-muted-foreground">
                i understand these options can cause data loss or reset the app
              </label>
            </div>

            <!-- actions -->
            <div class="space-y-3">
              <Button
                variant="outline"
                class="w-full justify-start"
                disabled={!acknowledgeRisk}
                onclick={clearOnboarding}
              >
                clear onboarding data
              </Button>
              <Button
                variant="outline"
                class="w-full justify-start"
                disabled={!acknowledgeRisk}
                onclick={clearAllAppData}
              >
                reset all app data
              </Button>
              <Button
                variant="outline"
                class="w-full justify-start"
                disabled={!acknowledgeRisk}
                onclick={reloadApp}
              >
                reload app
              </Button>
            </div>

            {#if resetStatus}
              <div class="p-3 bg-primary/10 border border-primary/20 rounded text-sm text-primary">
                {resetStatus}
              </div>
            {/if}

            <!-- app info -->
            <div class="pt-4 border-t border-border">
              <h4 class="text-sm font-medium text-card-foreground mb-2">debug info</h4>
              <p class="text-xs text-muted-foreground">api url: {apiUrl}</p>
              <p class="text-xs text-muted-foreground">user id: {userId}</p>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
