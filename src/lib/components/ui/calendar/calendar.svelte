<script lang="ts">
  interface Props {
    value?: string | null;
    onValueChange?: (value: string) => void;
  }

  let { value = null, onValueChange }: Props = $props();

  const WEEKDAY_HEADERS = ["一", "二", "三", "四", "五", "六", "日"];

  function iso(year: number, month: number, day: number): string {
    return `${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  }

  const today = (() => {
    const now = new Date();
    return iso(now.getFullYear(), now.getMonth(), now.getDate());
  })();

  // svelte-ignore state_referenced_locally
  let viewYear = $state(Number((value ?? today).slice(0, 4)));
  // svelte-ignore state_referenced_locally
  let viewMonth = $state(Number((value ?? today).slice(5, 7)) - 1);

  const label = $derived(`${viewYear} 年 ${viewMonth + 1} 月`);
  const cells = $derived.by(() => {
    const lead = (new Date(viewYear, viewMonth, 1).getDay() + 6) % 7;
    const days = new Date(viewYear, viewMonth + 1, 0).getDate();
    const list: Array<{ iso: string; day: number } | null> = [];
    for (let i = 0; i < lead; i++) list.push(null);
    for (let day = 1; day <= days; day++) {
      list.push({ iso: iso(viewYear, viewMonth, day), day });
    }
    while (list.length % 7 !== 0) list.push(null);
    return list;
  });

  function shiftMonth(delta: number) {
    viewMonth += delta;
    if (viewMonth < 0) {
      viewMonth = 11;
      viewYear -= 1;
    } else if (viewMonth > 11) {
      viewMonth = 0;
      viewYear += 1;
    }
  }
</script>

<div class="w-64 select-none p-1.5" role="dialog" aria-label="选择日期">
  <div class="mb-1 flex items-center justify-between px-1">
    <button
      type="button"
      class="grid size-7 place-items-center rounded-md text-[15px] text-muted-foreground transition hover:bg-muted hover:text-foreground"
      aria-label="上个月"
      onclick={() => shiftMonth(-1)}
    >
      ‹
    </button>
    <span class="text-[13px] font-medium">{label}</span>
    <button
      type="button"
      class="grid size-7 place-items-center rounded-md text-[15px] text-muted-foreground transition hover:bg-muted hover:text-foreground"
      aria-label="下个月"
      onclick={() => shiftMonth(1)}
    >
      ›
    </button>
  </div>
  <div class="grid grid-cols-7 gap-0.5">
    {#each WEEKDAY_HEADERS as header}
      <span class="grid h-7 place-items-center text-[11px] text-muted-foreground">{header}</span>
    {/each}
    {#each cells as cell}
      {#if cell}
        <button
          type="button"
          class={`grid h-8 place-items-center rounded-md text-[13px] tabular-nums transition ${
            cell.iso === value
              ? "bg-primary font-semibold text-primary-foreground"
              : cell.iso === today
                ? "font-semibold text-primary hover:bg-muted"
                : "hover:bg-muted"
          }`}
          aria-pressed={cell.iso === value}
          onclick={() => onValueChange?.(cell.iso)}
        >
          {cell.day}
        </button>
      {:else}
        <span class="h-8"></span>
      {/if}
    {/each}
  </div>
</div>
