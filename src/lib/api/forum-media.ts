const FORUM_STICKER_BASE_URL = "https://static.fduhole.com/stickers/";

export function forumStickerUrl(source: string): string | null {
  const match = /^(?:dx_|danxi_)([a-z0-9_]{1,64})$/.exec(source);
  return match ? `${FORUM_STICKER_BASE_URL}dx_${match[1]}.webp` : null;
}
