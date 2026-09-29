<script lang="ts">
  import { toTransportError, desktopWindow } from "$lib/api";
  import { Button } from "$lib/components/ui/button";
  import { session } from "$lib/stores/session.svelte";

  type LoginMethod = "community" | "campus";
  type CommunityMode = "login" | "register";

  interface Props {
    open: boolean;
    onClose: () => void;
    initialMethod?: LoginMethod;
  }

  let { open = false, onClose, initialMethod = "community" }: Props = $props();

  let communityMode = $state<CommunityMode>("login");
  let email = $state("");
  let password = $state("");
  let verifyCode = $state("");
  let busy = $state(false);
  let codeSent = $state(false);
  let countdown = $state(0);
  let error = $state("");
  let notice = $state("");
  let emailFocused = $state(false);
  let activeSuggestion = $state(0);
  let campusId = $state("");
  let campusPassword = $state("");

  const isCampus = $derived(initialMethod === "campus");
  const isRegistering = $derived(communityMode === "register");
  const emailValid = $derived(email.includes("@") && email.trim().length > 3);
  const passwordValid = $derived(password.length >= 6);
  const codeValid = $derived(/^\d{4,8}$/.test(verifyCode.trim()));
  const campusIdValid = $derived(/^\d{6,}$/.test(campusId.trim()));
  const campusPasswordValid = $derived(campusPassword.length >= 6);
  const submitBlocked = $derived(
    busy ||
      (isCampus
        ? !campusIdValid || !campusPasswordValid
        : !emailValid || !passwordValid || (isRegistering && !codeValid)),
  );
  const emailSuggestions = $derived(
    /^\d{6,}$/.test(email.trim())
      ? [`${email.trim()}@m.fudan.edu.cn`, `${email.trim()}@fudan.edu.cn`]
      : [],
  );
  const suggestionOpen = $derived(
    !isCampus && emailFocused && emailSuggestions.length > 0,
  );
  const dialogTitle = $derived(
    isCampus ? "登录复旦 UIS" : isRegistering ? "注册旦挞账号" : "登录旦挞账号",
  );
  const dialogDescription = $derived(
    isCampus
      ? "用于日程与校园服务"
      : isRegistering
        ? "注册后即可使用茶楼与评教"
        : "用于茶楼与评教等社区服务",
  );

  const FORGOT_PASSWORD_URL = "https://auth.fduhole.com/register?type=forget_password";

  $effect(() => {
    if (!open) return;
    communityMode = "login";
    password = "";
    verifyCode = "";
    busy = false;
    codeSent = false;
    countdown = 0;
    error = "";
    notice = "";
    emailFocused = false;
    activeSuggestion = 0;
    campusId = "";
    campusPassword = "";
  });

  $effect(() => {
    if (countdown <= 0) return;
    const timer = setTimeout(() => (countdown -= 1), 1000);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (activeSuggestion >= emailSuggestions.length) activeSuggestion = 0;
  });

  function pickSuggestion(value: string) {
    email = value;
    emailFocused = false;
  }

  function handleEmailKeydown(event: KeyboardEvent) {
    if (!suggestionOpen) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      activeSuggestion = (activeSuggestion + 1) % emailSuggestions.length;
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      activeSuggestion =
        (activeSuggestion - 1 + emailSuggestions.length) % emailSuggestions.length;
    } else if (event.key === "Enter") {
      event.preventDefault();
      pickSuggestion(emailSuggestions[activeSuggestion]);
    } else if (event.key === "Escape") {
      emailFocused = false;
    }
  }

  function setCommunityMode(nextMode: CommunityMode) {
    communityMode = nextMode;
    password = "";
    verifyCode = "";
    error = "";
    notice = "";
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (submitBlocked) return;
    error = "";
    notice = "";
    busy = true;

    try {
      if (isCampus) {
        const result = await session.loginCampus(campusId.trim(), campusPassword);
        campusPassword = "";
        if (result.state === "requiresSecondFactor") {
          notice = result.message;
          await session.completeCampusSecondFactor();
        }
      } else if (isRegistering) {
        await session.register(email.trim(), password, verifyCode.trim());
        password = "";
        verifyCode = "";
      } else {
        await session.login(email.trim(), password);
        password = "";
      }
      onClose();
    } catch (raw) {
      notice = "";
      error = toTransportError(raw).message;
    } finally {
      busy = false;
    }
  }

  async function sendCode() {
    if (!emailValid || countdown > 0 || busy) return;
    error = "";
    busy = true;
    try {
      await session.sendVerificationCode(email.trim());
      codeSent = true;
      countdown = 60;
    } catch (raw) {
      error = toTransportError(raw).message;
    } finally {
      busy = false;
    }
  }

  function requestClose() {
    if (busy) return;
    onClose();
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/45 p-4"
    role="presentation"
    onclick={requestClose}
    onkeydown={(event) => event.key === "Escape" && requestClose()}
  >
    <div
      class="max-h-[calc(100vh-2rem)] w-full max-w-sm overflow-y-auto rounded-xl border border-border bg-card p-6 shadow-lg"
      role="dialog"
      aria-modal="true"
      aria-label={dialogTitle}
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        event.stopPropagation();
        if (event.key === "Escape") requestClose();
      }}
    >
      <div class="mb-5">
        <h2 class="m-0 text-lg font-semibold tracking-[-0.015em]">{dialogTitle}</h2>
        <p class="mb-0 mt-1 text-xs text-muted-foreground">{dialogDescription}</p>
      </div>

      <form class="space-y-4" onsubmit={submit}>
        {#if isCampus}
          <label class="block" for="campus-id">
            <span class="mb-1.5 block text-xs font-medium text-muted-foreground">学号</span>
            <input
              id="campus-id"
              type="text"
              inputmode="numeric"
              bind:value={campusId}
              autocomplete="username"
              placeholder="输入学号"
              class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm tabular-nums outline-none transition focus:border-primary focus:ring-1 focus:ring-primary"
            />
          </label>
          <label class="block" for="campus-password">
            <span class="mb-1.5 block text-xs font-medium text-muted-foreground">UIS 密码</span>
            <input
              id="campus-password"
              type="password"
              bind:value={campusPassword}
              autocomplete="current-password"
              placeholder="输入 UIS 密码"
              class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition focus:border-primary focus:ring-1 focus:ring-primary"
            />
          </label>
          <p class="m-0 text-[11px] leading-4 text-muted-foreground">
            凭证仅交给后端处理；桌面端加密存储，Web 端仅保存在服务端会话中。
          </p>
        {:else}
          <div class="relative">
            <label class="block" for="community-email">
              <span class="mb-1.5 block text-xs font-medium text-muted-foreground">邮箱</span>
              <input
                id="community-email"
                type="email"
                bind:value={email}
                autocomplete="email"
                placeholder="name@example.com 或学号"
                role="combobox"
                aria-expanded={suggestionOpen}
                aria-controls="email-suggestions"
                aria-activedescendant={suggestionOpen ? `email-suggestion-${activeSuggestion}` : undefined}
                onfocus={() => (emailFocused = true)}
                onblur={() => (emailFocused = false)}
                onkeydown={handleEmailKeydown}
                class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition focus:border-primary focus:ring-1 focus:ring-primary"
              />
            </label>
            {#if suggestionOpen}
              <ul
                id="email-suggestions"
                role="listbox"
                aria-label="校园邮箱候选"
                class="absolute inset-x-0 top-full z-10 m-0 mt-1 list-none overflow-hidden rounded-lg border border-border bg-card p-0 shadow-md"
              >
                {#each emailSuggestions as suggestion, index}
                  <li id={`email-suggestion-${index}`} role="option" aria-selected={index === activeSuggestion}>
                    <button
                      type="button"
                      class={`flex w-full items-center justify-between px-3 py-2 text-left text-sm transition ${
                        index === activeSuggestion
                          ? "bg-primary/10 text-primary"
                          : "text-foreground hover:bg-foreground/8"
                      }`}
                      onmousedown={(event) => event.preventDefault()}
                      onclick={() => pickSuggestion(suggestion)}
                    >
                      <span>{suggestion}</span>
                      <span class="text-[10px] text-muted-foreground">校园邮箱</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>

          <label class="block" for="community-password">
            <span class="mb-1.5 block text-xs font-medium text-muted-foreground">
              {isRegistering ? "设置密码（至少 6 位）" : "密码"}
            </span>
            <input
              id="community-password"
              type="password"
              bind:value={password}
              autocomplete={isRegistering ? "new-password" : "current-password"}
              placeholder="••••••••"
              class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition focus:border-primary focus:ring-1 focus:ring-primary"
            />
          </label>

          {#if isRegistering}
            <div>
              <label class="mb-1.5 block text-xs font-medium text-muted-foreground" for="verify-code">
                邮箱验证码
              </label>
              <div class="flex gap-2">
                <input
                  id="verify-code"
                  type="text"
                  inputmode="numeric"
                  bind:value={verifyCode}
                  autocomplete="one-time-code"
                  placeholder="6 位数字"
                  class="min-w-0 flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm tabular-nums outline-none transition focus:border-primary focus:ring-1 focus:ring-primary"
                />
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  class="h-auto shrink-0 px-3"
                  disabled={!emailValid || countdown > 0 || busy}
                  onclick={sendCode}
                >
                  {countdown > 0 ? `${countdown}s` : codeSent ? "重新发送" : "发送验证码"}
                </Button>
              </div>
            </div>
          {/if}

          <div class="flex min-h-8 items-center justify-between gap-2">
            {#if isRegistering}
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="-ml-2 px-2 text-primary hover:text-primary"
                onclick={() => setCommunityMode("login")}
              >
                已有账号，返回登录
              </Button>
            {:else}
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="-ml-2 px-2 text-primary hover:text-primary"
                onclick={() => setCommunityMode("register")}
              >
                注册新账户
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="-mr-2 cursor-pointer px-2 text-primary hover:underline"
                onclick={() => desktopWindow.openExternal(FORGOT_PASSWORD_URL)}
              >
                忘记密码？
              </Button>
            {/if}
          </div>
        {/if}

        {#if error}
          <p class="m-0 rounded-lg bg-destructive/10 px-3 py-2 text-xs text-destructive">{error}</p>
        {/if}
        {#if notice}
          <p class="m-0 rounded-lg bg-amber-500/12 px-3 py-2 text-xs text-amber-800 dark:text-amber-300">{notice}</p>
        {/if}

        <div class="flex justify-end gap-2 border-t border-border pt-4">
          <Button type="button" variant="ghost" size="sm" onclick={requestClose} disabled={busy}>
            取消
          </Button>
          <Button type="submit" size="sm" disabled={submitBlocked}>
            {busy ? "请稍候…" : isCampus ? "登录 UIS" : isRegistering ? "注册并登录" : "登录"}
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}
