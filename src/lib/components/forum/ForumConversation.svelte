<script lang="ts">
  import { Dialog } from "bits-ui";
  import { LoaderCircle, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import type { ForumConversationController } from "$lib/features/forum/conversation.svelte";
  import { forumReplyExcerpt } from "$lib/features/forum/content";
  import ForumContent from "./ForumContent.svelte";

  interface Props { controller: ForumConversationController }
  let { controller }: Props = $props();
  let viewport = $state<HTMLDivElement>();
  const entries = $derived(controller.result.entries);
  const hasUnavailable = $derived(entries.some((entry) => entry.unavailable));

  function label(floorId: number): string {
    const floor = entries.find((entry) => entry.floorId === floorId)?.floor;
    return `${floor?.floorNumber ? `${floor.floorNumber}F · ` : ""}${floor?.anonyname.trim() || "引用楼层"}`;
  }

  function locate(floorId: number) {
    const element = viewport?.querySelector<HTMLElement>(`[data-conversation-floor="${floorId}"]`);
    element?.scrollIntoView({ block: "center" });
    element?.focus({ preventScroll: true });
  }
</script>

<Dialog.Root open={controller.opened} onOpenChange={(open) => { if (!open) controller.close(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/35" />
    <Dialog.Content class="fixed inset-y-0 right-0 z-50 flex w-full max-w-2xl flex-col border-l border-border bg-card shadow-xl outline-none">
      <header class="flex shrink-0 items-start gap-4 border-b border-border px-5 py-4">
        <div class="min-w-0 flex-1">
          <Dialog.Title class="m-0 text-base font-semibold">对话上下文</Dialog.Title>
          <Dialog.Description class="sr-only">查看这条回复的相关对话。</Dialog.Description>
        </div>
        <Dialog.Close class="focus-ring grid size-8 shrink-0 place-items-center rounded-lg text-muted-foreground hover:bg-muted" aria-label="关闭对话上下文"><X size={18} /></Dialog.Close>
      </header>
      <div bind:this={viewport} class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-5 py-2" aria-busy={controller.loading}>
        {#each entries as entry (entry.floorId)}
          <article tabindex="-1" data-conversation-floor={entry.floorId} style:margin-left={`${entry.depth * 12}px`} class={`border-b border-border py-4 outline-none focus:bg-primary/5 ${entry.floorId === controller.sourceId ? "border-l-2 border-l-primary pl-3" : ""}`}>
            <header class="flex flex-wrap items-center gap-2 text-xs">
              <span class="font-semibold text-foreground">{label(entry.floorId)}</span>
              {#if entry.floorId === controller.sourceId}<span class="text-primary">当前回复</span>{/if}
              {#if entry.floorId === controller.targetId}<span class="text-muted-foreground">引用起点</span>{/if}
            </header>
            {#if entry.replyToIds.length}
              <div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
                <span>回复</span>
                {#each entry.replyToIds as floorId}
                  <button type="button" class="focus-ring rounded-sm text-primary hover:underline disabled:text-muted-foreground" disabled={!entries.some((item) => item.floorId === floorId)} onclick={() => locate(floorId)}>{label(floorId)}</button>
                {/each}
              </div>
            {/if}
            {#if entry.floor?.deleted}
              <p class="mb-0 mt-2 text-sm text-muted-foreground">该楼层已删除</p>
            {:else if entry.unavailable}
              {#if entry.floor}<p class="mb-0 mt-2 text-sm">{forumReplyExcerpt(entry.floor.content)}</p>{/if}
              <p class="mb-0 mt-2 text-xs text-muted-foreground">{entry.floor ? "仅显示引用摘要，完整楼层暂时无法加载" : "楼层暂时无法加载或无权查看"}</p>
            {:else if entry.floor}
              <ForumContent content={entry.floor.content} hideReferences class="mt-2" />
            {/if}
          </article>
        {/each}
        {#if controller.loading}
          <p role="status" class="flex items-center justify-center gap-2 py-6 text-xs text-muted-foreground"><LoaderCircle class="animate-spin" size={14} /> 正在整理对话上下文</p>
        {/if}
        {#if !controller.loading && (hasUnavailable || controller.result.limited)}
          <div class="flex items-center justify-center gap-3 py-5">
            {#if hasUnavailable}<Button size="sm" variant="outline" onclick={() => controller.load()}>重试未加载楼层</Button>{/if}
            {#if controller.result.limited}<Button size="sm" variant="outline" onclick={() => controller.load(true)}>继续加载上下文</Button>{/if}
          </div>
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
