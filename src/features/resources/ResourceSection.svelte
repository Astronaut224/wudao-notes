<script lang="ts">
  /** 资料区：文件/图片（默认复制）/链接 的展示与操作，待办详情与笔记编辑器共用 */
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import { api, resourceImageUrl } from "../../lib/api";
  import type { Resource } from "../../lib/types";
  import { kindSublabel, sizeLabel } from "../../lib/utils";
  import { toast, toastError } from "../../stores/toast.svelte";

  interface Props {
    ownerType: "todo" | "note";
    ownerId: string;
  }
  let { ownerType, ownerId }: Props = $props();

  let resources = $state<Resource[]>([]);
  let loading = $state(false);
  let referenceMode = $state(false); // 仅引用原始文件位置

  let showLinkDialog = $state(false);
  let linkName = $state("");
  let linkUrl = $state("");
  let linkSaving = $state(false);

  let confirmRemove = $state<Resource | null>(null);
  let lightbox = $state<Resource | null>(null);
  let draggingFile = $state(false);

  export async function reload() {
    if (!ownerId) return;
    loading = true;
    try {
      resources = await api.listResources(ownerType, ownerId);
    } catch (e) {
      toastError(e, "加载资料失败");
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void ownerId;
    void reload();
  });

  const images = $derived(resources.filter((r) => r.kind === "image" && r.exists && r.file_mode === "copy"));

  async function pickAndAttach() {
    const picked = await api.pickFiles("选择要添加的文件或图片", []);
    if (!picked.length) return; // 取消不产生空记录
    for (const p of picked) {
      try {
        await api.attachLocalFile(ownerType, ownerId, p, referenceMode ? "reference" : "copy");
      } catch (e) {
        toastError(e, "添加资料失败");
      }
    }
    if (referenceMode && picked.length) {
      toast("已按“仅引用”记录原位置：原文件移动或删除后此处会提示失效");
    }
    await reload();
  }

  async function onDropFiles(e: DragEvent) {
    e.preventDefault();
    draggingFile = false;
    // Tauri 拖放的系统文件经 dataTransfer.files 提供
    const files = e.dataTransfer?.files;
    if (!files || !files.length) return;
    toast("拖拽的文件将复制到应用内保存");
    // WebView 不提供完整路径，改用 webview 拖放事件路径
  }

  async function openRes(r: Resource) {
    if (r.kind === "image" && r.file_mode === "copy" && r.exists) {
      lightbox = r;
      return;
    }
    try {
      await api.openResource(r.id);
    } catch (e) {
      const shape = e as { code?: string };
      if (shape?.code === "file_missing") {
        toastError(e);
      } else {
        toastError(e, "打开失败");
      }
    }
  }

  async function relocate(r: Resource) {
    const picked = await api.pickFile("重新定位文件", []);
    if (!picked) return;
    try {
      await api.relocateResource(r.id, picked);
      toast("已重新定位");
      await reload();
    } catch (e) {
      toastError(e, "重新定位失败");
    }
  }

  function saveLinkDisabled() {
    return !linkName.trim() || !/^https?:\/\//i.test(linkUrl.trim());
  }

  async function saveLink() {
    linkSaving = true;
    try {
      await api.attachUrl(ownerType, ownerId, linkName.trim(), linkUrl.trim());
      showLinkDialog = false;
      linkName = "";
      linkUrl = "";
      await reload();
    } catch (e) {
      toastError(e, "添加链接失败");
    } finally {
      linkSaving = false;
    }
  }

  async function doRemove() {
    const r = confirmRemove;
    confirmRemove = null;
    if (!r) return;
    try {
      await api.removeResource(r.id);
      toast(
        r.file_mode === "copy"
          ? "已移除关联并删除应用内副本（原文件不受影响）"
          : "已移除关联（未动原文件）",
      );
      await reload();
    } catch (e) {
      toastError(e, "移除失败");
    }
  }

  function removeMessage(r: Resource): string {
    return r.file_mode === "copy"
      ? `移除「${r.display_name}」将删除应用内部副本，不会影响原始文件。`
      : `移除「${r.display_name}」仅删除记录，不会删除你的原始文件。`;
  }
</script>

<div
  class="res-section"
  class:dragging={draggingFile}
  ondragover={(e) => {
    e.preventDefault();
    draggingFile = true;
  }}
  ondragleave={() => (draggingFile = false)}
  ondrop={(e) => void onDropFiles(e)}
