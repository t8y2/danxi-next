<script lang="ts">
  import { forumReplyExcerpt, type ForumReferenceTarget } from "$lib/features/forum/content";

  interface Props {
    target: ForumReferenceTarget | null;
    onOpen?: () => void;
  }

  let { target, onOpen }: Props = $props();
  const summary = $derived(
    target?.deleted ? "该楼层已删除" : target ? forumReplyExcerpt(target.content) || "内容暂不可见" : "引用的楼层暂不可见",
  );
</script>

{#snippet summaryContent()}
  <span class="block font-medium text-muted-foreground">
    回复{target?.floorNumber ? ` ${target.floorNumber}F` : "引用楼层"}{target?.anonyname.trim() ? ` · ${target.anonyname}` : ""}
  </span>
  <span class="line-clamp-2 break-words text-foreground/80">{summary}</span>
{/snippet}

{#if onOpen}
  <button type="button" class="focus-ring my-1.5 block w-full border-l-2 border-primary/35 bg-muted/45 px-3 py-1.5 text-left text-xs leading-5 transition-colors hover:bg-muted" aria-label="查看对话上下文" onclick={onOpen}>
    {@render summaryContent()}
    <span class="block text-[11px] text-primary">查看对话 ›</span>
  </button>
{:else}
  <span class="my-1.5 block border-l-2 border-primary/35 bg-muted/45 px-3 py-1.5 text-xs leading-5">{@render summaryContent()}</span>
{/if}
