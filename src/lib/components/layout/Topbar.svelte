<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import Minus from "@lucide/svelte/icons/minus";
  import Moon from "@lucide/svelte/icons/moon";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Square from "@lucide/svelte/icons/square";
  import Sun from "@lucide/svelte/icons/sun";
  import X from "@lucide/svelte/icons/x";
  import { desktopWindow, isMacOS } from "$lib/api/window";
  import type { CampusLocation, NavigationSection } from "$lib/types/app";
  import { Button } from "$lib/components/ui/button";
  import { Popover } from "$lib/components/ui/popover";
  import { CAMPUS_OPTIONS, campusServices } from "$lib/stores/campus-services.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { theme } from "$lib/theme/theme.svelte";
  import { cn } from "$lib/utils";

  interface Props {
    activeSection: NavigationSection;
  }

  let { activeSection }: Props = $props();
  let campusMenuOpen = $state(false);

  const campusLabel = $derived(
    CAMPUS_OPTIONS.find((option) => option.value === campusServices.campus)?.label ?? "邯郸",
  );

  const titles: Record<NavigationSection, string> = {
    overview: "今天",
    timetable: "日程",
    campus: "校园",
    forum: "茶楼",
    evaluation: "评教",
    settings: "设置",
  };

  function selectCampus(campus: CampusLocation) {
    campusServices.setCampus(campus);
    campusMenuOpen = false;
  }

  function refreshCampus() {
    if (session.status.campusLoggedIn) {
      void campusServices.loadAll(true);
    } else {
      void campusServices.loadLibrary(true);
      campusServices.requireLogin();
    }
  }
</script>

<header
  class="flex h-11 shrink-0 select-none items-center justify-between border-b border-border bg-background/94 pl-4 lg:pl-7"
>
  <div data-tauri-drag-region class="flex h-full min-w-0 flex-1 items-center gap-2">
    <img src="/logo.png" alt="旦夕" class="size-7 rounded-lg object-cover lg:hidden" />
    <span class="text-[16px] font-[650] tracking-[-0.018em] text-foreground">
      {titles[activeSection]}
    </span>
  </div>
  {#if activeSection === "campus"}
    <div class="hidden shrink-0 items-center gap-1 sm:flex">
      <div class="flex items-center rounded-lg bg-muted/65 p-0.5" aria-label="选择校区">
        {#each CAMPUS_OPTIONS as option}
          <button
            type="button"
            class={cn(
              "focus-ring h-7 rounded-md px-3 text-xs font-medium transition",
              campusServices.campus === option.value
                ? "bg-card text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
            aria-pressed={campusServices.campus === option.value}
            onclick={() => selectCampus(option.value)}
          >
            {option.label}
          </button>
        {/each}
      </div>
      <Button
        variant="ghost"
        size="icon"
        class="size-8"
        aria-label="刷新校园数据"
        title="刷新校园数据"
        onclick={refreshCampus}
      >
        <RefreshCw size={14} />
      </Button>
    </div>
    <div class="flex shrink-0 items-center gap-0.5 sm:hidden">
      <Popover
        bind:open={campusMenuOpen}
        align="end"
        triggerClass="focus-ring flex h-8 items-center gap-1 rounded-lg px-2 text-xs font-medium text-muted-foreground transition hover:bg-muted hover:text-foreground"
      >
        {#snippet trigger()}{campusLabel}<ChevronDown size={13} />{/snippet}
        {#snippet content()}
          <div class="grid min-w-28 gap-0.5" aria-label="选择校区">
            {#each CAMPUS_OPTIONS as option}
              <button
                type="button"
                class={cn(
                  "focus-ring rounded-md px-3 py-2 text-left text-xs transition",
                  campusServices.campus === option.value
                    ? "bg-secondary font-medium text-foreground"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
                aria-pressed={campusServices.campus === option.value}
                onclick={() => selectCampus(option.value)}
              >
                {option.label}
              </button>
            {/each}
          </div>
        {/snippet}
      </Popover>
      <Button
        variant="ghost"
        size="icon"
        class="size-8"
        aria-label="刷新校园数据"
        title="刷新校园数据"
        onclick={refreshCampus}
      >
        <RefreshCw size={14} />
      </Button>
    </div>
  {/if}
  <div class="flex h-full items-center gap-1 pl-3">
    <Button
      variant="ghost"
      size="icon"
      class="size-8"
      aria-label={theme.resolved === "dark" ? "切换到浅色主题" : "切换到深色主题"}
      title={theme.resolved === "dark" ? "切换到浅色主题" : "切换到深色主题"}
      onclick={() => theme.toggle()}
    >
      {#if theme.resolved === "dark"}
        <Sun size={16} />
      {:else}
        <Moon size={16} />
      {/if}
    </Button>
    {#if desktopWindow.available && !isMacOS}
      <div class="ml-2 flex h-full items-stretch border-l border-border">
        <button
          type="button"
          class="grid w-11 place-items-center text-muted-foreground transition hover:bg-muted hover:text-foreground"
          aria-label="最小化窗口"
          title="最小化"
          onclick={() => desktopWindow.minimize()}
        >
          <Minus size={14} />
        </button>
        <button
          type="button"
          class="grid w-11 place-items-center text-muted-foreground transition hover:bg-muted hover:text-foreground"
          aria-label="最大化或还原窗口"
          title="最大化或还原"
          onclick={() => desktopWindow.toggleMaximize()}
        >
          <Square size={12} />
        </button>
        <button
          type="button"
          class="grid w-11 place-items-center text-muted-foreground transition hover:bg-red-500 hover:text-white"
          aria-label="关闭窗口"
          title="关闭"
          onclick={() => desktopWindow.close()}
        >
          <X size={15} />
        </button>
      </div>
    {:else}
      <span class="w-4"></span>
    {/if}
  </div>
</header>
