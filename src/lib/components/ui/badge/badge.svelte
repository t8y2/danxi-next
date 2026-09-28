<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";
  import { cn } from "$lib/utils";

  type Variant = "default" | "outline" | "success" | "warning" | "muted";
  type Props = HTMLAttributes<HTMLSpanElement> & {
    variant?: Variant;
    children?: Snippet;
  };

  const variants: Record<Variant, string> = {
    default: "bg-primary/10 text-primary",
    outline: "border border-border bg-card text-muted-foreground",
    success: "bg-emerald-500/11 text-emerald-700",
    warning: "bg-amber-500/13 text-amber-800",
    muted: "bg-muted text-muted-foreground",
  };

  let {
    variant = "default",
    class: className,
    children,
    ...rest
  }: Props = $props();
</script>

<span
  class={cn(
    "inline-flex items-center rounded-full px-2.5 py-1 text-[11px] font-semibold tracking-wide",
    variants[variant],
    className,
  )}
  {...rest}
>
  {@render children?.()}
</span>
