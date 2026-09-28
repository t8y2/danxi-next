<script lang="ts">
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import BookOpenCheck from "@lucide/svelte/icons/book-open-check";
  import CircleUserRound from "@lucide/svelte/icons/circle-user-round";
  import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
  import MessageCircleMore from "@lucide/svelte/icons/message-circle-more";
  import School from "@lucide/svelte/icons/school";
  import PanelLeftClose from "@lucide/svelte/icons/panel-left-close";
  import PanelLeftOpen from "@lucide/svelte/icons/panel-left-open";
  import Settings2 from "@lucide/svelte/icons/settings-2";
  import type { NavigationSection } from "$lib/types/app";
  import { cn } from "$lib/utils";
  import { desktopWindow, isMacOS } from "$lib/api/window";
  import { SIDEBAR_RAIL_WIDTH, sidebar } from "$lib/stores/sidebar.svelte";
  import { session } from "$lib/stores/session.svelte";

  interface Props {
    activeSection: NavigationSection;
    onSelect: (section: NavigationSection) => void;
  }

  let { activeSection, onSelect }: Props = $props();
  const compact = $derived(!sidebar.collapsed && sidebar.width < 184);
  const accountName = $derived(
    session.ready
      ? session.status.campusName?.trim() || session.status.campusId?.trim() || "user"
      : "正在恢复账户",
  );

  type NavigationItem = {
    id: NavigationSection;
    label: string;
    icon: typeof LayoutDashboard;
  };

  const primaryNavigation = [
    { id: "overview", label: "今天", icon: LayoutDashboard },
    { id: "timetable", label: "日程", icon: CalendarDays },
    { id: "campus", label: "校园", icon: School },
    { id: "forum", label: "茶楼", icon: MessageCircleMore },
    { id: "evaluation", label: "评教", icon: BookOpenCheck },
  ] satisfies NavigationItem[];

  const secondaryNavigation = [
    { id: "settings", label: "设置", icon: Settings2 },
  ] satisfies NavigationItem[];
</script>

<aside
  class={cn(
    "hidden h-full shrink-0 flex-col overflow-hidden border-r border-border bg-secondary/60 pb-4 lg:flex",
    !sidebar.resizing && "transition-[width,padding] duration-200 ease-in-out motion-reduce:transition-none",
    sidebar.collapsed || compact ? "px-2" : "px-3",
    isMacOS && desktopWindow.available ? "pt-10" : "pt-6",
  )}
  style={`width: ${sidebar.collapsed ? SIDEBAR_RAIL_WIDTH : sidebar.width}px`}
