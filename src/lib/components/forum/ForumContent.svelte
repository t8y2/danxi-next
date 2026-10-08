<script lang="ts">
  import { parseForumContent, resolveForumReference, type ForumContentNode, type ForumReferenceTarget } from "$lib/features/forum/content";
  import type { ForumFloorMention } from "$lib/types/app";
  import ForumImage from "./ForumImage.svelte";
  import ForumLink from "./ForumLink.svelte";
  import ForumReference from "./ForumReference.svelte";

  interface Props {
    content: string;
    mentions?: ForumFloorMention[];
    loadedFloors?: ForumReferenceTarget[];
    muted?: boolean;
    class?: string;
    onOpenReference?: (floorId: number) => void;
    hideReferences?: boolean;
  }

  let { content, mentions = [], loadedFloors = [], muted = false, class: className = "", onOpenReference, hideReferences = false }: Props = $props();
  const nodes = $derived(parseForumContent(content, { hideReferences }));
</script>

{#snippet renderNodes(items: ForumContentNode[], inLink = false)}
  {#each items as node}
    {#if node.type === "text"}
      {node.text}
    {:else if node.type === "paragraph"}
      <p>{@render renderNodes(node.children, inLink)}</p>
    {:else if node.type === "heading"}
      <svelte:element this={`h${node.depth}`}>{@render renderNodes(node.children, inLink)}</svelte:element>
    {:else if node.type === "strong"}
      <strong>{@render renderNodes(node.children, inLink)}</strong>
    {:else if node.type === "em"}
      <em>{@render renderNodes(node.children, inLink)}</em>
    {:else if node.type === "del"}
      <del>{@render renderNodes(node.children, inLink)}</del>
    {:else if node.type === "br"}
      <br />
    {:else if node.type === "hr"}
      <hr />
    {:else if node.type === "code"}
      <pre><code>{node.text}</code></pre>
    {:else if node.type === "codespan"}
      <code>{node.text}</code>
    {:else if node.type === "blockquote"}
      <blockquote>{@render renderNodes(node.children, inLink)}</blockquote>
    {:else if node.type === "link"}
      {#if node.url}
        <ForumLink url={node.url}>{@render renderNodes(node.children, true)}</ForumLink>
      {:else}
        {@render renderNodes(node.children, true)}
      {/if}
    {:else if node.type === "image"}
      {#if node.url}
        {#key node.url}
          <ForumImage url={node.url} alt={node.alt} linked={inLink} />
        {/key}
      {:else}
        <span class="text-xs text-muted-foreground">[图片地址不可用]</span>
      {/if}
    {:else if node.type === "reference"}
      <ForumReference target={resolveForumReference(node.floorId, mentions, loadedFloors)} onOpen={onOpenReference ? () => onOpenReference?.(node.floorId) : undefined} />
    {:else if node.type === "sticker"}
      {#key node.url}<ForumImage url={node.url} alt={node.alt} sticker />{/key}
    {:else if node.type === "list"}
      {#if node.ordered}
        <ol start={node.start}>{@render renderNodes(node.children, inLink)}</ol>
      {:else}
        <ul>{@render renderNodes(node.children, inLink)}</ul>
      {/if}
    {:else if node.type === "listItem"}
      <li>{node.checked === undefined ? "" : node.checked ? "☑ " : "☐ "}{@render renderNodes(node.children, inLink)}</li>
    {:else if node.type === "table"}
      <div class="overflow-x-auto">
        <table>
          <thead><tr>{#each node.header as cell}<th>{@render renderNodes(cell, inLink)}</th>{/each}</tr></thead>
          <tbody>{#each node.rows as row}<tr>{#each row as cell}<td>{@render renderNodes(cell, inLink)}</td>{/each}</tr>{/each}</tbody>
        </table>
      </div>
    {/if}
  {/each}
{/snippet}

<div class={`forum-content min-w-0 break-words text-[14px] leading-6 ${muted ? "text-muted-foreground" : "text-foreground/92"} ${className}`}>
  {@render renderNodes(nodes)}
</div>

<style>
  .forum-content { overflow-wrap: anywhere; }
  .forum-content :global(p) { margin: 0.5rem 0; white-space: pre-wrap; }
  .forum-content :global(h1), .forum-content :global(h2), .forum-content :global(h3),
  .forum-content :global(h4), .forum-content :global(h5), .forum-content :global(h6) {
    margin: 0.75rem 0 0.5rem; font-size: 1rem; font-weight: 600; line-height: 1.5;
  }
  .forum-content :global(ul), .forum-content :global(ol) { margin: 0.5rem 0; padding-left: 1.5rem; }
  .forum-content :global(ul) { list-style-type: disc; }
  .forum-content :global(ol) { list-style-type: decimal; }
  .forum-content :global(blockquote) { margin: 0.5rem 0; border-left: 2px solid hsl(var(--border)); padding-left: 0.75rem; color: hsl(var(--muted-foreground)); }
  .forum-content :global(pre) { overflow-x: auto; margin: 0.75rem 0; padding: 0.75rem; border-radius: 8px; background: hsl(var(--muted)); white-space: pre; }
  .forum-content :global(code) { padding: 0.1rem 0.25rem; border-radius: 4px; background: hsl(var(--muted)); font-size: 0.85em; }
  .forum-content :global(pre code) { padding: 0; background: none; }
  .forum-content :global(hr) { margin: 0.75rem 0; border: 0; border-top: 1px solid hsl(var(--border)); }
  .forum-content :global(table) { width: 100%; margin: 0.5rem 0; border-collapse: collapse; font-size: 0.85em; }
  .forum-content :global(th), .forum-content :global(td) { padding: 0.4rem 0.6rem; border: 1px solid hsl(var(--border)); text-align: left; }
  .forum-content :global(th) { background: hsl(var(--muted)); font-weight: 600; }
  .forum-content > :global(:first-child) { margin-top: 0; }
  .forum-content > :global(:last-child) { margin-bottom: 0; }
</style>
