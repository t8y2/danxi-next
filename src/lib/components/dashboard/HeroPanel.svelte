<script lang="ts">
  import CalendarCheck from "@lucide/svelte/icons/calendar-check";
  import CalendarOff from "@lucide/svelte/icons/calendar-off";
  import LogIn from "@lucide/svelte/icons/log-in";
  import MapPin from "@lucide/svelte/icons/map-pin";
  import { Button } from "$lib/components/ui/button";
  import { session } from "$lib/stores/session.svelte";
  import { coursePalette, timetable, todayCourses } from "$lib/stores/timetable.svelte";

  interface Props {
    onLogin: () => void;
  }

  let { onLogin }: Props = $props();

  const now = new Date();
  const dateLabel = new Intl.DateTimeFormat("zh-CN", {
    month: "long",
    day: "numeric",
    weekday: "long",
  }).format(now);

  const greeting = (() => {
    const hour = now.getHours();
    if (hour < 5) return "夜深了";
    if (hour < 11) return "早上好";
    if (hour < 14) return "中午好";
    if (hour < 18) return "下午好";
    return "晚上好";
  })();
  const personalizedGreeting = $derived(
    session.status.campusName ? `${greeting}，${session.status.campusName}～` : `${greeting}～`,
  );

  const readyTimetable = $derived(
    timetable.state.phase === "ready" ? timetable.state.timetable : null,
  );
  const todayList = $derived(
    readyTimetable
      ? todayCourses(readyTimetable, timetable.semesterStartOverride)
      : [],
  );
</script>

<section class="flex h-full flex-col rounded-xl border border-border bg-card p-5">
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

  <div class="mt-auto border-t border-border pt-4">
    {#if timetable.state.phase === "ready" && todayList.length > 0}
      <div class="space-y-2">
        <div class="flex items-center justify-between text-xs">
          <span class="font-medium text-foreground">今天有 {todayList.length} 门课</span>
          <span class="text-muted-foreground">按节次排列</span>
        </div>
        <div class="grid gap-1.5">
          {#each todayList.slice(0, 2) as course}
            {@const palette = coursePalette(course.courseName)}
            <article
              class="grid grid-cols-[52px_1fr] items-center gap-3 rounded-r-lg px-3 py-2.5"
              style={`background: ${palette.background}; border-left: 3px solid ${palette.bar};`}
            >
              <span class="text-xs font-semibold tabular-nums" style={`color: ${palette.text};`}>
                {course.startUnit}{course.startUnit === course.endUnit ? "" : `-${course.endUnit}`} 节
              </span>
              <div class="min-w-0">
                <p class="m-0 truncate text-sm font-medium text-foreground">{course.courseName}</p>
                {#if course.roomName}
                  <p class="mb-0 mt-0.5 flex items-center gap-1 truncate text-[11px] text-muted-foreground">
                    <MapPin size={11} class="shrink-0" /> {course.roomName}
                  </p>
                {/if}
              </div>
            </article>
          {/each}
        </div>
        {#if todayList.length > 2}
          <p class="m-0 text-right text-[11px] text-muted-foreground">另有 {todayList.length - 2} 门课程</p>
        {/if}
      </div>
    {:else if timetable.state.phase === "ready"}
      <div class="flex min-h-24 flex-col items-center justify-center gap-2 rounded-lg bg-muted/40 px-4 py-6 text-center">
        <CalendarCheck size={19} class="text-primary/80" />
        <p class="m-0 text-sm font-medium text-foreground">今天没有课程</p>
        <p class="m-0 text-xs text-muted-foreground">可以放松一下，或提前看看本周安排</p>
      </div>
    {:else if timetable.state.phase === "loading" || timetable.state.phase === "idle"}
      <div class="grid min-h-24 gap-2 rounded-lg bg-muted/40 px-4 py-4">
        <div class="h-3 w-20 animate-pulse rounded bg-muted"></div>
        <div class="h-9 animate-pulse rounded-lg bg-muted"></div>
      </div>
    {:else if timetable.state.phase === "error"}
      <div class="flex min-h-24 flex-col items-center justify-center gap-2 rounded-lg bg-muted/40 px-4 py-6 text-center">
        <CalendarOff size={18} class="text-muted-foreground/70" />
        <p class="m-0 text-sm font-medium text-foreground">今日日程加载失败</p>
        <p class="m-0 line-clamp-1 text-xs text-muted-foreground">{timetable.state.message}</p>
      </div>
    {:else}
      <div class="flex min-h-24 flex-col items-center justify-center gap-2 rounded-lg bg-muted/40 px-4 py-6 text-center">
        <CalendarOff size={18} class="text-muted-foreground/70" />
        <p class="m-0 text-xs text-muted-foreground">登录复旦 UIS 后显示今日日程</p>
      </div>
    {/if}
  </div>
</section>
