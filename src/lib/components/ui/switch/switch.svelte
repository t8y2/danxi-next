<script lang="ts">
  import { Switch as SwitchPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  let {
    checked = $bindable(false),
    class: className,
    ...rest
  }: SwitchPrimitive.RootProps = $props();
</script>

<SwitchPrimitive.Root
  bind:checked
  class={cn(
    "focus-ring inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-transparent transition-colors disabled:opacity-45",
    checked ? "bg-primary" : "bg-muted",
    className,
  )}
  {...rest}
>
  <SwitchPrimitive.Thumb
    class={cn(
      "pointer-events-none block size-4 rounded-full bg-white shadow-sm transition-transform",
      checked ? "translate-x-4" : "translate-x-0",
    )}
  />
</SwitchPrimitive.Root>
