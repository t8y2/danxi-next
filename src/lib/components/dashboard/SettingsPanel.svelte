<script lang="ts">
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import LogOut from "@lucide/svelte/icons/log-out";
  import ThemeSelector from "$lib/components/settings/ThemeSelector.svelte";
  import { Switch } from "$lib/components/ui/switch";
  import { communityNetwork } from "$lib/stores/community-network.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { Button } from "$lib/components/ui/button";

  interface Props {
    onLogin: (method: "community" | "campus") => void;
  }

  let { onLogin }: Props = $props();

  const communityStatus = $derived(
    session.status.communityLoggedIn
      ? (session.status.communityUser?.nickname ?? "已登录")
      : "未登录",
  );
  const campusStatus = $derived(session.status.campusLoggedIn ? session.status.campusId : null);

  async function logoutCommunity() {
    await session.logout();
  }

  async function logoutCampus() {
    await session.logoutCampus();
  }
</script>

<section class="pb-10 pt-5 lg:pt-6">
  <div class="grid w-full items-start gap-7 lg:grid-cols-2 lg:gap-x-6 xl:gap-x-8">
    <div class="space-y-7">
      <section>
        <h2 class="mb-2 px-1 text-xs font-medium text-muted-foreground">外观</h2>
        <ThemeSelector />
      </section>
      <section>
        <h2 class="mb-2 px-1 text-xs font-medium text-muted-foreground">账户</h2>
        <div class="overflow-hidden rounded-xl border border-border bg-card">
          {#if campusStatus}
            <div class="flex w-full items-center gap-4 border-b border-border px-4 py-3">
              <span class="flex-1 text-sm">
                复旦统一认证
                <span class="ml-2 font-mono text-xs text-muted-foreground tabular-nums">{campusStatus}</span>
              </span>
              <Button variant="ghost" size="sm" class="text-muted-foreground" onclick={logoutCampus}>
                <LogOut size={13} /> 退出登录
              </Button>
            </div>
          {:else}
            <button
              type="button"
              class="flex w-full items-center gap-4 border-b border-border px-4 py-3.5 text-left hover:bg-muted/45"
              onclick={() => onLogin("campus")}
            >
              <span class="flex-1 text-sm">复旦统一认证</span>
              <span class="text-xs text-primary">登录</span>
              <ChevronRight size={14} class="text-muted-foreground/65" />
            </button>
          {/if}
          {#if session.status.communityLoggedIn}
            <div class="flex w-full items-center gap-4 px-4 py-3">
              <span class="flex-1 text-sm">
                旦挞账号
                <span class="ml-2 text-xs text-muted-foreground">{communityStatus}</span>
              </span>
              <Button variant="ghost" size="sm" class="text-muted-foreground" onclick={logoutCommunity}>
                <LogOut size={13} /> 退出登录
              </Button>
            </div>
          {:else}
            <button
              type="button"
              class="flex w-full items-center gap-4 px-4 py-3.5 text-left hover:bg-muted/45"
              onclick={() => onLogin("community")}
            >
              <span class="flex-1 text-sm">旦挞账号</span>
              <span class="text-xs text-primary">登录</span>
              <ChevronRight size={14} class="text-muted-foreground/65" />
            </button>
          {/if}
        </div>
      </section>
      <section>
        <h2 class="mb-2 px-1 text-xs font-medium text-muted-foreground">网络</h2>
        <div class="overflow-hidden rounded-xl border border-border bg-card">
          <div class="flex items-center gap-5 px-4 py-3.5">
            <div class="min-w-0 flex-1">
              <span id="use-webvpn-label" class="block text-sm">自动使用 WebVPN</span>
              <span class="mt-0.5 block text-xs leading-5 text-muted-foreground">
                旦挞无法直连时，复用复旦 UIS 会话自动连接
              </span>
            </div>
            <Switch
              id="use-webvpn"
              aria-labelledby="use-webvpn-label"
              checked={communityNetwork.useWebvpn}
              onCheckedChange={(checked) => communityNetwork.setUseWebvpn(checked)}
            />
          </div>
        </div>
      </section>
    </div>

    <div class="space-y-7">
      <section>
        <h2 class="mb-2 px-1 text-xs font-medium text-muted-foreground">数据与安全</h2>
        <div class="overflow-hidden rounded-xl border border-border bg-card">
          {#each [["本地缓存", "启用"], ["凭证存储", "受控后端"], ["诊断日志", "自动脱敏"]] as item}
            <div class="flex items-center gap-4 border-b border-border px-4 py-3.5 last:border-b-0">
              <span class="flex-1 text-sm">{item[0]}</span>
              <span class="text-xs text-muted-foreground">{item[1]}</span>
            </div>
          {/each}
        </div>
      </section>
      <section>
        <h2 class="mb-2 px-1 text-xs font-medium text-muted-foreground">运行方式</h2>
        <div class="overflow-hidden rounded-xl border border-border bg-card">
          {#each [["桌面端", "Tauri invoke"], ["Web 端", "HTTP API 网关"]] as item}
            <div class="flex items-center gap-4 border-b border-border px-4 py-3.5 last:border-b-0">
              <span class="flex-1 text-sm">{item[0]}</span>
              <span class="text-xs text-muted-foreground">{item[1]}</span>
            </div>
          {/each}
        </div>
      </section>
    </div>
  </div>
</section>
