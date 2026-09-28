<script lang="ts">
  import LogIn from "@lucide/svelte/icons/log-in";
  import MoonStar from "@lucide/svelte/icons/moon-star";
  import Sun from "@lucide/svelte/icons/sun";
  import { Button } from "$lib/components/ui/button";
  import { session } from "$lib/stores/session.svelte";

  interface Props {
    onLogin: () => void;
  }

  let { onLogin }: Props = $props();

  const now = new Date();
  const hour = now.getHours();
  const weekdayLabels = ["一", "二", "三", "四", "五", "六", "日"];
  const todayIndex = (now.getDay() + 6) % 7;
  const nightTime = hour < 5 || hour >= 18;
  const dateLabel = new Intl.DateTimeFormat("zh-CN", {
    month: "long",
    day: "numeric",
    weekday: "long",
  }).format(now);

  const greeting = (() => {
    if (hour < 5) return "夜深了";
    if (hour < 11) return "早上好";
    if (hour < 14) return "中午好";
    if (hour < 18) return "下午好";
    return "晚上好";
  })();
  const personalizedGreeting = $derived(
    session.status.campusName ? `${greeting}，${session.status.campusName}～` : `${greeting}～`,
  );
</script>

<section class="relative flex h-full min-h-64 flex-col overflow-hidden rounded-xl border border-border bg-card p-5">
  <div class="flex flex-col justify-between gap-5 sm:flex-row sm:items-start">
    <div>
      <div class="text-xs text-muted-foreground">
        <span>{dateLabel}</span>
      </div>
      <h1 class="mb-0 mt-2 text-[28px] font-semibold tracking-[-0.035em] lg:text-[32px]">
        {personalizedGreeting}
      </h1>
    </div>
    {#if session.ready && !session.status.communityLoggedIn && !session.status.campusLoggedIn}
      <Button size="sm" class="self-start" onclick={onLogin}>
        <LogIn size={15} /> 登录旦挞账号
      </Button>
    {/if}
  </div>

  <div
    aria-hidden="true"
    class="pointer-events-none absolute bottom-16 right-6 text-primary/[0.09]"
  >
    {#if nightTime}
      <MoonStar size={92} strokeWidth={1.15} />
    {:else}
      <Sun size={92} strokeWidth={1.15} />
    {/if}
  </div>

  <div class="relative mt-auto border-t border-border pt-4">
    <div class="mb-2.5 flex items-center justify-between text-[11px] text-muted-foreground">
      <span>这一周</span>
      <span>星期{weekdayLabels[todayIndex]}</span>
    </div>
    <div class="grid grid-cols-7 gap-2" aria-label={`今天是星期${weekdayLabels[todayIndex]}`}>
      {#each weekdayLabels as label, index}
        <div class="flex min-w-0 flex-col gap-1.5">
          <span
            class={`h-1.5 rounded-full ${index === todayIndex ? "bg-primary" : index < todayIndex ? "bg-primary/25" : "bg-muted"}`}
          ></span>
          <span
            class={`text-center text-[10px] ${index === todayIndex ? "font-semibold text-primary" : "text-muted-foreground/75"}`}
          >{label}</span>
        </div>
      {/each}
    </div>
  </div>
</section>
