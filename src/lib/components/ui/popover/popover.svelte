<script lang="ts">
  import { Popover as PopoverPrimitive } from "bits-ui";
  import type { Snippet } from "svelte";

  interface Props {
    trigger: Snippet;
    content: Snippet;
    triggerClass?: string;
    align?: "start" | "center" | "end";
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
  }

  let {
    trigger,
    content,
    triggerClass = "",
    align = "start",
    open = $bindable(false),
    onOpenChange,
  }: Props = $props();
</script>

<PopoverPrimitive.Root
  {open}
  onOpenChange={(next) => {
    open = next;
    onOpenChange?.(next);
  }}
>
  <PopoverPrimitive.Trigger class={triggerClass}>
    {@render trigger()}
  </PopoverPrimitive.Trigger>
  <PopoverPrimitive.Content
    {align}
    side="bottom"
    sideOffset={6}
    class="z-50 rounded-lg border border-border bg-card p-1 shadow-lg outline-none"
  >
    {@render content()}
  </PopoverPrimitive.Content>
</PopoverPrimitive.Root>
