<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import "../app.css";

  type VideoInfo = {
    title: string;
    qualities: number[];
  };

  let url = $state("");
  let fileName = $state("");
  let videoInfo = $state<VideoInfo | null>(null);
  let infoForUrl = $state("");
  let selectedQuality = $state<number | undefined>(undefined);
  let downloadMode = $state<"video" | "audio">("video");
  let isFetchingInfo = $state(false);
  let isDownloading = $state(false);
  let isOpeningResult = $state(false);
  let updateStatus = $state<"updating" | "success" | "error">("updating");
  let updateMessage = $state("yt-dlpの更新を確認しています...");
  let isUpdatingYtdlp = $derived(updateStatus === "updating");
  let progress = $state("");
  let message = $state("");
  let resultType = $state<"success" | "error" | "">("");
  let hasVideoInfo = $derived(videoInfo !== null && infoForUrl === url.trim());

  onMount(() => {
    void updateYtdlp();
  });

  async function updateYtdlp() {
    try {
      updateMessage = await invoke<string>("update_ytdlp");
      updateStatus = "success";
    } catch (error) {
      const details = error instanceof Error ? error.message : String(error);
      updateMessage = `yt-dlpの自動更新に失敗しました。現在のバージョンで続行します。\n${details}`;
      updateStatus = "error";
    }
  }

  function showError(error: unknown) {
    message = error instanceof Error ? error.message : String(error);
    resultType = "error";
  }

  async function openResultFolder() {
    if (isOpeningResult) return;
    isOpeningResult = true;
    message = "";
    resultType = "";

    try {
      await invoke<void>("open_result_dir");
    } catch (error) {
      showError(error);
    } finally {
      isOpeningResult = false;
    }
  }

  async function fetchVideoInfo() {
    const requestedUrl = url.trim();
    if (!requestedUrl || isFetchingInfo || isDownloading || isUpdatingYtdlp) return;

    isFetchingInfo = true;
    videoInfo = null;
    infoForUrl = "";
    selectedQuality = undefined;
    message = "";
    resultType = "";

    try {
      videoInfo = await invoke<VideoInfo>("get_video_info", { url: requestedUrl });
      infoForUrl = requestedUrl;
      selectedQuality = videoInfo.qualities[0];
      message = "動画情報を取得しました。";
      resultType = "success";
    } catch (error) {
      showError(error);
    } finally {
      isFetchingInfo = false;
    }
  }

  async function downloadVideo(event: SubmitEvent) {
    event.preventDefault();
    if (!hasVideoInfo || isDownloading || isFetchingInfo || isUpdatingYtdlp) return;
    if (downloadMode === "video" && selectedQuality === undefined) return;

    isDownloading = true;
    progress = "";
    message = "";
    resultType = "";

    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<string>("download-progress", (event) => {
        progress = event.payload;
      });
      message = await invoke<string>("download_video", {
        url: infoForUrl,
        quality: downloadMode === "video" ? selectedQuality : null,
        audioOnly: downloadMode === "audio",
        fileName: fileName.trim() || null,
      });
      resultType = "success";
    } catch (error) {
      showError(error);
    } finally {
      unlisten?.();
      isDownloading = false;
    }
  }
</script>

<svelte:head>
  <title>Mejiro YTDL</title>
  <meta name="description" content="URLを入力して動画をダウンロードします。" />
</svelte:head>

<main
  class="flex min-h-screen items-center justify-center bg-slate-950 px-5 py-12 text-slate-100"
