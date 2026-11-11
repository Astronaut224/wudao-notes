<script lang="ts">
  /** 设置：全局快捷键、开机启动、备份与恢复（底部⚙入口，非业务模块） */
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { api } from "../lib/api";
  import { appState } from "../stores/app.svelte";
  import { loadNotes } from "../stores/notes.svelte";
  import { loadTodos } from "../stores/todos.svelte";
  import { toast, toastError } from "../stores/toast.svelte";

  let open = $derived(appState.settingsOpen);

  let shortcut = $state("");
  let launchAtLogin = $state(false);
  let compactOnTop = $state(false);
  let saving = $state(false);

  let confirmRestore = $state<{ path: string } | null>(null);
  let busy = $state(false);

  $effect(() => {
    if (open) {
      void api
        .getSettings()
        .then((s) => {
          shortcut = s.global_shortcut ?? "Ctrl+Shift+Space";
          launchAtLogin = s.launch_at_login === "true";
          compactOnTop = s.compact_on_top === "true";
        })
        .catch(() => {});
      void api
        .shortcutStatus()
        .then((st) => {
          if (!st.registered) {
            toast(`全局快捷键 ${st.shortcut} 未生效（可能被占用），可用托盘呼出窗口`, "error");
          }
        })
        .catch(() => {});
    }
  });

  function close() {
    appState.settingsOpen = false;
  }

  async function saveShortcut() {
    if (saving) return;
    saving = true;
    try {
      await api.updateSetting("global_shortcut", shortcut.trim());
      toast("全局快捷键已更新");
    } catch (e) {
      toastError(e, "快捷键设置失败");
    } finally {
      saving = false;
    }
  }

  async function toggleLaunch() {
    const next = !launchAtLogin;
    try {
      await api.setLaunchAtLogin(next);
      launchAtLogin = next;
      await api.updateSetting("launch_at_login", next ? "true" : "false");
    } catch (e) {
      toastError(e, "设置开机启动失败");
    }
  }

  async function toggleCompactTop() {
    const next = !compactOnTop;
    try {
      await api.updateSetting("compact_on_top", next ? "true" : "false");
      compactOnTop = next;
    } catch (e) {
      toastError(e, "设置失败");
    }
  }

  async function doExport() {
    busy = true;
    try {
      const path = await api.exportBackup();
      toast(`备份已保存：${path}`);
    } catch (e) {
      const shape = e as { code?: string; message?: string };
      if (shape?.code !== "cancelled") toastError(e, "导出失败");
    } finally {
      busy = false;
    }
  }

  async function doRestorePrepare() {
    busy = true;
    try {
      const info = await api.prepareRestore();
      confirmRestore = info;
    } catch (e) {
      const shape = e as { code?: string };
      if (shape?.code !== "cancelled") toastError(e, "备份校验失败");
    } finally {
      busy = false;
    }
  }

  async function doRestoreExecute() {
    const info = confirmRestore;
    confirmRestore = null;
    if (!info) return;
    busy = true;
    try {
      const r = await api.executeRestore(info.path);
      await Promise.all([loadTodos(), loadNotes()]);
      toast(`恢复完成：${r.todos} 条待办、${r.notes} 篇笔记`);
    } catch (e) {
      toastError(e, "恢复失败");
    } finally {
      busy = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (open && e.key === "Escape" && !confirmRestore) close();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" onclick={(e) => e.target === e.currentTarget && close()}>
    <div class="dialog settings-dialog">
      <h3>设置</h3>

      <div class="set-row">
        <div class="set-info">
          <div class="set-name">全局快捷键</div>
          <div class="set-desc">呼出 / 隐藏窗口，例如 Ctrl+Shift+Space</div>
        </div>
        <div class="set-ctrl">
          <input class="edit-input shortcut-input" bind:value={shortcut} spellcheck="false" />
          <button class="btn-secondary" disabled={saving} onclick={() => void saveShortcut()}>应用</button>
        </div>
      </div>

      <div class="set-row">
        <div class="set-info">
          <div class="set-name">登录系统后启动</div>
          <div class="set-desc">开启后随系统启动并隐藏在托盘</div>
        </div>
        <div class="set-ctrl">
          <button class="btn-secondary" onclick={() => void toggleLaunch()}>
            {launchAtLogin ? "已开启" : "未开启"}
          </button>
        </div>
      </div>

      <div class="set-row">
        <div class="set-info">
          <div class="set-name">迷你窗置顶</div>
          <div class="set-desc">迷你悬浮窗默认保持在其他窗口上方</div>
        </div>
        <div class="set-ctrl">
          <button class="btn-secondary" onclick={() => void toggleCompactTop()}>
            {compactOnTop ? "已开启" : "未开启"}
          </button>
        </div>
      </div>

      <div class="hr"></div>

      <div class="set-row">
        <div class="set-info">
          <div class="set-name">备份</div>
          <div class="set-desc">导出数据库与托管附件（zip）</div>
        </div>
        <div class="set-ctrl">
          <button class="btn-secondary" disabled={busy} onclick={() => void doExport()}>导出备份…</button>
        </div>
      </div>

      <div class="set-row">
        <div class="set-info">
          <div class="set-name">恢复</div>
          <div class="set-desc">从备份恢复，将覆盖当前全部数据</div>
        </div>
        <div class="set-ctrl">
          <button class="btn-secondary" disabled={busy} onclick={() => void doRestorePrepare()}>从备份恢复…</button>
        </div>
      </div>

      <div class="dialog-row">
        <button class="btn-secondary" onclick={close}>关闭</button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  open={confirmRestore !== null}
  title="确认恢复备份"
  message={`将用备份覆盖当前全部待办、笔记与托管附件，且无法撤销。\n\n备份文件：${confirmRestore?.path ?? ""}\n\n确定继续吗？`}
  confirmText="覆盖并恢复"
  danger
  onConfirm={() => void doRestoreExecute()}
  onCancel={() => (confirmRestore = null)}
/>

<style>
  .settings-dialog {
    width: 480px;
  }
  .set-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 10px 0;
  }
  .set-name {
    font-size: 13px;
    font-weight: 700;
  }
  .set-desc {
    font-size: 11px;
    color: var(--text-3);
    margin-top: 3px;
  }
  .set-ctrl {
    display: flex;
    gap: 8px;
    align-items: center;
    flex: none;
  }
  .shortcut-input {
    width: 170px;
    text-align: center;
  }
</style>
