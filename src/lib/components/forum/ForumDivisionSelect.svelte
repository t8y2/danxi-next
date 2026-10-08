<script lang="ts">
  import { Check, ChevronDown } from "@lucide/svelte";
  import { Popover } from "$lib/components/ui/popover";
  import type { ForumDivision } from "$lib/types/app";

  interface Props {
    value: number | null;
    divisions: ForumDivision[];
    homepageLabel?: string | null;
    placeholder?: string;
    disabled?: boolean;
    compact?: boolean;
    ariaLabel?: string;
    onValueChange: (value: number | null) => void;
  }

  let {
    value,
    divisions,
    homepageLabel = null,
    placeholder = "选择分区",
    disabled = false,
    compact = false,
    ariaLabel = "选择茶楼分区",
    onValueChange,
  }: Props = $props();

  let open = $state(false);

  const selectedDivision = $derived(
    value == null ? null : divisions.find((division) => division.divisionId === value) ?? null,
  );
  const label = $derived(value == null ? (homepageLabel ?? placeholder) : (selectedDivision?.name ?? placeholder));

  function select(next: number | null) {
    onValueChange(next);
    open = false;
  }
</script>

<Popover
  bind:open
  align="start"
  {disabled}
  triggerClass={`focus-ring flex w-full items-center gap-2 rounded-lg border border-border bg-background text-left text-foreground transition hover:bg-muted disabled:pointer-events-none disabled:opacity-45 ${
    compact ? "h-8 px-2.5 text-xs" : "h-9 px-3 text-sm"
  }`}
>
  {#snippet trigger()}
    <span class="min-w-0 flex-1 truncate">{label}</span>
    <ChevronDown size={compact ? 13 : 14} class="shrink-0 text-muted-foreground" />
    <span class="sr-only">{ariaLabel}</span>
  {/snippet}
  {#snippet content()}
    <div class="grid max-h-72 min-w-64 gap-0.5 overflow-y-auto" aria-label={ariaLabel}>
      {#if homepageLabel !== null}
        <button
          type="button"
          class={`grid appearance-none grid-cols-[1fr_auto] items-center gap-3 rounded-md border-0 px-3 py-2 text-left outline-none transition focus:bg-muted focus:text-foreground ${
            value === null
              ? "bg-secondary text-foreground"
              : "bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
          onclick={() => select(null)}
        >
          <span class="text-xs font-medium">{homepageLabel}</span>
          {#if value === null}<Check size={13} class="text-primary" />{/if}
        </button>
      {/if}
      {#each divisions as division (division.divisionId)}
        <button
          type="button"
          class={`grid appearance-none grid-cols-[1fr_auto] items-center gap-3 rounded-md border-0 px-3 py-2 text-left outline-none transition focus:bg-muted focus:text-foreground ${
            value === division.divisionId
              ? "bg-secondary text-foreground"
              : "bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground"
          }`}
          onclick={() => select(division.divisionId)}
        >
          <span class="min-w-0 truncate text-xs font-medium">{division.name}</span>
          {#if value === division.divisionId}<Check size={13} class="text-primary" />{/if}
        </button>
      {/each}
      {#if divisions.length === 0 && homepageLabel === null}
        <p class="m-0 px-3 py-3 text-xs text-muted-foreground">暂无可用分区</p>
      {/if}
    </div>
  {/snippet}
</Popover>
