<script lang="ts">
  import type { Snippet } from "svelte";
  import { desktopWindow } from "$lib/api/window";

  interface Props {
    url: string;
    children: Snippet;
    class?: string;
  }

  let { url, children, class: className = "" }: Props = $props();
  let error = $state(false);

  async function open(event: MouseEvent) {
    if (!desktopWindow.available) return;
    event.preventDefault();
    error = false;
    try {
      await desktopWindow.openExternal(url);
    } catch {
      error = true;
    }
  }
</script>

<a href={url} target="_blank" rel="noopener noreferrer" referrerpolicy="no-referrer" class={`focus-ring text-primary underline underline-offset-3 ${className}`} onclick={open}>
  {@render children()}
</a>
{#if error}
  <span role="status" class="ml-2 text-xs text-destructive">链接未能打开，请重试</span>
{/if}
