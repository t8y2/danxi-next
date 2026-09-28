<script lang="ts">
  import Check from "@lucide/svelte/icons/check";
  import Laptop from "@lucide/svelte/icons/laptop";
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import { theme, type ThemePreference } from "$lib/theme/theme.svelte";
  import { cn } from "$lib/utils";

  const options = [
    { value: "light", label: "浅色", icon: Sun },
    { value: "dark", label: "深色", icon: Moon },
    { value: "system", label: "跟随系统", icon: Laptop },
  ] satisfies Array<{
    value: ThemePreference;
    label: string;
    icon: typeof Sun;
  }>;
</script>

<div class="overflow-hidden rounded-xl border border-border bg-card">
  {#each options as option}
    {@const Icon = option.icon}
    <button
      type="button"
      class="flex w-full items-center gap-3 border-b border-border px-4 py-3.5 text-left last:border-b-0 hover:bg-muted/45"
      aria-pressed={theme.preference === option.value}
      onclick={() => theme.set(option.value)}
    >
      <Icon size={16} class="text-muted-foreground" />
      <span class="flex-1 text-sm">{option.label}</span>
      <span
        class={cn(
          "grid size-5 place-items-center rounded-full",
          theme.preference === option.value
            ? "bg-primary text-primary-foreground"
            : "text-transparent",
        )}
      >
        <Check size={12} strokeWidth={2.5} />
      </span>
    </button>
  {/each}
</div>