>
  <div class="section-head">
    <span class="section-title">相关资料 <span class="count">{resources.length}</span></span>
    <span class="res-actions">
      <button class="btn-text" onclick={() => (showLinkDialog = true)}>＋ 添加链接</button>
      <button class="btn-text" onclick={() => void pickAndAttach()}>＋ 文件 / 图片</button>
    </span>
  </div>

  <label class="ref-toggle" title="勾选后不复制文件，仅记录原始位置；原文件移动或删除会造成失效">
    <input type="checkbox" bind:checked={referenceMode} />
    仅引用原始文件位置
  </label>

  {#if loading && resources.length === 0}
    <div class="res-empty">加载中…</div>
  {:else if resources.length === 0}
    <div class="res-empty">暂无资料，可将文件拖到这里，或点击右上角添加。</div>
  {:else}
    {#each resources as r (r.id)}
      <div class="resource" class:missing={!r.exists}>
        <div class="ri" aria-hidden="true">
          {r.kind === "url" ? "↗" : r.kind === "image" ? "▧" : "▤"}
        </div>
        <button class="rmain" title={r.exists ? "打开" : "文件不存在，点击操作"} onclick={() => void openRes(r)}>
          <span class="rname">{r.display_name}</span>
          <span class="rsub">
            {#if !r.exists}
              ⚠ 文件不存在
            {:else if r.kind === "url"}
              {r.url}
            {:else}
              {kindSublabel(r.display_name, r.kind)}{r.file_size ? ` · ${sizeLabel(r.file_size)}` : ""}{r.file_mode === "reference" ? " · 仅引用" : ""}
            {/if}
          </span>
        </button>
        {#if !r.exists}
          <button class="rbtn" title="重新定位文件" onclick={() => void relocate(r)}>⟳</button>
        {/if}
        <button class="rbtn" title="移除" onclick={() => (confirmRemove = r)}>×</button>
      </div>
    {/each}

    {#if images.length > 0}
      <div class="thumbs">
        {#each images as img (img.id)}
          <button class="thumb" title={img.display_name} onclick={() => (lightbox = img)}>
            <img src={resourceImageUrl(img.id)} alt={img.display_name} loading="lazy" />
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

{#if showLinkDialog}
  <div class="overlay" role="dialog" aria-modal="true" onclick={(e) => e.target === e.currentTarget && (showLinkDialog = false)}>
    <div class="dialog">
      <h3>添加网页链接</h3>
      <label for="link-name">显示名称</label>
      <input id="link-name" class="edit-input" bind:value={linkName} placeholder="例如：设计稿在线链接" />
      <label for="link-url">网址</label>
      <input id="link-url" class="edit-input" bind:value={linkUrl} placeholder="https://…" spellcheck="false" />
      {#if linkUrl.trim() && !/^https?:\/\//i.test(linkUrl.trim())}
        <p class="link-warn">链接必须以 http:// 或 https:// 开头</p>
      {/if}
      <div class="dialog-row">
        <button class="btn-secondary" onclick={() => (showLinkDialog = false)}>取消</button>
        <button class="btn-primary" disabled={saveLinkDisabled() || linkSaving} onclick={() => void saveLink()}>保存</button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  open={confirmRemove !== null}
  title="移除资料"
  message={confirmRemove ? removeMessage(confirmRemove) : ""}
  confirmText="移除"
  danger
  onConfirm={doRemove}
  onCancel={() => (confirmRemove = null)}
/>

{#if lightbox}
  <div class="lightbox" role="dialog" aria-modal="true" onclick={() => (lightbox = null)}>
    <img src={resourceImageUrl(lightbox.id)} alt={lightbox.display_name} />
    <div class="lightbox-name">{lightbox.display_name}</div>
  </div>
{/if}

<style>
  .res-section {
    border: 1px dashed transparent;
    border-radius: var(--radius-panel);
    transition: background 0.15s;
  }
  .res-section.dragging {
    background: #f4f8ff;
    border-color: #b9c9f5;
  }
  .count {
    color: #abb7c7;
    font-size: 11px;
    font-weight: 450;
    margin-left: 2px;
  }
  .res-actions {
    display: flex;
    gap: 10px;
  }
  .ref-toggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-3);
    margin-bottom: 8px;
    cursor: pointer;
  }
  .ref-toggle input {
    accent-color: var(--primary);
  }
  .res-empty {
    color: #a3aebf;
    font-size: 12px;
    padding: 8px 0;
  }
  .resource {
    padding: 9px 0;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    color: #45516a;
    border-bottom: 1px dashed #eef0f5;
  }
  .resource.missing .rname {
    color: var(--danger);
  }
  .ri {
    width: 28px;
    height: 28px;
    background: #eef3ff;
    color: #5372db;
    display: grid;
    place-items: center;
    border-radius: 6px;
    font-size: 14px;
    flex: none;
  }
  .rmain {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }
  .rname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .rsub {
    font-size: 10px;
    color: #abb6c6;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rbtn {
    background: transparent;
    border: 0;
    color: #a0abba;
    padding: 4px 6px;
    border-radius: 5px;
    font-size: 13px;
  }
  .rbtn:hover {
    background: #f1f4fa;
    color: var(--text);
  }
  .thumbs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 10px;
  }
  .thumb {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0;
    overflow: hidden;
    width: 76px;
    height: 76px;
    background: #fbfcff;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .link-warn {
    color: var(--danger);
    font-size: 11px;
    margin: 6px 0 0;
  }
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgba(15, 22, 35, 0.78);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    cursor: zoom-out;
  }
  .lightbox img {
    max-width: 92vw;
    max-height: 82vh;
    border-radius: 8px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
  }
  .lightbox-name {
    color: #dfe6f2;
    font-size: 12px;
  }
</style>