>
  <div class="mb-7 grid grid-cols-1">
    <button
      type="button"
      class={cn(
        "col-start-1 row-start-1 grid size-8 justify-self-center place-items-center rounded-lg text-muted-foreground transition-opacity duration-200 ease-in-out hover:bg-foreground/8 hover:text-foreground",
        sidebar.collapsed ? "opacity-100" : "pointer-events-none opacity-0",
      )}
      inert={!sidebar.collapsed}
      title="展开侧栏"
      aria-label="展开侧栏"
      onclick={() => sidebar.toggle()}
    >
      <PanelLeftOpen size={16} strokeWidth={1.8} />
    </button>
    <div
      class={cn(
        "col-start-1 row-start-1 flex items-center transition-[gap,padding,opacity] duration-200 ease-in-out",
        compact ? "gap-2 pl-1" : "gap-2.5 pl-2",
        sidebar.collapsed ? "pointer-events-none opacity-0" : "opacity-100",
      )}
      inert={sidebar.collapsed}
    >
      <img src="/logo.png" alt="旦夕" class="size-8 rounded-[9px] object-cover" />
      <div class="min-w-0 flex-1">
        <p class="m-0 truncate text-[17px] font-bold tracking-tight">旦夕</p>
      </div>
      <button
        type="button"
        class={cn(
          "grid size-7 shrink-0 place-items-center rounded-lg text-muted-foreground transition hover:bg-foreground/8 hover:text-foreground",
          compact ? "mr-0.5" : "mr-1.5",
        )}
        title="收起侧栏"
        aria-label="收起侧栏"
        onclick={() => sidebar.toggle()}
      >
        <PanelLeftClose size={15} strokeWidth={1.8} />
      </button>
    </div>
  </div>

  <nav class="space-y-1" aria-label="主要导航">
    {#each primaryNavigation as item}
      {@const Icon = item.icon}
      <button
        type="button"
        class={cn(
          "flex h-9 w-full items-center overflow-hidden whitespace-nowrap rounded-lg text-left text-[13px] transition-[gap,padding] duration-200 ease-in-out",
          sidebar.collapsed ? "gap-0 px-3" : compact ? "gap-2 px-2" : "gap-2.5 px-2.5",
          activeSection === item.id
            ? "bg-primary/12 font-medium text-primary"
            : "text-muted-foreground hover:bg-foreground/8 hover:text-foreground",
        )}
        aria-pressed={activeSection === item.id}
        title={item.label}
        onclick={() => onSelect(item.id)}
      >
        <Icon size={16} strokeWidth={1.8} class="shrink-0" />
        <span
          class={cn(
            "min-w-0 truncate transition-[max-width,opacity] duration-200 ease-in-out",
            sidebar.collapsed ? "max-w-0 opacity-0" : "max-w-48 opacity-100",
          )}
        >
          {item.label}
        </span>
      </button>
    {/each}
  </nav>

  <nav class="space-y-1" aria-label="次要导航">
    {#each secondaryNavigation as item}
      {@const Icon = item.icon}
      <button
        type="button"
        class={cn(
          "flex h-9 w-full items-center overflow-hidden whitespace-nowrap rounded-lg text-left text-[13px] transition-[gap,padding] duration-200 ease-in-out",
          sidebar.collapsed ? "gap-0 px-3" : compact ? "gap-2 px-2" : "gap-2.5 px-2.5",
          activeSection === item.id
            ? "bg-primary/12 font-medium text-primary"
            : "text-muted-foreground hover:bg-foreground/8 hover:text-foreground",
        )}
        aria-pressed={activeSection === item.id}
        title={item.label}
        onclick={() => onSelect(item.id)}
      >
        <Icon size={16} strokeWidth={1.8} class="shrink-0" />
        <span
          class={cn(
            "min-w-0 truncate transition-[max-width,opacity] duration-200 ease-in-out",
            sidebar.collapsed ? "max-w-0 opacity-0" : "max-w-48 opacity-100",
          )}
        >
          {item.label}
        </span>
      </button>
    {/each}
  </nav>

  <div
    class={cn(
      "mt-auto overflow-hidden transition-[max-height,opacity,padding,border-color] duration-200 ease-in-out",
      sidebar.collapsed
        ? "max-h-0 border-t border-transparent pt-0 opacity-0"
        : "max-h-24 border-t border-border pt-3 opacity-100",
    )}
    inert={sidebar.collapsed}
  >
    <button
      type="button"
      class={cn(
        "flex w-full items-center rounded-lg py-2 text-left hover:bg-foreground/8",
        compact ? "gap-2 px-1.5" : "gap-2.5 px-2",
      )}
    >
      <span class="grid size-8 shrink-0 place-items-center rounded-full bg-muted text-muted-foreground">
        <CircleUserRound size={17} />
      </span>
      <span class="min-w-0">
        <strong class="block truncate text-xs font-medium">
          {accountName}
        </strong>
        <small class="block truncate text-[10px] text-muted-foreground">
          {!session.ready
            ? "正在读取本地登录状态"
            : session.status.communityLoggedIn
            ? session.status.campusLoggedIn
              ? "旦挞与复旦 UIS 已登录"
              : "旦挞账号已登录"
            : session.status.campusLoggedIn
              ? "复旦统一认证已登录"
              : "在设置中管理账户"}
        </small>
      </span>
    </button>
  </div>
</aside>
