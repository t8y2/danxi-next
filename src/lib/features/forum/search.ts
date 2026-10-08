export type ForumSearchIntent =
  | { kind: "empty" }
  | { kind: "hole"; holeId: number }
  | { kind: "search"; query: string }
  | { kind: "invalid"; message: string };

export function parseForumSearchInput(input: string): ForumSearchIntent {
  const query = input.trim();
  if (!query) return { kind: "empty" };
  if (/^#\d+$/.test(query)) {
    const holeId = Number(query.slice(1));
    return Number.isSafeInteger(holeId) && holeId > 0
      ? { kind: "hole", holeId }
      : { kind: "invalid", message: "请输入有效的帖子编号" };
  }
  if ([...query].length > 200) return { kind: "invalid", message: "搜索词不能超过 200 个字符" };
  return { kind: "search", query };
}
