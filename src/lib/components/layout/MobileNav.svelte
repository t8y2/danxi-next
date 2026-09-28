<script lang="ts">
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import BookOpenCheck from "@lucide/svelte/icons/book-open-check";
  import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
  import MessageCircleMore from "@lucide/svelte/icons/message-circle-more";
  import School from "@lucide/svelte/icons/school";
  import Settings2 from "@lucide/svelte/icons/settings-2";
  import type { NavigationSection } from "$lib/types/app";
  import { cn } from "$lib/utils";

  interface Props {
    activeSection: NavigationSection;
    onSelect: (section: NavigationSection) => void;
  }

  let { activeSection, onSelect }: Props = $props();

  const navigation = [
    { id: "overview", label: "今天", icon: LayoutDashboard },
    { id: "timetable", label: "日程", icon: CalendarDays },
    { id: "campus", label: "校园", icon: School },
    { id: "forum", label: "茶楼", icon: MessageCircleMore },
    { id: "evaluation", label: "评教", icon: BookOpenCheck },
    { id: "settings", label: "设置", icon: Settings2 },
  ] satisfies Array<{
    id: NavigationSection;
    label: string;
    icon: typeof LayoutDashboard;
  }>;
</script>

<nav class="fixed inset-x-0 bottom-0 z-20 grid h-15 grid-cols-6 border-t border-border bg-background/96 px-1 backdrop-blur lg:hidden">
  {#each navigation as item}
    {@const Icon = item.icon}
    <button
      type="button"
      class={cn(
        "flex flex-col items-center justify-center gap-0.5 text-[9px] transition",
        activeSection === item.id ? "text-primary" : "text-muted-foreground",
      )}
      aria-pressed={activeSection === item.id}
      onclick={() => onSelect(item.id)}
    >
      <Icon size={17} strokeWidth={activeSection === item.id ? 2.2 : 1.8} />
      <span>{item.label}</span>
    </button>
  {/each}
</nav>