>
  <section class="w-full max-w-xl">
    <header class="mb-8 text-center">
      <p class="mb-2 text-xs font-semibold tracking-[0.25em] text-cyan-300 uppercase">
        Mejiro YTDL
      </p>
      <h1 class="text-3xl font-bold tracking-tight sm:text-4xl">動画をダウンロード</h1>
      <p class="mt-3 text-sm text-slate-400">
        URLから画質を取得して、ダウンロード方法を選択できます。
      </p>
    </header>

    <div
      role={updateStatus === "error" ? "alert" : "status"}
      class={`mb-5 flex items-start gap-3 rounded-xl border px-4 py-3 text-xs ${
        updateStatus === "error"
          ? "border-amber-400/20 bg-amber-400/10 text-amber-200"
          : updateStatus === "updating"
            ? "border-slate-700 bg-slate-900 text-slate-300"
            : "border-emerald-400/20 bg-emerald-400/10 text-emerald-200"
      }`}
    >
      {#if isUpdatingYtdlp}
        <span
          class="mt-0.5 size-3 shrink-0 animate-spin rounded-full border-2 border-slate-400/30 border-t-slate-300"
        ></span>
      {/if}
      <p class="whitespace-pre-wrap break-all">{updateMessage}</p>
    </div>

    <form
      onsubmit={downloadVideo}
      class="rounded-2xl border border-slate-800 bg-slate-900 p-5 shadow-2xl shadow-black/20 sm:p-7"
    >
      <label for="video-url" class="mb-2 block text-sm font-medium text-slate-300">
        動画URL
      </label>
      <div class="flex flex-col gap-2 sm:flex-row">
        <input
          id="video-url"
          type="url"
          bind:value={url}
          placeholder="https://example.com/video"
          required
          disabled={isFetchingInfo || isDownloading || isUpdatingYtdlp}
          class="min-w-0 flex-1 rounded-xl border border-slate-700 bg-slate-950 px-4 py-3 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-cyan-400 focus:ring-2 focus:ring-cyan-400/20 disabled:opacity-60"
        />
        <button
          type="button"
          onclick={fetchVideoInfo}
          disabled={!url.trim() || isFetchingInfo || isDownloading || isUpdatingYtdlp}
          class="rounded-xl border border-slate-700 px-4 py-3 text-sm font-semibold text-slate-200 transition hover:border-cyan-400 hover:text-cyan-300 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {isFetchingInfo ? "取得中..." : "画質を取得"}
        </button>
      </div>

      {#if hasVideoInfo && videoInfo}
        <div class="mt-5 rounded-xl border border-slate-800 bg-slate-950/70 p-4">
          <p class="truncate text-sm font-medium text-slate-200" title={videoInfo.title}>
            {videoInfo.title}
          </p>

          <label for="file-name" class="mb-2 mt-4 block text-sm font-medium text-slate-300">
            保存ファイル名（任意）
          </label>
          <input
            id="file-name"
            type="text"
            bind:value={fileName}
            placeholder="空欄の場合は動画タイトルを使用"
            disabled={isDownloading || isUpdatingYtdlp}
            maxlength="200"
            class="w-full rounded-xl border border-slate-700 bg-slate-900 px-4 py-3 text-sm text-white outline-none transition placeholder:text-slate-600 focus:border-cyan-400 focus:ring-2 focus:ring-cyan-400/20 disabled:opacity-60"
          />
          <p class="mt-1.5 text-xs text-slate-500">拡張子は形式に合わせて自動で付与されます。</p>

          <fieldset class="mt-4">
            <legend class="mb-2 text-sm font-medium text-slate-300">ダウンロード形式</legend>
            <div class="grid grid-cols-2 gap-2">
              <label
                class={`cursor-pointer rounded-lg border px-3 py-2.5 text-center text-sm transition ${
                  downloadMode === "video"
                    ? "border-cyan-400 bg-cyan-400/10 text-cyan-200"
                    : "border-slate-700 text-slate-400 hover:border-slate-500"
                }`}
              >
                <input
                  class="sr-only"
                  type="radio"
                  name="download-mode"
                  bind:group={downloadMode}
                  value="video"
                />
                動画
              </label>
              <label
                class={`cursor-pointer rounded-lg border px-3 py-2.5 text-center text-sm transition ${
                  downloadMode === "audio"
                    ? "border-cyan-400 bg-cyan-400/10 text-cyan-200"
                    : "border-slate-700 text-slate-400 hover:border-slate-500"
                }`}
              >
                <input
                  class="sr-only"
                  type="radio"
                  name="download-mode"
                  bind:group={downloadMode}
                  value="audio"
                />
                音声のみ
              </label>
            </div>
          </fieldset>

          {#if downloadMode === "video"}
            <label for="video-quality" class="mb-2 mt-4 block text-sm font-medium text-slate-300">
              画質
            </label>
            {#if videoInfo.qualities.length > 0}
              <select
                id="video-quality"
                bind:value={selectedQuality}
                disabled={isDownloading || isUpdatingYtdlp}
                class="w-full rounded-xl border border-slate-700 bg-slate-900 px-4 py-3 text-sm text-white outline-none focus:border-cyan-400 focus:ring-2 focus:ring-cyan-400/20 disabled:opacity-60"
              >
                {#each videoInfo.qualities as quality}
                  <option value={quality}>{quality}p (最大)</option>
                {/each}
              </select>
            {:else}
              <p class="rounded-lg bg-amber-400/10 px-3 py-2 text-sm text-amber-200">
                動画画質を取得できませんでした。音声のみダウンロードできます。
              </p>
            {/if}
          {:else}
            <p class="mt-4 text-xs text-slate-500">
              利用可能な最高音質で音声を抽出します。
            </p>
          {/if}
        </div>

        <button
          type="submit"
          disabled={isDownloading ||
            isFetchingInfo ||
            isUpdatingYtdlp ||
            (downloadMode === "video" && selectedQuality === undefined)}
          class="mt-4 flex w-full items-center justify-center gap-2 rounded-xl bg-cyan-400 px-4 py-3 font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
        >
          {#if isDownloading}
            <span
              class="size-4 animate-spin rounded-full border-2 border-slate-950/30 border-t-slate-950"
            ></span>
            ダウンロード中...
          {:else}
            ダウンロード開始
          {/if}
        </button>
      {/if}

      {#if message}
        <div
          role={resultType === "error" ? "alert" : "status"}
          class={`mt-5 rounded-xl border px-4 py-3 text-sm ${
            resultType === "error"
              ? "border-rose-400/20 bg-rose-400/10 text-rose-200"
              : "border-emerald-400/20 bg-emerald-400/10 text-emerald-200"
          }`}
        >
          {message}
        </div>
      {/if}

      {#if isDownloading && progress}
        <div class="mt-5 rounded-xl bg-slate-950 px-4 py-3" aria-live="polite">
          <p class="mb-1 text-xs font-medium text-slate-500">進捗</p>
          <p class="break-all text-sm text-slate-300">{progress}</p>
        </div>
      {/if}
    </form>

    <div class="mt-5 flex flex-col items-center gap-3">
      <p class="text-center text-xs text-slate-500">
        ダウンロードしたファイルはアプリの <code class="text-slate-400">result</code>
        フォルダーに保存されます。
      </p>
      <button
        type="button"
        onclick={openResultFolder}
        disabled={isOpeningResult}
        class="rounded-xl border border-slate-700 px-4 py-2.5 text-sm font-medium text-slate-300 transition hover:border-cyan-400 hover:text-cyan-300 disabled:cursor-wait disabled:opacity-50"
      >
        {isOpeningResult ? "フォルダーを開いています..." : "resultフォルダーを開く"}
      </button>
    </div>
  </section>
</main>
