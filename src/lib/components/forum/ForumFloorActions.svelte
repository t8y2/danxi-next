<script lang="ts">
  import { Flag, Heart, MessageCircleReply, ThumbsDown } from "@lucide/svelte";
  import type { ForumFloor, ForumFloorPreview } from "$lib/types/app";

  interface Props {
    floor: ForumFloor | ForumFloorPreview;
    busy?: boolean;
    replyDisabled?: boolean;
    onReact?: (kind: "like" | "dislike") => void;
    onReply: () => void;
    onReport: () => void;
  }

  let { floor, busy = false, replyDisabled = false, onReact, onReply, onReport }: Props = $props();

  const completeFloor = $derived("liked" in floor ? floor : null);
</script>

<footer class="mt-3 flex items-center gap-1 text-[11px] text-muted-foreground">
  {#if completeFloor}
    <button
      type="button"
      class={`focus-ring inline-flex h-7 items-center gap-1 rounded-lg px-2 transition-colors hover:bg-muted hover:text-foreground ${completeFloor.liked ? "bg-primary/9 text-primary" : ""}`}
      aria-label={completeFloor.liked ? "取消点赞" : "点赞"}
      aria-pressed={completeFloor.liked}
      disabled={busy}
      onclick={() => onReact?.("like")}
    >
      <Heart size={12} fill={completeFloor.liked ? "currentColor" : "none"} />
      {completeFloor.like}
    </button>
    <button
      type="button"
      class={`focus-ring inline-flex h-7 items-center gap-1 rounded-lg px-2 transition-colors hover:bg-muted hover:text-foreground ${completeFloor.disliked ? "bg-destructive/8 text-destructive" : ""}`}
      aria-label={completeFloor.disliked ? "取消点踩" : "点踩"}
      aria-pressed={completeFloor.disliked}
      disabled={busy}
      onclick={() => onReact?.("dislike")}
    >
      <ThumbsDown size={12} fill={completeFloor.disliked ? "currentColor" : "none"} />
      {completeFloor.dislike}
    </button>
  {/if}
  <button
    type="button"
    class="focus-ring inline-flex h-7 items-center gap-1 rounded-lg px-2 transition-colors hover:bg-muted hover:text-foreground"
    disabled={replyDisabled}
    title={replyDisabled ? "讨论已锁定" : "回复这层"}
    onclick={onReply}
  >
    <MessageCircleReply size={12} /> 回复
  </button>
  <button
    type="button"
    class="focus-ring ml-auto inline-flex size-7 items-center justify-center rounded-lg transition-colors hover:bg-muted hover:text-foreground"
    aria-label="举报这条内容"
    onclick={onReport}
  >
    <Flag size={12} />
  </button>
</footer>
