<script lang="ts">
  import ForumLink from "./ForumLink.svelte";

  interface Props {
    url: string;
    alt: string;
    linked?: boolean;
    sticker?: boolean;
  }

  let { url, alt, linked = false, sticker = false }: Props = $props();
  let failed = $state(false);
</script>

{#snippet image()}
  {#if failed}
    <span class={sticker ? "inline-block text-xs text-muted-foreground" : "my-2 block rounded-lg border border-border bg-muted/40 px-3 py-4 text-xs text-muted-foreground"}>
      {sticker ? "[表情加载失败]" : `图片加载失败${alt ? ` · ${alt}` : ""}${linked ? "" : " · 点击打开原图"}`}
    </span>
  {:else}
    <img src={url} alt={alt || "帖子图片"} loading="lazy" decoding="async" referrerpolicy="no-referrer" class={sticker ? "inline-block size-12 object-contain align-middle" : "my-2 block max-h-100 max-w-full rounded-lg object-contain"} onerror={() => failed = true} />
  {/if}
{/snippet}

{#if linked || sticker}
  {@render image()}
{:else}
  <ForumLink {url} class="inline-block max-w-full align-middle no-underline">
    {@render image()}
  </ForumLink>
{/if}
