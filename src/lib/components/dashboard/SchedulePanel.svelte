<script lang="ts">
  import CalendarOff from "@lucide/svelte/icons/calendar-off";
  import CalendarRange from "@lucide/svelte/icons/calendar-range";
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import MapPin from "@lucide/svelte/icons/map-pin";
  import UserRound from "@lucide/svelte/icons/user-round";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import { Calendar } from "$lib/components/ui/calendar";
  import { Popover } from "$lib/components/ui/popover";
  import { session } from "$lib/stores/session.svelte";
  import {
    WEEKDAY_LABELS,
    coursePalette,
    currentWeek,
    dateOfWeekday,
    effectiveStart,
    maxWeekOf,
    todayCourses,
    timetable,
    weekCourses,
    weekdayOfToday,
  } from "$lib/stores/timetable.svelte";

  interface Props {
    expanded?: boolean;
    onLogin: () => void;
  }

  const PERIOD_HEIGHT = 52;
  const PERIOD_START_TIMES = [
    "08:00",
    "08:55",
    "09:55",
    "10:50",
    "11:45",
    "13:30",
    "14:25",
    "15:25",
    "16:20",
    "17:15",
    "18:30",
    "19:25",
    "20:20",
    "21:15",
    "22:10",
  ];

  let { expanded = false, onLogin }: Props = $props();

  $effect(() => {
    if (!session.ready) return;
    if (session.status.campusLoggedIn) {
      void timetable.load();
    } else {
      timetable.requireLogin();
    }
  });

  const ready = $derived(timetable.state.phase === "ready" ? timetable.state.timetable : null);
  const start = $derived(ready ? effectiveStart(ready, timetable.semesterStartOverride) : null);
  const thisWeek = $derived(currentWeek(start));
  const maxWeek = $derived(ready ? maxWeekOf(ready) : 1);
  const todayList = $derived(ready ? todayCourses(ready, timetable.semesterStartOverride) : []);
  const weekly = $derived(ready ? weekCourses(ready, timetable.selectedWeek) : null);
  const showingCurrentWeek = $derived(thisWeek === timetable.selectedWeek);
  const periodCount = $derived(
    Math.max(14, ...((ready?.courses ?? []).map((course) => course.endUnit)), 1),
  );
  const periods = $derived(
    Array.from({ length: periodCount }, (_, index) => ({
      unit: index + 1,
      time: PERIOD_START_TIMES[index] ?? "",
    })),
  );
  const weekRange = $derived.by(() => {
    const first = dateOfWeekday(start, timetable.selectedWeek, 1);
    const last = dateOfWeekday(start, timetable.selectedWeek, 7);
    return first && last ? `${first} — ${last}` : null;
  });

  let startDateOpen = $state(false);
  let startDateDraft = $state<string | null>(null);
  const pendingStartDate = $derived(
    startDateDraft !== null && startDateDraft !== start,
  );

  function applyStartDate() {
    if (!pendingStartDate || !startDateDraft) return;
    timetable.setSemesterStart(startDateDraft);
    startDateDraft = null;
    startDateOpen = false;
  }

  function selectStartDate(value: string) {
    startDateDraft = value;
  }

  function courseTop(startUnit: number): number {
    return (startUnit - 1) * PERIOD_HEIGHT;
  }

  function courseHeight(startUnit: number, endUnit: number): number {
    return Math.max(PERIOD_HEIGHT, (endUnit - startUnit + 1) * PERIOD_HEIGHT);
  }
</script>

<section
  class={`flex flex-col overflow-hidden ${expanded ? "min-h-full bg-background" : "h-full rounded-xl border border-border bg-card p-5"}`}
