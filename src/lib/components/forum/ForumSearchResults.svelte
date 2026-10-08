<script lang="ts">
  import { untrack } from "svelte";
  import { ChevronRight, LoaderCircle } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { forumReplyExcerpt } from "$lib/features/forum/content";
  import type { ForumSearchController } from "$lib/features/forum/search.svelte";

  interface Props {
    controller: ForumSearchController;
    selectedHoleId: number | null;
    onOpenHole: (holeId: number) => void;
    onLogin: () => void;
  }

  let { controller, selectedHoleId, onOpenHole, onLogin }: Props = $props();
  let viewport = $state<HTMLDivElement>();
  const results = $derived(controller.state.phase === "ready" ? controller.state.floors.map((floor) => ({
    floor,
    excerpt: floor.deleted ? "该楼层已删除" : forumReplyExcerpt(floor.content) || "内容暂不可见",
  })) : []);

  $effect(() => {
    if (viewport && selectedHoleId === null) viewport.scrollTop = untrack(() => controller.scrollTop);
  });

  function openResult(holeId: number) {
    if (viewport) controller.scrollTop = viewport.scrollTop;
    onOpenHole(holeId);
  }

  function scroll(event: Event) {
    const element = event.currentTarget as HTMLDivElement;
    if (!element.clientHeight) return;
    controller.scrollTop = element.scrollTop;
    if (element.scrollHeight - element.scrollTop - element.clientHeight < 200) void controller.loadMore();
  }

  function dateLabel(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "" : new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "numeric", day: "numeric" }).format(date);
  }
</script>

<div class="flex shrink-0 items-center gap-2 border-b border-border px-4 py-2.5 text-xs text-muted-foreground">
  <span class="min-w-0 truncate" title={controller.query}>“{controller.query}”</span>
  <span class="shrink-0">· 全部茶楼</span>
</div>
{#if controller.state.phase === "loading"}
  <div role="status" class="flex flex-1 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle size={16} class="animate-spin" /> 正在搜索</div>
{:else if controller.state.phase === "error"}
  <div role="alert" class="flex flex-1 flex-col items-center justify-center gap-3 px-5 text-center">
    <p class="m-0 text-sm text-muted-foreground">{controller.state.message}</p>
    <Button variant="outline" size="sm" onclick={() => controller.retry()}>重试搜索</Button>
  </div>
{:else if controller.state.phase === "unauthenticated"}
  <div class="flex flex-1 flex-col items-center justify-center gap-3 px-5 text-center">
    <p class="m-0 text-sm text-muted-foreground">登录已失效，请重新登录后搜索</p>
    <Button size="sm" onclick={onLogin}>登录茶楼</Button>
  </div>
{:else if controller.state.phase === "ready"}
  <div bind:this={viewport} role="region" aria-label="茶楼搜索结果" class="min-h-0 flex-1 overflow-y-auto overscroll-contain" onscroll={scroll}>
    {#each results as { floor, excerpt } (floor.floorId)}
      <button type="button" aria-label={`查看帖子 #${floor.holeId}：${excerpt.slice(0, 60)}`} aria-current={selectedHoleId === floor.holeId ? "true" : undefined} class={`focus-ring group block w-full border-b border-border px-4 py-3.5 text-left transition-colors ${selectedHoleId === floor.holeId ? "bg-primary/[0.11]" : "hover:bg-muted/55"}`} onclick={() => openResult(floor.holeId)}>
        <span class="flex items-center gap-2 text-[11px] text-muted-foreground">
          <span class="font-medium tabular-nums">#{floor.holeId}</span>
          <span class="min-w-0 truncate">{floor.anonyname || "匿名同学"}</span>
          <time class="ml-auto shrink-0">{dateLabel(floor.timeCreated)}</time>
        </span>
        <span class="mt-2.5 block line-clamp-3 break-words text-sm leading-6">{excerpt}</span>
        <span class="mt-2 flex items-center gap-1 text-[11px] text-muted-foreground">查看帖子 <ChevronRight size={12} /></span>
      </button>
    {/each}
    {#if results.length === 0}
      <p role="status" class="m-0 px-4 py-12 text-center text-sm leading-6 text-muted-foreground">没有找到相关内容，换个关键词试试</p>
    {:else if controller.state.loadingMore}
      <p role="status" class="flex items-center justify-center gap-2 py-4 text-xs text-muted-foreground"><LoaderCircle size={14} class="animate-spin" /> 正在加载更多</p>
    {:else if controller.state.moreError}
      <div class="px-4 py-4 text-center">
        <p role="alert" class="mb-2 mt-0 text-xs text-muted-foreground">{controller.state.moreError}</p>
        <Button variant="outline" size="sm" onclick={() => controller.loadMore(true)}>重试加载更多</Button>
      </div>
    {:else if controller.state.nextOffset != null}
      <div class="py-2 text-center"><Button variant="ghost" size="sm" onclick={() => controller.loadMore()}>加载更多</Button></div>
    {:else}
      <p class="m-0 py-4 text-center text-xs text-muted-foreground">已显示全部结果</p>
    {/if}
  </div>
{/if}
