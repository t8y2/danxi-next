<script lang="ts">
  import { LoaderCircle, Send, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import ForumDivisionSelect from "$lib/components/forum/ForumDivisionSelect.svelte";
  import type { ForumActionResult } from "$lib/stores/session.svelte";
  import type { ForumDivision, ForumTag } from "$lib/types/app";

  type ComposerMode = "post" | "reply" | "report";

  interface SubmitPayload {
    content: string;
    divisionId: number | null;
    tags: ForumTag[];
  }

  interface Props {
    mode: ComposerMode;
    divisions?: ForumDivision[];
    tags?: ForumTag[];
    initialDivisionId?: number | null;
    targetLabel?: string;
    onCancel: () => void;
    onSubmit: (payload: SubmitPayload) => Promise<ForumActionResult>;
  }

  let {
    mode,
    divisions = [],
    tags = [],
    initialDivisionId = null,
    targetLabel = "",
    onCancel,
    onSubmit,
  }: Props = $props();

  let content = $state("");
  let divisionId = $state<number | null>(null);
  let selectedTagIds = $state<number[]>([]);
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let textarea = $state<HTMLTextAreaElement>();

  const title = $derived(mode === "post" ? "发布讨论" : mode === "reply" ? "回复讨论" : "举报内容");
  const description = $derived(
    mode === "post"
      ? "选择分区后发布，内容将以匿名身份展示。"
      : mode === "reply"
        ? targetLabel || "回复当前讨论"
        : targetLabel || "请说明举报原因",
  );
  const submitLabel = $derived(mode === "post" ? "发布" : mode === "reply" ? "回复" : "提交举报");
  const availableTags = $derived(tags.filter((tag) => tag.tagId !== 0 && tag.name !== "默认"));
  const canSubmit = $derived(
    content.trim().length > 0 && !submitting && (mode !== "post" || divisionId !== null),
  );

  $effect(() => {
    if (mode === "post" && divisionId === null) {
      divisionId = initialDivisionId ?? divisions[0]?.divisionId ?? null;
    }
    queueMicrotask(() => textarea?.focus());
  });

  function toggleTag(tagId: number) {
    selectedTagIds = selectedTagIds.includes(tagId)
      ? selectedTagIds.filter((id) => id !== tagId)
      : selectedTagIds.length < 3
        ? [...selectedTagIds, tagId]
        : selectedTagIds;
  }

  async function submit() {
    if (!canSubmit) return;
    submitting = true;
    error = null;
    const result = await onSubmit({
      content: content.trim(),
      divisionId,
      tags: availableTags.filter((tag) => selectedTagIds.includes(tag.tagId)),
    });
    submitting = false;
    if (result.ok) {
      onCancel();
    } else {
      error = result.message;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && !submitting) onCancel();
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      void submit();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-50 grid place-items-center bg-black/38 px-4 py-8"
  role="presentation"
  onclick={(event) => {
    if (event.currentTarget === event.target && !submitting) onCancel();
  }}
>
  <div
    class="flex max-h-full w-full max-w-xl flex-col overflow-hidden rounded-xl border border-border bg-card shadow-xl"
    role="dialog"
    aria-modal="true"
    aria-labelledby="forum-composer-title"
  >
    <header class="flex min-h-13 items-center gap-3 border-b border-border px-4 py-2.5">
      <div class="min-w-0 flex-1">
        <h2 id="forum-composer-title" class="m-0 text-[15px] font-semibold">{title}</h2>
        <p class="mb-0 mt-0.5 truncate text-xs text-muted-foreground">{description}</p>
      </div>
      <Button
        variant="ghost"
        size="icon"
        class="size-8"
        aria-label="关闭"
        disabled={submitting}
        onclick={onCancel}
      >
        <X size={15} />
      </Button>
    </header>

    <form
      class="min-h-0 overflow-y-auto p-4"
      onsubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      {#if mode === "post"}
        <span class="block text-xs font-medium text-foreground">分区</span>
        <div class="mt-2">
          <ForumDivisionSelect
            value={divisionId}
            {divisions}
            disabled={submitting}
            onValueChange={(value) => (divisionId = value)}
          />
        </div>

        {#if availableTags.length > 0}
          <fieldset class="mt-4 border-0 p-0">
            <legend class="text-xs font-medium text-foreground">标签（最多 3 个）</legend>
            <div class="mt-2 flex max-h-24 flex-wrap gap-1.5 overflow-y-auto">
              {#each availableTags as tag (tag.tagId)}
                <button
                  type="button"
                  class={`focus-ring rounded-lg border px-2.5 py-1 text-xs transition-colors ${
                    selectedTagIds.includes(tag.tagId)
                      ? "border-primary/35 bg-primary/10 text-primary"
                      : "border-border bg-background text-muted-foreground hover:text-foreground"
                  }`}
                  aria-pressed={selectedTagIds.includes(tag.tagId)}
                  disabled={submitting}
                  onclick={() => toggleTag(tag.tagId)}
                >
                  {tag.name}
                </button>
              {/each}
            </div>
          </fieldset>
        {/if}
      {/if}

      <label class="mt-4 block text-xs font-medium text-foreground" for="forum-content">
        {mode === "report" ? "举报原因" : "内容"}
      </label>
      <textarea
        id="forum-content"
        bind:this={textarea}
        bind:value={content}
        rows={mode === "report" ? 5 : 9}
        class="focus-ring mt-2 w-full resize-y rounded-lg border border-border bg-background px-3 py-2.5 text-sm leading-6 text-foreground placeholder:text-muted-foreground"
        placeholder={mode === "post" ? "写下想和大家讨论的内容…" : mode === "reply" ? "写下你的回复…" : "请具体说明需要处理的问题…"}
        disabled={submitting}
      ></textarea>

      {#if error}
        <p class="mb-0 mt-2 text-xs leading-5 text-destructive">{error}</p>
      {/if}

      <footer class="mt-4 flex items-center justify-between gap-3">
        <span class="text-[11px] text-muted-foreground">⌘/Ctrl + Enter 提交</span>
        <div class="flex items-center gap-2">
          <Button type="button" variant="ghost" size="sm" disabled={submitting} onclick={onCancel}>
            取消
          </Button>
          <Button type="submit" size="sm" disabled={!canSubmit}>
            {#if submitting}
              <LoaderCircle class="animate-spin" size={14} /> 提交中
            {:else}
              <Send size={14} /> {submitLabel}
            {/if}
          </Button>
        </div>
      </footer>
    </form>
  </div>
</div>
