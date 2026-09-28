<script lang="ts">
  import { untrack } from "svelte";
  import Building2 from "@lucide/svelte/icons/building-2";
  import BusFront from "@lucide/svelte/icons/bus-front";
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import LibraryBig from "@lucide/svelte/icons/library-big";
  import LogIn from "@lucide/svelte/icons/log-in";
  import Utensils from "@lucide/svelte/icons/utensils";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import { Calendar } from "$lib/components/ui/calendar";
  import { Popover } from "$lib/components/ui/popover";
  import {
    CAMPUS_BUILDINGS,
    CAMPUS_OPTIONS,
    campusServices,
  } from "$lib/stores/campus-services.svelte";
  import { session } from "$lib/stores/session.svelte";
  import type { CampusBus } from "$lib/types/app";
  import { cn } from "$lib/utils";

  interface Props {
    onLogin: () => void;
  }

  let { onLogin }: Props = $props();
  let dateOpen = $state(false);

  $effect(() => {
    const sessionReady = session.ready;
    const campusLoggedIn = session.status.campusLoggedIn;
    untrack(() => {
      void campusServices.loadLibrary();
      if (!sessionReady) return;
      if (campusLoggedIn) {
        void campusServices.loadDining();
        void campusServices.loadBuses();
        void campusServices.loadClassrooms();
      } else {
        campusServices.requireLogin();
      }
    });
  });

  const campusLabel = $derived(
    CAMPUS_OPTIONS.find((item) => item.value === campusServices.campus)?.label ?? "邯郸",
  );
  const filteredBuses = $derived.by(() => {
    if (campusServices.buses.phase !== "ready") return [];
    return campusServices.buses.data
      .filter((bus) => bus.startCampus.includes(campusLabel) || bus.endCampus.includes(campusLabel))
      .slice(0, 10);
  });

  function occupancyPercent(current: number, capacity: number): number {
    if (capacity <= 0) return 0;
    return Math.min(100, Math.max(0, Math.round((current / capacity) * 100)));
  }

  function occupancyLabel(current: number, capacity: number): string {
    const ratio = occupancyPercent(current, capacity);
    if (ratio >= 85) return "拥挤";
    if (ratio >= 55) return "较忙";
    return "宽松";
  }

  function routeLabel(bus: CampusBus): string {
    if (bus.direction === 2) return `${bus.endCampus} → ${bus.startCampus}`;
    if (bus.direction === 1) return `${bus.startCampus} ↔ ${bus.endCampus}`;
    return `${bus.startCampus} → ${bus.endCampus}`;
  }

  function timeLabel(bus: CampusBus): string {
    if (bus.direction === 2) return bus.endTime ?? bus.startTime ?? "--:--";
    if (bus.direction === 1 && bus.startTime && bus.endTime && bus.startTime !== bus.endTime) {
      return `${bus.startTime} / ${bus.endTime}`;
    }
    return bus.startTime ?? bus.endTime ?? "--:--";
  }

  function displayDate(value: string): string {
    const [, month, day] = value.split("-");
    return `${Number(month)} 月 ${Number(day)} 日`;
  }
</script>