>
  {#if expanded}
    <header class="flex h-11 shrink-0 items-center justify-between gap-2 overflow-hidden border-b border-border px-3 lg:px-5">
      <div class="hidden min-w-0 items-center gap-2 text-xs text-muted-foreground sm:flex">
        {#if ready}
          <Badge variant="outline" class="tabular-nums text-foreground">第 {timetable.selectedWeek} 周</Badge>
          {#if showingCurrentWeek}
            <Badge variant="success">本周</Badge>
          {/if}
          <span class="hidden lg:inline">共 {maxWeek} 周</span>
          {#if weekRange}<span class="hidden tabular-nums md:inline">{weekRange}</span>{/if}
        {:else}
          <span>复旦教务数据</span>
        {/if}
      </div>

      {#if ready}
      <div class="ml-auto flex shrink-0 items-center gap-1.5">
        <Popover
          bind:open={startDateOpen}
          align="end"
          triggerClass="flex h-8 items-center gap-1.5 rounded-lg border border-border bg-background px-2.5 text-xs text-muted-foreground shadow-sm transition hover:bg-muted hover:text-foreground"
        >
          {#snippet trigger()}
            <CalendarRange size={13} />
            <span class="hidden sm:inline">学期日期</span>
          {/snippet}
          {#snippet content()}
            <div class="w-64">
              <div class="px-2 pb-1.5 pt-1">
                <p class="m-0 text-xs font-semibold text-foreground">选择开学第一周的周一</p>
                <p class="mb-0 mt-1 text-[11px] leading-4 text-muted-foreground">
                  用于计算当前周次和每天对应的日期。
                </p>
              </div>
              <Calendar
                value={startDateDraft ?? start}
                onValueChange={selectStartDate}
              />
              <div class="flex items-center justify-between gap-3 border-t border-border px-2 py-2">
                <div class="min-w-0 text-[11px] text-muted-foreground">
                  {#if pendingStartDate}
                    <Badge variant="warning" class="mr-1.5 px-2 py-0.5">待保存</Badge>
                    <span class="tabular-nums">{startDateDraft}</span>
                  {:else if start}
                    <span>当前：<span class="tabular-nums text-foreground">{start}</span></span>
                  {:else}
                    <span>请选择日期</span>
                  {/if}
                </div>
                {#if pendingStartDate}
                  <Button size="sm" class="h-7 px-2.5 text-xs" onclick={applyStartDate}>保存</Button>
                {/if}
              </div>
            </div>
          {/snippet}
        </Popover>
        {#if thisWeek !== null && !showingCurrentWeek}
          <Button
            variant="ghost"
            size="sm"
            class="h-8 text-primary"
            onclick={() => timetable.goToWeek(thisWeek, maxWeek)}
          >
            回到本周
          </Button>
        {/if}
        <div class="flex h-8 items-center rounded-lg border border-border bg-background p-0.5 shadow-sm">
          <button
            type="button"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-muted hover:text-foreground disabled:opacity-30"
            aria-label="上一周"
            disabled={timetable.selectedWeek <= 1}
            onclick={() => timetable.goToWeek(timetable.selectedWeek - 1, maxWeek)}
          >
            <ChevronLeft size={15} />
          </button>
          <span class="min-w-[4.75rem] px-1.5 text-center text-xs font-semibold tabular-nums">
            第 {timetable.selectedWeek} 周
          </span>
          <button
            type="button"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-muted hover:text-foreground disabled:opacity-30"
            aria-label="下一周"
            disabled={timetable.selectedWeek >= maxWeek}
            onclick={() => timetable.goToWeek(timetable.selectedWeek + 1, maxWeek)}
          >
            <ChevronRight size={15} />
          </button>
        </div>
      </div>
      {:else if timetable.state.phase === "error"}
        <Button variant="ghost" size="sm" class="text-muted-foreground" onclick={() => timetable.load()}>
          重试
        </Button>
      {/if}
    </header>
  {:else}
    <header class="mb-4 flex items-center justify-between">
      <div>
        <h2 class="m-0 text-[16px] font-semibold tracking-[-0.015em]">今日日程</h2>
        <p class="mb-0 mt-1 text-xs text-muted-foreground">
          {#if ready}
            {thisWeek ? `第 ${thisWeek} 周` : "本周"} · {todayList.length} 节课
          {:else}
            复旦教务数据
          {/if}
        </p>
      </div>
      {#if timetable.state.phase === "error"}
        <Button variant="ghost" size="sm" class="text-muted-foreground" onclick={() => timetable.load()}>
          重试
        </Button>
      {/if}
    </header>
  {/if}

  {#if timetable.state.phase === "loading" || timetable.state.phase === "idle"}
    <div class={`${expanded ? "p-5 lg:p-6" : "border-t border-border"}`}>
      <div class={`grid gap-3 ${expanded ? "grid-cols-2 md:grid-cols-4" : ""}`}>
        {#each Array(expanded ? 8 : 3) as _}
          <div class="rounded-lg border border-border/70 p-3">
            <div class="h-3 w-16 animate-pulse rounded bg-muted"></div>
            <div class="mt-2 h-3.5 w-3/5 animate-pulse rounded bg-muted"></div>
          </div>
        {/each}
      </div>
    </div>
  {:else if timetable.state.phase === "unauthenticated"}
    <button
      type="button"
      class={`flex cursor-pointer flex-col items-center justify-center gap-3 border-dashed text-muted-foreground transition hover:border-primary/50 hover:text-foreground ${expanded ? "m-5 min-h-72 rounded-xl border bg-background/35 lg:m-6" : "flex-1 rounded-lg border py-10"}`}
      onclick={onLogin}
    >
      <span class="grid size-11 place-items-center rounded-full bg-muted"><CalendarOff size={20} /></span>
      <span class="text-sm font-medium text-foreground">登录复旦统一身份认证</span>
      <span class="text-xs">同步真实学期、周次与课程安排</span>
    </button>
  {:else if timetable.state.phase === "error"}
    <div class={`flex flex-col items-center justify-center gap-3 text-muted-foreground ${expanded ? "min-h-72" : "flex-1 py-10"}`}>
      <p class="m-0 text-sm">{timetable.state.message}</p>
      <Button variant="outline" size="sm" onclick={() => timetable.load()}>重试</Button>
    </div>
  {:else if expanded}
    <div class="min-h-0 flex-1">
      {#if weekly && weekly.size === 0}
        <div class="flex min-h-72 flex-col items-center justify-center gap-2 border-b border-border bg-background/35 text-muted-foreground">
          <CalendarOff size={24} class="text-muted-foreground/60" />
          <p class="m-0 text-sm">第 {timetable.selectedWeek} 周没有课程</p>
        </div>
      {:else}
        <div class="overflow-hidden border-b border-border bg-background/40">
          <div class="overflow-x-auto">
            <div class="min-w-[960px]">
              <div class="grid grid-cols-[64px_repeat(7,minmax(126px,1fr))] border-b border-border bg-muted/25">
                <div class="flex items-end justify-center border-r border-border px-2 py-3 text-[10px] font-medium text-muted-foreground">
                  节次
                </div>
                {#each WEEKDAY_LABELS as label, index}
                  {@const day = index + 1}
                  {@const dayDate = dateOfWeekday(start, timetable.selectedWeek, day)}
                  {@const isToday = showingCurrentWeek && weekdayOfToday() === day}
                  <div class={`border-r border-border px-2 py-2.5 text-center last:border-r-0 ${isToday ? "bg-primary/[0.06]" : ""}`}>
                    <div class="flex items-center justify-center gap-1.5">
                      <span class={`text-xs font-semibold ${isToday ? "text-primary" : "text-foreground"}`}>{label}</span>
                      {#if isToday}
                        <Badge class="px-1.5 py-0 text-[9px] leading-4">今天</Badge>
                      {/if}
                    </div>
                    <p class={`mb-0 mt-1 text-[10px] tabular-nums ${isToday ? "text-primary/80" : "text-muted-foreground"}`}>
                      {dayDate ?? "日期待设定"}
                    </p>
                  </div>
                {/each}
              </div>

              <div class="grid grid-cols-[64px_repeat(7,minmax(126px,1fr))]">
                <div class="border-r border-border bg-muted/15">
                  {#each periods as period}
                    <div
                      class={`flex flex-col items-center justify-center border-b border-border/70 last:border-b-0 ${period.unit === 6 || period.unit === 11 ? "border-t border-t-border" : ""}`}
                      style={`height: ${PERIOD_HEIGHT}px;`}
                    >
                      <span class="text-[11px] font-semibold tabular-nums text-foreground/80">{period.unit}</span>
                      {#if period.time}<span class="mt-0.5 text-[9px] tabular-nums text-muted-foreground">{period.time}</span>{/if}
                    </div>
                  {/each}
                </div>

                {#each WEEKDAY_LABELS as _, index}
                  {@const day = index + 1}
                  {@const dayCourses = weekly?.get(day) ?? []}
                  {@const isToday = showingCurrentWeek && weekdayOfToday() === day}
                  <div
                    class={`relative border-r border-border last:border-r-0 ${isToday ? "bg-primary/[0.025]" : ""}`}
                    style={`height: ${periodCount * PERIOD_HEIGHT}px;`}
                  >
                    {#each periods as period}
                      <div
                        class={`absolute inset-x-0 border-b border-border/55 ${period.unit === 5 || period.unit === 10 ? "border-b-border" : ""}`}
                        style={`top: ${(period.unit - 1) * PERIOD_HEIGHT}px; height: ${PERIOD_HEIGHT}px;`}
                      ></div>
                    {/each}

                    {#each dayCourses as course}
                      {@const palette = coursePalette(course.courseName)}
                      {@const span = course.endUnit - course.startUnit + 1}
                      <article
                        class="absolute inset-x-0 z-10 overflow-hidden px-2.5 py-2 text-left transition-colors duration-150 hover:z-20"
                        style={`top: ${courseTop(course.startUnit)}px; height: ${courseHeight(course.startUnit, course.endUnit)}px; background: color-mix(in srgb, ${palette.bar} 13%, hsl(var(--background))); border-right: 1px solid color-mix(in srgb, ${palette.bar} 18%, hsl(var(--border))); border-bottom: 1px solid color-mix(in srgb, ${palette.bar} 18%, hsl(var(--border))); border-left: 3px solid ${palette.bar};`}
                        title={`${course.courseName} · ${course.startUnit}-${course.endUnit} 节${course.roomName ? ` · ${course.roomName}` : ""}`}
                      >
                        <div class="flex items-start justify-between gap-1.5">
                          <h3 class="m-0 min-w-0 flex-1 truncate text-[12px] font-semibold leading-4 text-foreground">{course.courseName}</h3>
                          <span class="shrink-0 text-[9px] font-semibold tabular-nums" style={`color: ${palette.text};`}>
                            {course.startUnit}{course.startUnit !== course.endUnit ? `-${course.endUnit}` : ""}
                          </span>
                        </div>
                        {#if course.roomName}
                          <p class="mb-0 mt-1 flex min-w-0 items-center gap-1 truncate text-[10px] leading-3.5 text-muted-foreground">
                            <MapPin size={10} class="shrink-0" /> <span class="truncate">{course.roomName}</span>
                          </p>
                        {/if}
                        {#if span >= 3 && course.teacherNames.length > 0}
                          <p class="mb-0 mt-1 flex min-w-0 items-center gap-1 truncate text-[10px] leading-3.5 text-muted-foreground">
                            <UserRound size={10} class="shrink-0" /> <span class="truncate">{course.teacherNames.join(" / ")}</span>
                          </p>
                        {/if}
                      </article>
                    {/each}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {:else if todayList.length === 0}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 border-t border-border py-10 text-muted-foreground">
      <CalendarOff size={22} class="text-muted-foreground/70" />
      <p class="m-0 text-sm">今天没有课程</p>
    </div>
  {:else}
    <div class="grid flex-1 auto-rows-min gap-1.5 overflow-y-auto border-t border-border pt-3.5">
      {#each todayList as course}
        {@const palette = coursePalette(course.courseName)}
        <article
          class="grid grid-cols-[64px_1fr] items-center gap-3 rounded-r-lg py-2.5 pl-3.5 pr-3 sm:grid-cols-[76px_1fr_auto]"
          style={`background: ${palette.background}; border-left: 3px solid ${palette.bar};`}
        >
          <time class="text-xs font-semibold tabular-nums" style={`color: ${palette.text};`}>
            {course.startUnit}{course.startUnit !== course.endUnit ? `-${course.endUnit}` : ""} 节
          </time>
          <div>
            <h3 class="m-0 text-sm font-medium text-foreground">{course.courseName}</h3>
            <p class="mb-0 mt-0.5 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
              {#if course.roomName}
                <span class="flex items-center gap-1"><MapPin size={11} /> {course.roomName}</span>
              {/if}
              {#if course.teacherNames.length > 0}
                <span>{course.teacherNames.join(" / ")}</span>
              {/if}
            </p>
          </div>
          <span class="hidden self-center text-[11px] text-muted-foreground sm:block">
            周 {course.weeks.length > 0 ? `${Math.min(...course.weeks)}-${Math.max(...course.weeks)}` : "全部"}
          </span>
        </article>
      {/each}
    </div>
  {/if}
</section>
