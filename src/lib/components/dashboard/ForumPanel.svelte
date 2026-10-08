<script lang="ts">
  import {
    ArrowLeft,
    ArrowRight,
    ChevronRight,
    Eye,
    LoaderCircle,
    LogIn,
    MessageCircle,
    PenLine,
    RefreshCw,
    Star,
  } from "@lucide/svelte";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import ForumComposer from "$lib/components/forum/ForumComposer.svelte";
  import ForumDivisionSelect from "$lib/components/forum/ForumDivisionSelect.svelte";
  import ForumFloorActions from "$lib/components/forum/ForumFloorActions.svelte";
  import ForumContent from "$lib/components/forum/ForumContent.svelte";
  import ForumConversation from "$lib/components/forum/ForumConversation.svelte";
  import { ForumConversationController } from "$lib/features/forum/conversation.svelte";
  import { forumContentExcerpt, type ForumReferenceTarget } from "$lib/features/forum/content";
  import { forum, session } from "$lib/stores/session.svelte";
  import type { ForumFloor, ForumFloorPreview, ForumHole, ForumTag } from "$lib/types/app";

  interface Props {
    expanded?: boolean;
    onLogin: () => void;
    onOpenForum?: (holeId?: number) => void;
  }

  let { expanded = false, onLogin, onOpenForum }: Props = $props();
  let order = $state<"time_updated" | "time_created">("time_updated");
  let listViewport = $state<HTMLDivElement>();
  let threadViewport = $state<HTMLDivElement>();
  let listViewportHeight = $state(640);
  let selectedDivisionId = $state<number | null>(null);
  let composer = $state<
    | { mode: "post" }
    | { mode: "reply"; holeId: number; floorId?: number; targetLabel: string }
    | { mode: "report"; floorId: number; targetLabel: string }
    | null
  >(null);
  let actionNotice = $state<string | null>(null);
  const conversation = new ForumConversationController();

  const listLoadThreshold = 360;
  const tagTones = [
    "border-amber-500/20 bg-amber-500/12 text-amber-800 dark:text-amber-300",
    "border-teal-500/20 bg-teal-500/12 text-teal-800 dark:text-teal-300",
    "border-sky-500/20 bg-sky-500/12 text-sky-800 dark:text-sky-300",
    "border-rose-500/20 bg-rose-500/12 text-rose-800 dark:text-rose-300",
    "border-violet-500/20 bg-violet-500/12 text-violet-800 dark:text-violet-300",
  ] as const;

  const holes = $derived(forum.state.phase === "ready" ? forum.state.holes : []);
  const compactItems = $derived(holes.slice(0, 9));
  const excerpts = $derived(new Map(holes.map((hole) => [
    hole.holeId,
    forumContentExcerpt(hole.firstFloor?.content ?? "内容暂不可见"),
  ])));
  const showListFooter = $derived(
    forum.state.phase === "ready" &&
      (forum.state.loadingMore || forum.state.moreError !== null),
  );
  const selectedHoleId = $derived(
    forum.detail.phase === "idle" ? null : forum.detail.phase === "ready"
      ? forum.detail.thread.hole.holeId
      : forum.detail.holeId,
  );
  const thread = $derived(forum.detail.phase === "ready" ? forum.detail.thread : null);
  const firstFloor = $derived.by((): ForumFloor | ForumFloorPreview | null => {
    if (!thread) return null;
    const preview = thread.hole.firstFloor;
    return (
      thread.floors.find((floor) => floor.floorId === preview?.floorId) ??
      preview ??
      thread.floors[0]
    );
  });
  const replies = $derived.by(() => {
    if (!thread) return [];
    return firstFloor
      ? thread.floors.filter((floor) => floor.floorId !== firstFloor.floorId)
      : thread.floors;
  });
  const referenceFloors = $derived.by((): ForumReferenceTarget[] => {
    if (!thread) return [];
    const floors = firstFloor ? [firstFloor, ...replies] : replies;
    return floors.map((floor, index) => ({
      floorId: floor.floorId,
      holeId: thread.hole.holeId,
      content: floor.content,
      anonyname: floor.anonyname,
      deleted: "deleted" in floor && floor.deleted === true,
      mentions: floor.mentions,
      complete: "liked" in floor,
      floorNumber: firstFloor && index === 0 ? 1 : replyFloorNumber(firstFloor ? index - 1 : index),
    }));
  });

  $effect(() => {
    selectedHoleId;
    session.status.communityLoggedIn;
    return () => conversation.close();
  });

  $effect(() => {
    if (!session.ready) return;
    if (session.status.communityLoggedIn) {
      if (forum.state.phase === "idle" || forum.state.phase === "unauthenticated") {
        void forum.load(10, order, selectedDivisionId);
      }
      void forum.loadMeta();
      void forum.loadFavorites();
    } else {
      forum.requireLogin();
    }
  });

  $effect(() => {
    const element = threadViewport;
    const currentThread = thread;
    if (
      !element ||
      !currentThread ||
      forum.detail.phase !== "ready" ||
      forum.detail.loadingMore ||
      forum.detail.moreError ||
      currentThread.nextOffset == null
    ) {
      return;
    }
    const floorCount = currentThread.floors.length;
    queueMicrotask(() => {
      if (
        floorCount === currentThread.floors.length &&
        element.scrollHeight - element.scrollTop - element.clientHeight < 420
      ) {
        void forum.loadMore();
      }
    });
  });

  $effect(() => {
    const element = threadViewport;
    const currentThread = thread;
    if (!element || !currentThread || actionNotice !== "回复已发布") return;
    queueMicrotask(() => {
      element.scrollTop = element.scrollHeight;
    });
  });

  $effect(() => {
    const element = listViewport;
    if (!element) return;
    const updateHeight = () => {
      listViewportHeight = element.clientHeight;
    };
    updateHeight();
    const observer = new ResizeObserver(updateHeight);
    observer.observe(element);
    return () => observer.disconnect();
  });

  $effect(() => {
    if (
      expanded &&
      forum.state.phase === "ready" &&
      forum.state.hasMore &&
      !forum.state.loadingMore &&
      !forum.state.moreError &&
      listViewport &&
      listViewport.scrollHeight < listViewportHeight + listLoadThreshold
    ) {
      void forum.loadMoreHoles();
    }
  });

  function selectHole(holeId: number) {
    if (expanded) {
      void forum.open(holeId);
    } else {
      onOpenForum?.(holeId);
    }
  }

  function selectOrder(next: "time_updated" | "time_created") {
    if (order === next) return;
    order = next;
    if (listViewport) listViewport.scrollTop = 0;
    void forum.load(10, next, selectedDivisionId);
  }

  function selectDivision(value: number | null) {
    selectedDivisionId = value;
    if (listViewport) listViewport.scrollTop = 0;
    void forum.load(10, order, selectedDivisionId);
  }

  function reloadList() {
    if (listViewport) listViewport.scrollTop = 0;
    void forum.load(10, order, selectedDivisionId);
  }

  function handleListScroll(event: Event) {
    const element = event.currentTarget as HTMLDivElement;
    listViewportHeight = element.clientHeight;
    if (element.scrollHeight - element.scrollTop - element.clientHeight < listLoadThreshold) {
      void forum.loadMoreHoles();
    }
  }

  function handleThreadScroll(event: Event) {
    const element = event.currentTarget as HTMLDivElement;
    if (element.scrollHeight - element.scrollTop - element.clientHeight < 420) {
      void forum.loadMore();
    }
  }

  function relativeTime(iso: string): string {
    const timestamp = Date.parse(iso);
    if (Number.isNaN(timestamp)) return "";
    const seconds = Math.max(0, (Date.now() - timestamp) / 1000);
    if (seconds < 60) return "刚刚";
    if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟前`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)} 小时前`;
    if (seconds < 86400 * 7) return `${Math.floor(seconds / 86400)} 天前`;
    return new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" }).format(
      timestamp,
    );
  }

  function fullTime(iso: string): string {
    const timestamp = Date.parse(iso);
    if (Number.isNaN(timestamp)) return "";
    return new Intl.DateTimeFormat("zh-CN", {
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(timestamp);
  }

  function excerpt(hole: ForumHole): string {
    return excerpts.get(hole.holeId) ?? "内容暂不可见";
  }

  function authorName(name: string): string {
    return name.trim() || "匿名同学";
  }

  function authorMark(name: string): string {
    return authorName(name).slice(0, 1);
  }

  function toneIndex(value: string, length: number): number {
    let hash = 0;
    for (const character of value) hash = (hash * 31 + character.charCodeAt(0)) | 0;
    return Math.abs(hash) % length;
  }

  function tagTone(name: string): string {
    return tagTones[toneIndex(name, tagTones.length)];
  }

  function compactNumber(value: number): string {
    if (value < 1000) return String(value);
    return `${(value / 1000).toFixed(value >= 10000 ? 0 : 1)}k`;
  }

  function retryDetail() {
    if (forum.detail.phase === "error") void forum.open(forum.detail.holeId);
  }

  async function openPostComposer() {
    actionNotice = null;
    forum.actionError = null;
    await forum.loadMeta();
    if (forum.meta.phase === "ready" && forum.meta.divisions.length > 0) {
      composer = { mode: "post" };
    } else if (forum.meta.phase === "error") {
      forum.actionError = forum.meta.message;
    } else {
      forum.actionError = "当前没有可用的发帖分区";
    }
  }

  function openReplyComposer(holeId: number, floor?: ForumFloor | ForumFloorPreview, floorNumber?: number) {
    actionNotice = null;
    forum.actionError = null;
    composer = {
      mode: "reply",
      holeId,
      floorId: floorNumber === 1 ? undefined : floor?.floorId,
      targetLabel: floorNumber == null || floorNumber === 1 ? `回复 #${holeId}` : `回复 ${floorNumber}F`,
    };
  }

  function openReportComposer(floor: ForumFloor | ForumFloorPreview, floorNumber: number) {
    actionNotice = null;
    forum.actionError = null;
    composer = {
      mode: "report",
      floorId: floor.floorId,
      targetLabel: `举报 #${thread?.hole.holeId ?? ""} 的 ${floorNumber}F`,
    };
  }

  function reactToFloor(floor: ForumFloor | ForumFloorPreview, kind: "like" | "dislike") {
    if ("liked" in floor) void forum.reactFloor(floor, kind);
  }

  function replyFloorNumber(index: number): number {
    if (!thread) return index + 2;
    return thread.offset === 0 ? index + 2 : thread.offset + index + 1;
  }

  async function submitComposer(payload: {
    content: string;
    divisionId: number | null;
    tags: ForumTag[];
  }) {
    if (!composer) return { ok: false as const, message: "操作已取消" };
    if (composer.mode === "post") {
      if (payload.divisionId == null) return { ok: false as const, message: "请选择发帖分区" };
      const result = await forum.createHole(payload.divisionId, payload.content, payload.tags);
      if (result.ok) {
        forum.actionError = null;
        selectedDivisionId = payload.divisionId;
        await forum.load(10, order, selectedDivisionId);
        actionNotice = "讨论已发布";
      }
      return result;
    }
    if (composer.mode === "reply") {
      const result = await forum.createFloor(
        composer.holeId,
        payload.content,
        composer.floorId,
      );
      if (result.ok) {
        forum.actionError = null;
        actionNotice = "回复已发布";
      }
      return result;
    }
    const result = await forum.reportFloor(composer.floorId, payload.content);
    if (result.ok) {
      forum.actionError = null;
      actionNotice = "举报已提交";
    }
    return result;
  }
</script>

{#if expanded}
  <section class="h-full min-h-0">
    {#if forum.state.phase === "unauthenticated"}
      <div class="grid h-full min-h-[420px] place-items-center bg-card px-6">
        <div class="flex max-w-sm flex-col items-center text-center">
          <span class="grid size-12 place-items-center rounded-xl bg-primary/9 text-primary">
            <MessageCircle size={22} strokeWidth={1.8} />
          </span>
          <h2 class="mb-0 mt-5 text-lg font-semibold tracking-[-0.02em]">登录后查看茶楼</h2>
          <p class="mb-0 mt-2 text-sm leading-6 text-muted-foreground">
            使用旦挞账号浏览讨论与回复
          </p>
          <Button class="mt-5" onclick={onLogin}>
            <LogIn size={15} /> 登录旦挞账号
          </Button>
        </div>
      </div>
    {:else}
      <div
        class="grid h-full min-h-[520px] overflow-hidden bg-card lg:grid-cols-[minmax(300px,0.38fr)_minmax(0,0.62fr)]"
      >
        <aside
          class={`min-h-0 flex-col bg-card ${forum.detail.phase === "idle" ? "flex" : "hidden lg:flex"}`}
        >
          <div class="shrink-0 border-b border-border px-3 py-2">
            <div class="flex items-center gap-1">
              <Button
                variant={order === "time_updated" ? "secondary" : "ghost"}
                size="sm"
                onclick={() => selectOrder("time_updated")}>最近回复</Button
              >
              <Button
                variant={order === "time_created" ? "secondary" : "ghost"}
                size="sm"
                onclick={() => selectOrder("time_created")}>最新发布</Button
              >
              <Button
                variant="ghost"
                size="icon"
                class="ml-auto size-8"
                aria-label="刷新讨论"
                onclick={reloadList}
              >
                <RefreshCw size={14} />
              </Button>
              <Button size="sm" onclick={openPostComposer}>
                <PenLine size={14} /> 发帖
              </Button>
            </div>
            <div class="mt-2">
              <ForumDivisionSelect
                value={selectedDivisionId}
                divisions={forum.meta.phase === "ready" ? forum.meta.divisions : []}
                homepageLabel="首页推荐"
                compact
                onValueChange={selectDivision}
              />
            </div>
          </div>

          {#if forum.state.phase === "loading" || forum.state.phase === "idle"}
            <div class="min-h-0 flex-1 overflow-hidden">
              {#each Array(7) as _}
                <div class="border-b border-border px-4 py-4">
                  <div class="flex items-center gap-2">
                    <div class="h-4 w-14 animate-pulse rounded bg-muted"></div>
                    <div class="h-3 w-16 animate-pulse rounded bg-muted"></div>
                  </div>
                  <div class="mt-3 h-3.5 w-full animate-pulse rounded bg-muted"></div>
                  <div class="mt-2 h-3.5 w-3/4 animate-pulse rounded bg-muted"></div>
                </div>
              {/each}
            </div>
          {:else if forum.state.phase === "error"}
            <div class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
              <p class="m-0 text-sm leading-6 text-muted-foreground">{forum.state.message}</p>
              <Button
                variant="outline"
                size="sm"
                onclick={() => forum.load(10, order, selectedDivisionId)}
              >
                <RefreshCw size={14} /> 重试
              </Button>
            </div>
          {:else}
            <div
              bind:this={listViewport}
              class="min-h-0 flex-1 overflow-y-auto overscroll-contain"
              onscroll={handleListScroll}
            >
              <div>
                {#each holes as hole (hole.holeId)}
                  {@const isSelected = selectedHoleId === hole.holeId}
                  <button
                    type="button"
                    aria-current={isSelected ? "true" : undefined}
                    class={`forum-list-item focus-ring group relative flex w-full flex-col overflow-hidden border-b border-border px-4 py-3.5 text-left transition-colors ${
                      isSelected
                        ? "bg-primary/[0.11] hover:bg-primary/[0.11]"
                        : "bg-card hover:bg-muted/55"
                    }`}
                    onclick={() => selectHole(hole.holeId)}
                  >
                    <span
                      aria-hidden="true"
                      class={`absolute inset-y-0 left-0 w-[3px] bg-primary transition-opacity ${isSelected ? "opacity-100" : "opacity-0"}`}
                    ></span>
                    <div class="flex shrink-0 items-center gap-2">
                      <span
                        class={`text-[11px] font-medium tabular-nums ${isSelected ? "text-primary" : "text-muted-foreground"}`}
                        >#{hole.holeId}</span
                      >
                      {#if hole.tags[0]}
                        <Badge
                          variant="outline"
                          class={`max-w-30 truncate px-2 py-0.5 font-medium ${tagTone(hole.tags[0].name)}`}
                        >
                          {hole.tags[0].name}
                        </Badge>
                      {/if}
                      <span class="ml-auto shrink-0 text-[11px] text-muted-foreground">
                        {relativeTime(hole.timeUpdated)}
                      </span>
                    </div>
                    <p
                      class="mb-0 mt-2.5 line-clamp-3 min-h-0 text-[14px] leading-[1.55] text-foreground/92"
                    >
                      {excerpt(hole)}
                    </p>
                    <div
                      class="mt-2.5 flex shrink-0 items-center gap-3 text-[11px] text-muted-foreground"
                    >
                      <span class="flex items-center gap-1"
                        ><MessageCircle size={12} />{hole.reply}</span
                      >
                      <span class="flex items-center gap-1"
                        ><Eye size={12} />{compactNumber(hole.view)}</span
                      >
                      <ChevronRight
                        size={14}
                        class={`ml-auto transition group-hover:translate-x-0.5 group-hover:opacity-80 ${isSelected ? "text-primary opacity-100" : "opacity-35"}`}
                      />
                    </div>
                  </button>
                {/each}
                {#if showListFooter && forum.state.phase === "ready"}
                  <div class="flex h-12 items-center justify-center border-t border-border text-xs text-muted-foreground">
                    {#if forum.state.loadingMore}
                      <span class="flex items-center gap-2">
                        <LoaderCircle class="animate-spin" size={13} /> 加载更多讨论
                      </span>
                    {:else if forum.state.moreError}
                      <Button variant="ghost" size="sm" onclick={() => forum.retryMoreHoles()}>
                        加载失败，点击重试
                      </Button>
                    {/if}
                  </div>
                {/if}
              </div>
              {#if holes.length === 0}
                <p class="m-0 px-4 py-12 text-center text-sm text-muted-foreground">暂无讨论</p>
              {/if}
            </div>
          {/if}
        </aside>

        <div
          class={`min-h-0 flex-col border-border bg-background/35 lg:flex lg:border-l ${
            forum.detail.phase === "idle" ? "hidden" : "flex"
          }`}
        >
          {#if forum.detail.phase === "idle"}
            <div class="hidden flex-1 place-items-center lg:grid">
              <div class="max-w-xs text-center text-muted-foreground">
                <MessageCircle class="mx-auto opacity-35" size={30} strokeWidth={1.5} />
                <p class="mb-0 mt-3 text-sm">选择一条讨论查看完整内容</p>
              </div>
            </div>
          {:else if forum.detail.phase === "loading"}
            <div class="flex h-11 shrink-0 items-center border-b border-border px-3 lg:px-5">
              <Button
                variant="ghost"
                size="sm"
                class="-ml-1 lg:hidden"
                onclick={() => forum.closeDetail()}
              >
                <ArrowLeft size={15} /> 返回
              </Button>
              <span class="ml-auto flex items-center gap-2 text-xs text-muted-foreground lg:ml-0">
                <LoaderCircle class="animate-spin" size={14} /> 正在打开讨论
              </span>
            </div>
            <div class="min-h-0 flex-1 overflow-hidden px-5 py-6 lg:px-8">
              <div class="h-4 w-32 animate-pulse rounded bg-muted"></div>
              <div class="mt-5 h-5 w-11/12 animate-pulse rounded bg-muted"></div>
              <div class="mt-3 h-5 w-4/5 animate-pulse rounded bg-muted"></div>
              <div class="mt-3 h-5 w-2/3 animate-pulse rounded bg-muted"></div>
              <div class="mt-9 border-t border-border pt-6">
                <div class="h-4 w-24 animate-pulse rounded bg-muted"></div>
                <div class="mt-4 h-4 w-full animate-pulse rounded bg-muted"></div>
                <div class="mt-3 h-4 w-3/4 animate-pulse rounded bg-muted"></div>
              </div>
            </div>
          {:else if forum.detail.phase === "error"}
            <div class="flex h-11 shrink-0 items-center border-b border-border px-3 lg:px-5">
              <Button variant="ghost" size="sm" class="-ml-1" onclick={() => forum.closeDetail()}>
                <ArrowLeft size={15} /> 返回
              </Button>
            </div>
            <div class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
              <p class="m-0 text-sm leading-6 text-muted-foreground">{forum.detail.message}</p>
              <Button
                variant="outline"
                size="sm"
                onclick={retryDetail}
              >
                <RefreshCw size={14} /> 重新加载
              </Button>
            </div>
          {:else if thread}
            <div class="flex h-11 shrink-0 items-center gap-3 border-b border-border px-3 lg:px-5">
              <Button
                variant="ghost"
                size="sm"
                class="-ml-1 lg:hidden"
                onclick={() => forum.closeDetail()}
              >
                <ArrowLeft size={15} /> 返回
              </Button>
              <span class="text-xs font-medium tabular-nums text-muted-foreground">
                #{thread.hole.holeId}
              </span>
              <div class="flex min-w-0 items-center gap-1.5 overflow-hidden">
                {#each thread.hole.tags.slice(0, 3) as tag (tag.name)}
                  <Badge
                    variant="outline"
                    class={`shrink-0 px-2 py-0.5 font-medium ${tagTone(tag.name)}`}>{tag.name}</Badge
                  >
                {/each}
              </div>
              <div class="ml-auto flex shrink-0 items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  class={forum.isFavorite(thread.hole.holeId) ? "text-primary" : ""}
                  aria-label={forum.isFavorite(thread.hole.holeId) ? "取消收藏" : "收藏讨论"}
                  aria-pressed={forum.isFavorite(thread.hole.holeId)}
                  disabled={forum.favoriteBusy || forum.favoritesLoading}
                  onclick={() => forum.toggleFavorite(thread.hole.holeId)}
                >
                  <Star size={14} fill={forum.isFavorite(thread.hole.holeId) ? "currentColor" : "none"} />
                  {thread.hole.favoriteCount}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={thread.hole.locked}
                  title={thread.hole.locked ? "讨论已锁定" : "回复讨论"}
                  onclick={() => openReplyComposer(thread.hole.holeId, firstFloor ?? undefined, 1)}
                >
                  <MessageCircle size={13} /> 回复
                </Button>
              </div>
              <div class="hidden shrink-0 items-center gap-3 text-[11px] text-muted-foreground xl:flex">
                <span class="flex items-center gap-1"><MessageCircle size={12} />{thread.hole.reply}</span>
                <span class="flex items-center gap-1"><Eye size={12} />{compactNumber(thread.hole.view)}</span>
              </div>
            </div>

            {#if forum.actionError || actionNotice}
              <div
                class={`shrink-0 border-b border-border px-5 py-2 text-xs ${forum.actionError ? "text-destructive" : "text-primary"}`}
                role="status"
              >
                {forum.actionError ?? actionNotice}
              </div>
            {/if}

            <div
              bind:this={threadViewport}
              class="selectable min-h-0 flex-1 overflow-y-auto overscroll-contain"
              onscroll={handleThreadScroll}
            >
              <article class="px-5 py-4 lg:px-6 lg:py-5">
                {#if firstFloor}
                  <header class="flex items-center gap-2.5">
                    <span
                      class="grid size-8 shrink-0 place-items-center rounded-full bg-primary/9 text-xs font-semibold text-primary"
                    >
                      {authorMark(firstFloor.anonyname)}
                    </span>
                    <div class="min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="truncate text-sm font-semibold">{authorName(firstFloor.anonyname)}</span>
                        <Badge
                          variant="outline"
                          class="border-amber-500/20 bg-amber-500/12 px-2 py-0.5 font-medium text-amber-800 dark:text-amber-300"
                          >楼主</Badge
                        >
                      </div>
                      <p class="mb-0 mt-0.5 text-[11px] text-muted-foreground">
                        {fullTime(firstFloor.timeCreated)}
                      </p>
                    </div>
                  </header>
                  <ForumContent
                    content={firstFloor.content}
                    mentions={firstFloor.mentions}
                    loadedFloors={referenceFloors}
                    onOpenReference={(floorId) => conversation.open(firstFloor.floorId, floorId, referenceFloors)}
                    muted={"deleted" in firstFloor && firstFloor.deleted}
                    class="mt-3"
                  />
                  <ForumFloorActions
                    floor={firstFloor}
                    busy={"liked" in firstFloor && forum.reactingFloorIds.includes(firstFloor.floorId)}
                    replyDisabled={thread.hole.locked}
                    onReact={(kind) => reactToFloor(firstFloor, kind)}
                    onReply={() => openReplyComposer(thread.hole.holeId, firstFloor, 1)}
                    onReport={() => openReportComposer(firstFloor, 1)}
                  />
                {:else}
                  <p class="m-0 py-6 text-sm text-muted-foreground">主题内容暂不可见</p>
                {/if}
              </article>

              <div class="border-t border-border">
                <div class="flex h-10 items-center px-5 lg:px-6">
                  <span class="text-xs font-semibold text-foreground">回复</span>
                  <span class="ml-2 text-xs tabular-nums text-muted-foreground">{thread.hole.reply}</span>
                </div>

                {#each replies as floor, index (floor.floorId)}
                  <article class="virtual-floor border-t border-border px-5 py-3.5 lg:px-6 lg:py-4">
                    <header class="flex items-start gap-2.5">
                      <span
                        class="grid size-7 shrink-0 place-items-center rounded-full bg-muted text-[11px] font-semibold text-muted-foreground"
                      >
                        {authorMark(floor.anonyname)}
                      </span>
                      <div class="min-w-0 flex-1">
                        <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                          <span class="text-sm font-medium">{authorName(floor.anonyname)}</span>
                          {#if floor.specialTag}
                            <Badge
                              variant="outline"
                              class={`px-2 py-0.5 font-medium ${tagTone(floor.specialTag)}`}
                            >
                              {floor.specialTag}
                            </Badge>
                          {/if}
                          {#if floor.isMe}
                            <Badge variant="outline" class="px-2 py-0.5 font-medium">我</Badge>
                          {/if}
                          <span class="ml-auto text-[11px] tabular-nums text-muted-foreground">
                            {replyFloorNumber(index)}F
                          </span>
                        </div>
                        <p class="mb-0 mt-0.5 text-[11px] text-muted-foreground">
                          {fullTime(floor.timeCreated)}
                        </p>
                      </div>
                    </header>
                    {#if floor.fold.length > 0}
                      <p class="mb-0 mt-2.5 rounded-lg bg-muted px-3 py-2 text-xs text-muted-foreground">
                        {floor.fold.join(" · ")}
                      </p>
                    {/if}
                    <ForumContent
                      content={floor.content}
                      mentions={floor.mentions}
                      loadedFloors={referenceFloors}
                      onOpenReference={(floorId) => conversation.open(floor.floorId, floorId, referenceFloors)}
                      muted={floor.deleted}
                      class="mt-2.5"
                    />
                    <ForumFloorActions
                      {floor}
                      busy={forum.reactingFloorIds.includes(floor.floorId)}
                      replyDisabled={thread.hole.locked}
                      onReact={(kind) => void forum.reactFloor(floor, kind)}
                      onReply={() =>
                        openReplyComposer(thread.hole.holeId, floor, replyFloorNumber(index))}
                      onReport={() => openReportComposer(floor, replyFloorNumber(index))}
                    />
                  </article>
                {/each}

                {#if forum.detail.loadingMore || forum.detail.moreError}
                  <div class="border-t border-border px-5 py-5 text-center lg:px-8">
                    {#if forum.detail.moreError}
                      <p class="mb-3 mt-0 text-xs text-destructive">{forum.detail.moreError}</p>
                      <Button variant="outline" size="sm" onclick={() => forum.loadMore(true)}>
                        重新加载
                      </Button>
                    {:else}
                      <span class="inline-flex items-center gap-2 text-xs text-muted-foreground">
                        <LoaderCircle class="animate-spin" size={14} /> 正在加载更多回复
                      </span>
                    {/if}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </section>
{:else}
  <section class="flex h-full flex-col overflow-hidden rounded-xl border border-border bg-card">
    <div class="flex h-11 shrink-0 items-center justify-between border-b border-border px-4">
      <h2 class="m-0 text-[15px] font-semibold tracking-[-0.015em]">茶楼动态</h2>
      {#if forum.state.phase === "ready"}
        <Button variant="ghost" size="sm" class="h-7 px-2 text-muted-foreground" onclick={() => onOpenForum?.()}>
          全部 <ArrowRight size={14} />
        </Button>
      {/if}
    </div>

    {#if forum.state.phase === "loading" || forum.state.phase === "idle"}
      <div class="grid flex-1 px-4 md:grid-cols-3">
        {#each Array(9) as _, index}
          <div
            class={`flex flex-col justify-center gap-2 border-b border-border py-3 last:border-b-0 md:border-b-0 md:px-4 ${index >= 3 ? "md:border-t" : ""} ${index % 3 !== 0 ? "md:border-l" : "md:pl-0"} ${index % 3 === 2 ? "md:pr-0" : ""}`}
          >
            <div class="h-3 w-24 animate-pulse rounded bg-muted"></div>
            <div class="h-3.5 w-4/5 animate-pulse rounded bg-muted"></div>
          </div>
        {/each}
      </div>
    {:else if forum.state.phase === "unauthenticated"}
      <div class="m-3 flex flex-1 items-center gap-3 rounded-lg bg-muted/45 px-4 py-4">
        <span class="grid size-10 shrink-0 place-items-center rounded-lg bg-card text-primary">
          <MessageCircle size={18} strokeWidth={1.8} />
        </span>
        <div class="min-w-0 flex-1">
          <p class="m-0 text-sm font-medium text-foreground">登录后查看茶楼</p>
          <p class="mb-0 mt-1 text-xs text-muted-foreground">使用旦挞账号浏览讨论</p>
        </div>
        <Button size="sm" onclick={onLogin}><LogIn size={14} /> 登录</Button>
      </div>
    {:else if forum.state.phase === "error"}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 px-4 py-8 text-muted-foreground">
        <p class="m-0 text-sm">{forum.state.message}</p>
        <Button variant="outline" size="sm" onclick={() => forum.load(10, order)}>
          <RefreshCw size={14} /> 重试
        </Button>
      </div>
    {:else}
      <div class="grid flex-1 px-4 md:grid-cols-3">
        {#each compactItems as hole, index (hole.holeId)}
          <button
            type="button"
            class={`focus-ring group grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 border-b border-border py-3 text-left last:border-b-0 md:border-b-0 md:px-4 ${index >= 3 ? "md:border-t" : ""} ${index % 3 !== 0 ? "md:border-l" : "md:pl-0"} ${index % 3 === 2 ? "md:pr-0" : ""}`}
            onclick={() => selectHole(hole.holeId)}
          >
            <div class="min-w-0">
              <div class="flex items-center gap-2 text-[11px] text-muted-foreground">
                <span class="font-medium tabular-nums">#{hole.holeId}</span>
                {#if hole.tags[0]}
                  <Badge
                    variant="outline"
                    class={`max-w-28 truncate px-2 py-0.5 font-medium ${tagTone(hole.tags[0].name)}`}
                  >
                    {hole.tags[0].name}
                  </Badge>
                {/if}
                <span>·</span>
                <span>{relativeTime(hole.timeUpdated)}</span>
              </div>
              <p
                class="mb-0 mt-1.5 line-clamp-2 text-sm leading-5 text-foreground/90 transition group-hover:text-primary"
              >
                {excerpt(hole)}
              </p>
              <span class="mt-1.5 flex items-center gap-1 text-[11px] text-muted-foreground">
                <MessageCircle size={11} /> {hole.reply}
              </span>
            </div>
            <ChevronRight size={15} class="text-muted-foreground/45 transition group-hover:translate-x-0.5" />
          </button>
        {/each}
        {#if compactItems.length === 0}
          <p class="m-0 py-8 text-center text-sm text-muted-foreground md:col-span-3">暂无讨论</p>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<ForumConversation controller={conversation} />

{#if composer}
  <ForumComposer
    mode={composer.mode}
    divisions={forum.meta.phase === "ready" ? forum.meta.divisions : []}
    tags={forum.meta.phase === "ready" ? forum.meta.tags : []}
    initialDivisionId={selectedDivisionId}
    targetLabel={composer.mode === "post" ? "" : composer.targetLabel}
    onCancel={() => (composer = null)}
    onSubmit={submitComposer}
  />
{/if}

<style>
  .forum-list-item,
  .virtual-floor {
    content-visibility: auto;
    contain-intrinsic-size: auto 112px;
  }
</style>
