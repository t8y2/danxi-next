<script lang="ts">
  import BookOpenCheck from "@lucide/svelte/icons/book-open-check";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import FileText from "@lucide/svelte/icons/file-text";
  import GraduationCap from "@lucide/svelte/icons/graduation-cap";
  import LogIn from "@lucide/svelte/icons/log-in";
  import MessageSquareText from "@lucide/svelte/icons/message-square-text";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import EvaluationReviewCard from "$lib/components/evaluation/EvaluationReviewCard.svelte";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import { evaluation } from "$lib/stores/evaluation.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { cn } from "$lib/utils";

  interface Props {
    onLogin: () => void;
  }

  let { onLogin }: Props = $props();
  let searchText = $state("");

  $effect(() => {
    if (!session.ready) return;
    if (session.status.communityLoggedIn) {
      void evaluation.loadRandom();
    } else {
      evaluation.requireLogin();
    }
  });

  function submitSearch(event: SubmitEvent) {
    event.preventDefault();
    void evaluation.search(searchText);
  }

  function clearSearch() {
    searchText = "";
    void evaluation.search("");
  }
</script>

<section class="py-5 lg:flex lg:h-full lg:min-h-0 lg:flex-col lg:pb-0 lg:pt-6">
  <form class="flex shrink-0 items-center gap-2" onsubmit={submitSearch}>
    <div class="focus-within:ring-primary/25 flex h-10 min-w-0 flex-1 items-center rounded-xl border border-border bg-card px-3 transition focus-within:ring-2">
      <Search size={16} class="shrink-0 text-muted-foreground" />
      <input
        bind:value={searchText}
        class="h-full min-w-0 flex-1 border-0 bg-transparent px-2.5 text-sm outline-none placeholder:text-muted-foreground/70"
        placeholder="课程名称、代码或教师姓名"
        aria-label="搜索课程评价"
      />
      {#if searchText}
        <button type="button" class="text-xs text-muted-foreground hover:text-foreground" onclick={clearSearch}>
          清除
        </button>
      {/if}
    </div>
    <Button type="submit" class="h-10"><Search size={15} /> 搜索</Button>
  </form>

  {#if !session.ready}
    <div class="mt-5 grid min-h-64 gap-3 rounded-xl border border-border bg-card p-6">
      <div class="h-4 w-1/4 animate-pulse rounded bg-muted"></div>
      <div class="h-6 w-2/5 animate-pulse rounded bg-muted"></div>
      <div class="h-24 animate-pulse rounded bg-muted"></div>
    </div>
  {:else if !session.status.communityLoggedIn}
    <div class="flex min-h-[28rem] items-center justify-center">
      <div class="flex max-w-sm flex-col items-center text-center">
        <span class="grid size-14 place-items-center rounded-full bg-primary/10 text-primary">
          <BookOpenCheck size={25} strokeWidth={1.8} />
        </span>
        <h2 class="mb-0 mt-5 text-xl font-semibold tracking-[-0.02em]">登录后查看评教</h2>
        <p class="mb-0 mt-2 text-sm leading-6 text-muted-foreground">
          使用旦挞账号搜索课程、教师与真实课程评价
        </p>
        <Button class="mt-6" onclick={onLogin}><LogIn size={15} /> 登录旦挞账号</Button>
      </div>
    </div>
  {:else if evaluation.query}
    <div class="mt-5 grid items-start gap-5 lg:min-h-0 lg:flex-1 lg:grid-cols-[minmax(17rem,0.78fr)_minmax(0,1.45fr)] lg:items-stretch">
      <section class="flex min-h-0 flex-col overflow-hidden rounded-xl border border-border bg-card">
        <div class="flex shrink-0 items-center justify-between border-b border-border px-4 py-3">
          <span class="text-sm font-medium">搜索结果</span>
          {#if evaluation.searchResults.phase === "ready"}
            <Badge variant="muted">{evaluation.searchResults.data.length} 门</Badge>
          {/if}
        </div>

        {#if evaluation.searchResults.phase === "loading"}
          <div class="space-y-0 px-4 lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
            {#each Array(5) as _}
              <div class="border-b border-border py-4 last:border-b-0">
                <div class="h-4 w-2/3 animate-pulse rounded bg-muted"></div>
                <div class="mt-2 h-3 w-1/3 animate-pulse rounded bg-muted"></div>
              </div>
            {/each}
          </div>
        {:else if evaluation.searchResults.phase === "error"}
          <div class="p-6 text-center text-sm text-muted-foreground lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
            <p class="m-0">{evaluation.searchResults.message}</p>
            <Button variant="outline" size="sm" class="mt-4" onclick={() => evaluation.search(evaluation.query)}>重试</Button>
          </div>
        {:else if evaluation.searchResults.phase === "unauthenticated"}
          <div class="p-6 text-center lg:min-h-0 lg:flex-1 lg:overflow-y-auto"><Button size="sm" onclick={onLogin}>重新登录</Button></div>
        {:else if evaluation.searchResults.phase === "ready" && evaluation.searchResults.data.length === 0}
          <div class="p-8 text-center text-sm text-muted-foreground lg:min-h-0 lg:flex-1 lg:overflow-y-auto">没有找到匹配的课程</div>
        {:else if evaluation.searchResults.phase === "ready"}
          <div class="lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
            {#each evaluation.searchResults.data as group}
              <button
                type="button"
                class={cn(
                  "flex w-full items-center gap-3 border-b border-border px-4 py-3.5 text-left transition last:border-b-0 hover:bg-muted/40",
                  evaluation.selectedGroupId === group.groupId && "bg-primary/8",
                )}
                onclick={() => evaluation.selectGroup(group.groupId)}
              >
                <div class="min-w-0 flex-1">
                  <p class="m-0 truncate text-sm font-medium">{group.name}</p>
                  <p class="mb-0 mt-1 truncate text-xs text-muted-foreground">
                    {[group.department, group.code].filter(Boolean).join(" · ")}
                  </p>
                  <div class="mt-2 flex items-center gap-3 text-[11px] text-muted-foreground">
                    <span class="inline-flex items-center gap-1"><FileText size={12} /> {group.courseCount}</span>
                    <span class="inline-flex items-center gap-1"><MessageSquareText size={12} /> {group.reviewCount}</span>
                  </div>
                </div>
                <ChevronRight size={15} class="shrink-0 text-muted-foreground/60" />
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border border-border bg-card">
        {#if evaluation.courseDetail.phase === "loading" || evaluation.courseDetail.phase === "idle"}
          <div class="space-y-4 p-5 lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
            <div class="h-5 w-1/3 animate-pulse rounded bg-muted"></div>
            <div class="h-3.5 w-1/2 animate-pulse rounded bg-muted"></div>
            <div class="mt-7 h-28 animate-pulse rounded-lg bg-muted"></div>
          </div>
        {:else if evaluation.courseDetail.phase === "error"}
          <div class="flex min-h-64 flex-col items-center justify-center gap-3 p-6 text-center text-sm text-muted-foreground lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
            <p class="m-0">{evaluation.courseDetail.message}</p>
            {#if evaluation.selectedGroupId}
              <Button variant="outline" size="sm" onclick={() => evaluation.selectGroup(evaluation.selectedGroupId!)}>重试</Button>
            {/if}
          </div>
        {:else if evaluation.courseDetail.phase === "unauthenticated"}
          <div class="flex min-h-64 items-center justify-center lg:min-h-0 lg:flex-1"><Button size="sm" onclick={onLogin}>重新登录</Button></div>
        {:else}
          {@const detail = evaluation.courseDetail.data}
          <header class="shrink-0 border-b border-border px-5 py-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0">
                <h2 class="m-0 text-lg font-semibold tracking-[-0.02em]">{detail.group.name}</h2>
                <p class="mb-0 mt-1 text-xs text-muted-foreground">
                  {[detail.group.department, detail.group.code].filter(Boolean).join(" · ")}
                </p>
              </div>
              <div class="flex flex-wrap gap-1.5">
                {#each detail.group.credits as credit}
                  <Badge variant="warning">{credit.toFixed(1)} 学分</Badge>
                {/each}
                <Badge variant="muted">{detail.reviews.length} 条评价</Badge>
              </div>
            </div>
          </header>
          <div class="px-5 lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
            {#if detail.reviews.length === 0}
              <div class="flex min-h-60 flex-col items-center justify-center gap-2 text-muted-foreground">
                <GraduationCap size={24} />
                <p class="m-0 text-sm">该课程还没有评价</p>
              </div>
            {:else}
              {#each detail.reviews as review (review.reviewId)}
                <EvaluationReviewCard {review} />
              {/each}
            {/if}
          </div>
        {/if}
      </section>
    </div>
  {:else}
    <section class="mt-5">
      <div class="mb-3 flex items-center justify-between px-1">
        <div class="flex min-w-0 items-baseline gap-2.5">
          <h2 class="m-0 shrink-0 whitespace-nowrap text-[16px] font-semibold tracking-[-0.015em]">随便看看</h2>
          <p class="m-0 min-w-0 truncate text-xs text-muted-foreground">来自旦课的随机课程评价</p>
        </div>
        <Button variant="ghost" size="sm" class="text-muted-foreground" onclick={() => evaluation.loadRandom(true)}>
          <RefreshCw size={14} /> 换一条
        </Button>
      </div>

      {#if evaluation.randomReview.phase === "loading" || evaluation.randomReview.phase === "idle"}
        <div class="rounded-xl border border-border bg-card p-6">
          <div class="h-4 w-1/4 animate-pulse rounded bg-muted"></div>
          <div class="mt-5 h-6 w-2/5 animate-pulse rounded bg-muted"></div>
          <div class="mt-4 h-24 animate-pulse rounded bg-muted"></div>
        </div>
      {:else if evaluation.randomReview.phase === "error"}
        <div class="flex min-h-64 flex-col items-center justify-center gap-3 rounded-xl border border-border bg-card p-6 text-center text-sm text-muted-foreground">
          <p class="m-0">{evaluation.randomReview.message}</p>
          <Button variant="outline" size="sm" onclick={() => evaluation.loadRandom(true)}>重试</Button>
        </div>
      {:else if evaluation.randomReview.phase === "unauthenticated"}
        <div class="flex min-h-64 items-center justify-center rounded-xl border border-border bg-card">
          <Button onclick={onLogin}>登录旦挞账号</Button>
        </div>
      {:else}
        <EvaluationReviewCard review={evaluation.randomReview.data} prominent />
      {/if}
    </section>
  {/if}
</section>
