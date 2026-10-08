<script lang="ts">
  import { Search, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import type { ForumSearchController } from "$lib/features/forum/search.svelte";

  interface Props {
    controller: ForumSearchController;
    onOpenHole: (holeId: number) => void;
    onSearch: () => void;
  }

  let { controller, onOpenHole, onSearch }: Props = $props();
  const inputId = $props.id();

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const intent = controller.submit();
    if (intent.kind === "hole") onOpenHole(intent.holeId);
    if (intent.kind === "search" || intent.kind === "empty") onSearch();
  }
</script>

<form role="search" aria-label="茶楼搜索" class="shrink-0 border-b border-border px-3 py-2.5" onsubmit={submit}>
  <div class="flex items-center gap-1.5">
    <div class="relative min-w-0 flex-1">
      <Search size={14} class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
      <input id={inputId} type="search" aria-label="搜索茶楼或输入帖子编号" aria-invalid={controller.inputError ? "true" : undefined} aria-describedby={controller.inputError ? `${inputId}-error` : undefined} bind:value={controller.input} placeholder="搜索茶楼，或输入 #帖子编号" autocomplete="off" class="focus-ring h-9 w-full rounded-lg border border-border bg-background/50 pl-8 pr-2 text-sm placeholder:text-muted-foreground" />
    </div>
    <Button type="submit" size="sm" variant="secondary">搜索</Button>
    {#if controller.query}
      <Button type="button" size="icon" variant="ghost" class="size-8 shrink-0" aria-label="退出搜索" onclick={() => { controller.clear(); onSearch(); }}><X size={15} /></Button>
    {/if}
  </div>
  {#if controller.inputError}
    <p id={`${inputId}-error`} role="alert" class="mb-0 mt-2 text-xs text-destructive">{controller.inputError}</p>
  {/if}
</form>
