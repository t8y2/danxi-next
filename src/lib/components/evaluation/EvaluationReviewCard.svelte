<script lang="ts">
  import MessageSquareText from "@lucide/svelte/icons/message-square-text";
  import ThumbsUp from "@lucide/svelte/icons/thumbs-up";
  import { Badge } from "$lib/components/ui/badge";
  import type { EvaluationRating, EvaluationReview } from "$lib/types/app";

  interface Props {
    review: EvaluationReview;
    prominent?: boolean;
  }

  let { review, prominent = false }: Props = $props();

  const ratingLabels: Record<keyof EvaluationRating, string> = {
    overall: "总体",
    content: "课程风格",
    workload: "工作量",
    assessment: "考核",
  };

  const ratingWords: Record<keyof EvaluationRating, string[]> = {
    overall: ["特别差评", "差评", "中等", "好评", "特别好评"],
    content: ["硬核", "较难", "中等", "容易", "非常容易"],
    workload: ["非常大", "较大", "中等", "较小", "非常小"],
    assessment: ["非常严格", "严格", "中等", "宽松", "非常宽松"],
  };

  const ratingBadgeClasses: Record<keyof EvaluationRating, string> = {
    overall: "bg-blue-500/12 text-blue-700 dark:text-blue-300",
    content: "bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
    workload: "bg-amber-500/14 text-amber-800 dark:text-amber-300",
    assessment: "bg-violet-500/12 text-violet-700 dark:text-violet-300",
  };

  function ratingItems(rating: EvaluationRating) {
    return (Object.keys(ratingLabels) as Array<keyof EvaluationRating>)
      .map((key) => ({ key, value: rating[key] }))
      .filter((item): item is { key: keyof EvaluationRating; value: number } => item.value != null);
  }

  function ratingWord(key: keyof EvaluationRating, value: number): string {
    return ratingWords[key][Math.max(0, Math.min(4, value - 1))] ?? String(value);
  }

  function formatDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "";
    return new Intl.DateTimeFormat("zh-CN", {
      year: "numeric",
      month: "short",
      day: "numeric",
    }).format(date);
  }
</script>

<article class={prominent ? "rounded-xl border border-border bg-card p-5 lg:p-6" : "border-b border-border py-5 last:border-b-0"}>
  <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
    {#if review.courseName}<Badge variant="default">{review.courseName}</Badge>{/if}
    {#if review.teachers}<Badge variant="outline">{review.teachers}</Badge>{/if}
    {#if review.term}<span>{review.term}</span>{/if}
    {#if review.timeCreated}<span class="ml-auto tabular-nums">{formatDate(review.timeCreated)}</span>{/if}
  </div>

  <h3 class={`mb-0 mt-4 font-semibold tracking-[-0.018em] ${prominent ? "text-xl" : "text-[16px]"}`}>
    {review.title || "课程评价"}
  </h3>
  <p class="selectable mb-0 mt-2 whitespace-pre-wrap text-sm leading-7 text-foreground/88">
    {review.content || "这条评价暂时没有正文。"}
  </p>

  {#if ratingItems(review.rating).length > 0}
    <div class="mt-4 flex flex-wrap gap-2">
      {#each ratingItems(review.rating) as item}
        <Badge
          variant="muted"
          class={`font-medium tracking-normal ${ratingBadgeClasses[item.key]}`}
        >
          {ratingLabels[item.key]} · {ratingWord(item.key, item.value)}
        </Badge>
      {/each}
    </div>
  {/if}

  <div class="mt-4 flex items-center gap-4 text-xs text-muted-foreground">
    <span class="inline-flex items-center gap-1.5"><ThumbsUp size={13} /> {review.vote}</span>
    <span class="inline-flex items-center gap-1.5"><MessageSquareText size={13} /> {review.remark}</span>
  </div>
</article>