<section class="pb-10">
  <div class="grid border-b border-border lg:grid-cols-2">
    <section class="py-5 lg:border-r lg:border-border lg:pr-7">
      <div class="mb-4 flex items-center gap-2.5">
        <LibraryBig size={18} class="text-primary" />
        <h2 class="m-0 text-[15px] font-semibold">图书馆人数</h2>
        {#if campusServices.library.phase === "ready"}
          <Badge variant="muted" class="ml-auto">实时</Badge>
        {/if}
      </div>
      {#if campusServices.library.phase === "loading" || campusServices.library.phase === "idle"}
        <div class="grid grid-cols-2 gap-x-6 gap-y-5 lg:grid-cols-4">
          {#each Array(4) as _}
            <div><div class="h-3 w-16 animate-pulse rounded bg-muted"></div><div class="mt-2 h-7 w-20 animate-pulse rounded bg-muted"></div></div>
          {/each}
        </div>
      {:else if campusServices.library.phase === "error"}
        {@render ResourceError(campusServices.library.message, () => campusServices.loadLibrary(true))}
      {:else if campusServices.library.phase === "ready" && campusServices.library.data.length === 0}
        <p class="m-0 text-sm text-muted-foreground">暂无图书馆人数数据</p>
      {:else if campusServices.library.phase === "ready"}
        <div class="grid grid-cols-2 gap-x-6 gap-y-5 lg:grid-cols-4">
          {#each campusServices.library.data as library}
            <div>
              <p class="m-0 truncate text-xs text-muted-foreground">{library.campusName}</p>
              <p class="mb-0 mt-1 text-2xl font-semibold tracking-[-0.035em] tabular-nums">
                {library.people.toLocaleString("zh-CN")}
                <span class="ml-1 text-xs font-normal text-muted-foreground">人</span>
              </p>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="border-t border-border py-5 lg:border-t-0 lg:pl-7">
      <div class="mb-4 flex items-center gap-2.5">
        <Utensils size={18} class="text-primary" />
        <h2 class="m-0 text-[15px] font-semibold">食堂拥挤度</h2>
        <Badge variant="outline" class="ml-auto">{campusLabel}</Badge>
      </div>
      {#if campusServices.dining.phase === "unauthenticated"}
        {@render LoginState(onLogin, true)}
      {:else if campusServices.dining.phase === "loading" || campusServices.dining.phase === "idle"}
        <div class="space-y-4">{#each Array(3) as _}<div class="h-9 animate-pulse rounded bg-muted"></div>{/each}</div>
      {:else if campusServices.dining.phase === "enhancedAuth"}
        {@render ResourceError(campusServices.dining.message, () => campusServices.authenticateDining(), "完成验证")}
      {:else if campusServices.dining.phase === "error"}
        {@render ResourceError(campusServices.dining.message, () => campusServices.loadDining(true))}
      {:else if campusServices.dining.phase === "ready" && !campusServices.dining.data.available}
        <p class="m-0 text-sm text-muted-foreground">当前不在食堂数据开放时段</p>
      {:else if campusServices.dining.phase === "ready" && campusServices.dining.data.venues.length === 0}
        <p class="m-0 text-sm text-muted-foreground">该校区暂无食堂数据</p>
      {:else if campusServices.dining.phase === "ready"}
        <div class="space-y-4">
          {#each campusServices.dining.data.venues as venue}
            <div>
              <div class="mb-1.5 flex items-center gap-3 text-xs">
                <span class="min-w-0 flex-1 truncate text-foreground">{venue.name}</span>
                <span class="text-muted-foreground tabular-nums">{venue.current} / {venue.capacity}</span>
                <span class="w-8 text-right text-muted-foreground">{occupancyLabel(venue.current, venue.capacity)}</span>
              </div>
              <div class="h-1.5 overflow-hidden rounded-full bg-muted">
                <div class="h-full rounded-full bg-primary/70" style={`width:${occupancyPercent(venue.current, venue.capacity)}%`}></div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>

  <section class="border-b border-border py-6">
    <div class="mb-4 flex items-center gap-2.5">
      <BusFront size={18} class="text-primary" />
      <h2 class="m-0 text-[15px] font-semibold">校车时刻</h2>
      <Badge variant="outline">{campusLabel}</Badge>
    </div>
    {#if campusServices.buses.phase === "unauthenticated"}
      {@render LoginState(onLogin)}
    {:else if campusServices.buses.phase === "loading" || campusServices.buses.phase === "idle"}
      <div class="grid gap-3 md:grid-cols-2">{#each Array(4) as _}<div class="h-14 animate-pulse rounded-lg bg-muted"></div>{/each}</div>
    {:else if campusServices.buses.phase === "error"}
      {@render ResourceError(campusServices.buses.message, () => campusServices.loadBuses(true))}
    {:else if filteredBuses.length === 0}
      <p class="m-0 text-sm text-muted-foreground">今天没有匹配该校区的班次</p>
    {:else}
      <div class="grid overflow-hidden rounded-xl border border-border bg-card md:grid-cols-2">
        {#each filteredBuses as bus, index}
          <div class={cn("flex items-center gap-4 px-4 py-3.5", index > 0 && "border-t border-border", index === 1 && "md:border-t-0", index % 2 === 1 && "md:border-l md:border-border")}>
            <span class="w-24 shrink-0 text-lg font-semibold tracking-[-0.025em] tabular-nums">{timeLabel(bus)}</span>
            <span class="min-w-0 flex-1 truncate text-sm text-foreground/85">{routeLabel(bus)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section class="pt-6">
    <div class="flex flex-wrap items-center gap-2.5">
      <Building2 size={18} class="text-primary" />
      <h2 class="m-0 text-[15px] font-semibold">空教室</h2>
      <div class="ml-auto flex items-center gap-2">
        <Popover
          bind:open={dateOpen}
          align="end"
          triggerClass="flex h-8 items-center gap-1.5 rounded-lg border border-border bg-card px-3 text-xs text-muted-foreground transition hover:bg-muted hover:text-foreground"
        >
          {#snippet trigger()}<CalendarDays size={13} /> {displayDate(campusServices.date)}{/snippet}
          {#snippet content()}
            <Calendar value={campusServices.date} onValueChange={(value) => { campusServices.setDate(value); dateOpen = false; }} />
          {/snippet}
        </Popover>
      </div>
    </div>
    <div class="mt-4 flex flex-wrap gap-1.5" aria-label="选择教学楼">
      {#each CAMPUS_BUILDINGS[campusServices.campus] as building}
        <Button
          size="sm"
          variant={campusServices.building === building ? "secondary" : "ghost"}
          class="h-7 px-2.5"
          aria-pressed={campusServices.building === building}
          onclick={() => campusServices.setBuilding(building)}
        >{building}</Button>
      {/each}
    </div>

    {#if campusServices.classrooms.phase === "unauthenticated"}
      <div class="mt-4">{@render LoginState(onLogin)}</div>
    {:else if campusServices.classrooms.phase === "loading" || campusServices.classrooms.phase === "idle"}
      <div class="mt-4 space-y-2">{#each Array(6) as _}<div class="h-10 animate-pulse rounded bg-muted"></div>{/each}</div>
    {:else if campusServices.classrooms.phase === "error"}
      <div class="mt-4">{@render ResourceError(campusServices.classrooms.message, () => campusServices.loadClassrooms(true))}</div>
    {:else if campusServices.classrooms.phase === "ready" && campusServices.classrooms.data.length === 0}
      <p class="mb-0 mt-5 text-sm text-muted-foreground">没有读取到该教学楼的教室数据</p>
    {:else if campusServices.classrooms.phase === "ready"}
      <div class="mt-4 overflow-x-auto rounded-xl border border-border bg-card">
        <div class="min-w-[700px]">
          <div class="grid grid-cols-[7rem_5rem_1fr] items-center border-b border-border px-4 py-2.5 text-[11px] text-muted-foreground">
            <span>教室</span><span>座位</span>
            <div class="grid grid-cols-13 gap-1 text-center tabular-nums">{#each Array(13) as _, index}<span>{index + 1}</span>{/each}</div>
          </div>
          {#each campusServices.classrooms.data as room}
            {@const available = room.busy.filter((busy) => !busy).length}
            <div class="grid grid-cols-[7rem_5rem_1fr] items-center border-b border-border px-4 py-2.5 last:border-b-0">
              <span class="text-sm font-medium">{room.roomName}</span>
              <span class="text-xs text-muted-foreground tabular-nums">{room.seats ?? "—"}</span>
              <div class="grid grid-cols-13 gap-1" title={`空闲 ${available} 节`}>
                {#each room.busy as busy}<span class={cn("h-4 rounded-[3px]", busy ? "bg-foreground/18" : "bg-emerald-500/65")}></span>{/each}
              </div>
            </div>
          {/each}
        </div>
      </div>
      <div class="mt-2 flex items-center justify-end gap-4 text-[11px] text-muted-foreground">
        <span class="inline-flex items-center gap-1.5"><i class="size-2 rounded-sm bg-emerald-500/65"></i>空闲</span>
        <span class="inline-flex items-center gap-1.5"><i class="size-2 rounded-sm bg-foreground/18"></i>有课</span>
      </div>
    {/if}
  </section>
</section>

{#snippet LoginState(onLogin: () => void, compact = false)}
  <div class={cn("flex items-center justify-between gap-4 rounded-xl border border-dashed border-border px-4", compact ? "py-4" : "py-5")}>
    <div>
      <p class="m-0 text-sm font-medium">登录后读取校园服务</p>
      <p class="mb-0 mt-1 text-xs text-muted-foreground">使用复旦统一认证，无需旦挞账号</p>
    </div>
    <Button size="sm" onclick={onLogin}><LogIn size={14} /> 登录 UIS</Button>
  </div>
{/snippet}

{#snippet ResourceError(message: string, onRetry: () => void, action = "重试")}
  <div class="flex items-center justify-between gap-4 text-sm text-muted-foreground">
    <span>{message}</span>
    <Button variant="outline" size="sm" onclick={onRetry}>{action}</Button>
  </div>
{/snippet}
