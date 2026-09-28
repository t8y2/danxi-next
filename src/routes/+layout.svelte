<script lang="ts">
  import { onMount } from "svelte";
  import "@unocss/reset/tailwind.css";
  import "virtual:uno.css";
  import "../app.css";
  import { theme } from "$lib/theme/theme.svelte";

  let { children } = $props();

  onMount(() => theme.init());
</script>

{@render children()}
