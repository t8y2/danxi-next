<script lang="ts">
  import { onMount } from "svelte";
  import EvaluationPanel from "$lib/components/dashboard/EvaluationPanel.svelte";
  import CampusPanel from "$lib/components/dashboard/CampusPanel.svelte";
  import ForumPanel from "$lib/components/dashboard/ForumPanel.svelte";
  import HeroPanel from "$lib/components/dashboard/HeroPanel.svelte";
  import SchedulePanel from "$lib/components/dashboard/SchedulePanel.svelte";
  import SettingsPanel from "$lib/components/dashboard/SettingsPanel.svelte";
  import LoginDialog from "$lib/components/settings/LoginDialog.svelte";
  import AppShell from "$lib/components/layout/AppShell.svelte";
  import { forum, session } from "$lib/stores/session.svelte";
  import type { NavigationSection } from "$lib/types/app";

  let activeSection = $state<NavigationSection>("overview");
  let loginOpen = $state(false);
  let loginMethod = $state<"community" | "campus">("community");

  function openLogin(method: "community" | "campus") {
    loginMethod = method;
    loginOpen = true;
  }

  function openForum(holeId?: number) {
    activeSection = "forum";
    if (holeId != null) void forum.open(holeId);
  }

  onMount(() => {
    void session.restore();
  });
</script>

<AppShell {activeSection} onSelect={(section) => (activeSection = section)}>
  {#if activeSection === "overview"}
    <div class="grid gap-3 py-3 lg:grid-cols-2 lg:gap-4 lg:py-4">
      <div>
        <HeroPanel onLogin={() => openLogin("community")} />
      </div>
      <div>
        <SchedulePanel onLogin={() => openLogin("campus")} />
      </div>
      <div class="lg:col-span-2">
        <ForumPanel onLogin={() => openLogin("community")} onOpenForum={openForum} />
      </div>
    </div>
  {:else if activeSection === "timetable"}
    <SchedulePanel expanded onLogin={() => openLogin("campus")} />
  {:else if activeSection === "campus"}
    <CampusPanel onLogin={() => openLogin("campus")} />
  {:else if activeSection === "forum"}
    <ForumPanel expanded onLogin={() => openLogin("community")} />
  {:else if activeSection === "evaluation"}
    <EvaluationPanel onLogin={() => openLogin("community")} />
  {:else}
    <SettingsPanel onLogin={openLogin} />
  {/if}
</AppShell>

<LoginDialog open={loginOpen} onClose={() => (loginOpen = false)} initialMethod={loginMethod} />
