<script lang="ts">
  import type { Snippet } from "svelte";
  import MobileNav from "./MobileNav.svelte";
  import Sidebar from "./Sidebar.svelte";
  import Topbar from "./Topbar.svelte";
  import type { NavigationSection } from "$lib/types/app";
  import { sidebar } from "$lib/stores/sidebar.svelte";

  interface Props {
    activeSection: NavigationSection;
    onSelect: (section: NavigationSection) => void;
    children: Snippet;
  }

  let { activeSection, onSelect, children }: Props = $props();

  function startResize(event: PointerEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = sidebar.width;

    const onMove = (move: PointerEvent) => {
      sidebar.setWidth(startWidth + (move.clientX - startX));
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      document.body.style.cursor = "";
      sidebar.resizing = false;
      sidebar.commitWidth();
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
    document.body.style.cursor = "col-resize";
    sidebar.resizing = true;
  }
</script>

<div class="flex h-screen min-h-0 w-full overflow-hidden bg-background text-foreground">
  <Sidebar {activeSection} {onSelect} />
  {#if !sidebar.collapsed}
    <div
      class="group relative z-10 hidden w-0 shrink-0 lg:block"
      role="separator"
      aria-orientation="vertical"
      aria-label="拖拽调整侧栏宽度，双击恢复默认"
    >
      <div
        class="absolute inset-y-0 -left-2 w-4 cursor-col-resize"
        role="separator"
        aria-orientation="vertical"
        tabindex="-1"
        onpointerdown={startResize}
        ondblclick={() => sidebar.resetWidth()}
      ></div>
      <div
        class="pointer-events-none absolute inset-y-0 left-0 w-px bg-primary/0 transition group-hover:bg-primary/35"
      ></div>
    </div>
  {/if}
  <section class="flex min-w-0 flex-1 flex-col">
    <Topbar {activeSection} />
    <main
      class={`min-h-0 flex-1 ${
        activeSection === "forum"
          ? "overflow-hidden pb-15 lg:pb-0"
          : activeSection === "evaluation"
          ? "overflow-y-auto px-4 pb-20 lg:overflow-hidden lg:px-6 lg:pb-6"
          : `overflow-y-auto ${activeSection === "timetable" ? "" : "px-4 pb-20 lg:px-6 lg:pb-8"}`
      }`}
    >
      {#if activeSection === "timetable"}
        {@render children()}
      {:else}
        <div
          class={
            activeSection === "forum"
              ? "h-full min-h-0 w-full"
              : `mx-auto w-full max-w-[1180px] ${
                  activeSection === "evaluation" ? "lg:h-full lg:min-h-0" : ""
                }`
          }
        >
          {@render children()}
        </div>
      {/if}
    </main>
  </section>
  <MobileNav {activeSection} {onSelect} />
</div>
