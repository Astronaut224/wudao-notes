<script lang="ts">
  /**
   * 胶囊悬浮形态：
   * - 展开态：羽毛笔图标 + 未完成数 + 收起钮；单击胶囊打开简洁窗；按住拖动可移动
   * - 收起态：贴屏幕左/右缘的细条，鼠标划过滑出为胶囊（锚定态，鼠标离开自动回收）
   * - 拖到屏幕边缘松手自动吸附；拖离边缘自动弹出
   */
  import { onMount } from "svelte";
  import FeatherIcon from "../../components/FeatherIcon.svelte";
  import { api } from "../../lib/api";
  import { appState, switchMode } from "../../stores/app.svelte";
  import { loadTodos, todoStore } from "../../stores/todos.svelte";
  import { toastError } from "../../stores/toast.svelte";

  /** 边缘吸附阈值（物理像素） */
  const EDGE_SNAP = 48;
  /** 锚定态下鼠标离开后延迟回收，避免误触 */
  const REDOCK_DELAY = 450;

  let dragging = $state(false);
  /** 从收起态滑出的锚定侧（拖动或手动收起后清除） */
  let anchoredSide = $state<"" | "left" | "right">("");

  let downPos = { x: 0, y: 0 };
  let grabOffset = { x: 0, y: 0 };
  let dragArmed = false;
  let dragMoved = false;
  let redockTimer: ReturnType<typeof setTimeout> | null = null;
  let lastClickAt = 0;
  let singleClickTimer: ReturnType<typeof setTimeout> | null = null;

  const activeCount = $derived(todoStore.items.filter((t) => t.status === "active").length);
  const docked = $derived(appState.capsuleDocked !== "");

  onMount(() => {
    void loadTodos();
    return () => {
      if (redockTimer) clearTimeout(redockTimer);
      if (singleClickTimer) clearTimeout(singleClickTimer);
    };
  });

  function dpr(): number {
    return window.devicePixelRatio || 1;
  }

  // ---------- 展开 ⇄ 收起 ----------
  async function dock(side?: string, onlyIfOutside = false) {
    const target = side ?? anchoredSide ?? appState.capsuleDocked ?? "right";
    anchoredSide = "";
    try {
      await api.setCapsuleDocked(true, target, onlyIfOutside);
    } catch (e) {
      toastError(e, "收起失败");
    }
  }

  async function expandFromDock() {
    try {
      await api.setCapsuleDocked(false, appState.capsuleDocked || "right");
    } catch (e) {
      toastError(e, "展开失败");
    }
  }

  function cancelRedock() {
    if (redockTimer) {
      clearTimeout(redockTimer);
      redockTimer = null;
    }
  }

  function onBarEnter() {
    cancelRedock();
    anchoredSide = (appState.capsuleDocked || "right") as "left" | "right";
    void expandFromDock();
  }

  function onPillLeave(e: MouseEvent) {
    // relatedTarget 为空 = 鼠标离开了整个窗口（胶囊周围有透明边距，元素间移动不算离开）
    if (e.relatedTarget !== null) return;
    // 仅“从收起态划出”的锚定胶囊在鼠标离开后回收；自由拖放的胶囊保持原位
    if (anchoredSide && !dragging) {
      cancelRedock();
      redockTimer = setTimeout(() => {
        redockTimer = null;
        void dock(anchoredSide, true);
      }, REDOCK_DELAY);
    }
  }

  // ---------- 自定义拖拽（JS 驱动，精确控制吸附时机）----------
  function onBodyMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    cancelRedock();
    downPos = { x: e.screenX, y: e.screenY };
    dragArmed = true;
    dragMoved = false;
    void (async () => {
      try {
        const [wx, wy] = await api.getWindowPosition();
        grabOffset = {
          x: Math.round(e.screenX * dpr()) - wx,
          y: Math.round(e.screenY * dpr()) - wy,
        };
      } catch {
        grabOffset = { x: 0, y: 0 };
      }
    })();
    window.addEventListener("mousemove", onBodyMouseMove);
    window.addEventListener("mouseup", onBodyMouseUp, { once: true });
  }

  async function onBodyMouseMove(e: MouseEvent) {
    if (!dragArmed) return;
    if (!dragMoved && Math.hypot(e.screenX - downPos.x, e.screenY - downPos.y) < 6) return;
    if (!dragMoved) {
      dragMoved = true;
      dragging = true;
      // 从收起态直接拖动 → 先弹出为完整胶囊，并重取窗口原点
      if (docked) {
        anchoredSide = "";
        await expandFromDock();
        try {
          const [wx, wy] = await api.getWindowPosition();
          grabOffset = {
            x: Math.round(e.screenX * dpr()) - wx,
            y: Math.round(e.screenY * dpr()) - wy,
          };
        } catch {
          /* 保持原值 */
        }
      }
    }
    const nx = Math.round(e.screenX * dpr()) - grabOffset.x;
    const ny = Math.round(e.screenY * dpr()) - grabOffset.y;
    try {
      await api.moveWindowTo(nx, ny);
    } catch {
      /* 移动失败忽略，下一帧重试 */
    }
  }

  async function onBodyMouseUp() {
    dragArmed = false;
    window.removeEventListener("mousemove", onBodyMouseMove);
    const wasDrag = dragMoved;
    dragging = false;
    if (!wasDrag) {
      // 单击（无拖动）：打开简洁窗；双击：直达完整窗口
      anchoredSide = "";
      const now = Date.now();
      if (now - lastClickAt < 350) {
        lastClickAt = 0;
        if (singleClickTimer) {
          clearTimeout(singleClickTimer);
          singleClickTimer = null;
        }
        void openFull();
      } else {
        lastClickAt = now;
        if (singleClickTimer) clearTimeout(singleClickTimer);
        singleClickTimer = setTimeout(() => {
          singleClickTimer = null;
          void openCompact();
        }, 280);
      }
      return;
    }
    // 拖动结束：清除单击判定状态
    lastClickAt = 0;
    if (singleClickTimer) {
      clearTimeout(singleClickTimer);
      singleClickTimer = null;
    }
    // 拖动结束：贴近所在屏幕的左/右缘 → 吸附收起
    try {
      const [wx, , ww] = await api.getWindowRect();
      const [mx, , mw] = await api.getScreenSize();
      if (wx <= mx + EDGE_SNAP) {
        await api.setCapsuleDocked(true, "left");
      } else if (wx + ww >= mx + mw - EDGE_SNAP) {
        await api.setCapsuleDocked(true, "right");
      }
    } catch {
      /* 吸附判定失败保持当前位置 */
    }
  }

  async function openFull() {
    try {
      await switchMode("normal");
    } catch (e) {
      toastError(e, "打开完整窗口失败");
    }
  }

  async function openCompact() {
    try {
      await switchMode("compact");
    } catch (e) {
      toastError(e, "打开简洁窗失败");
    }
  }
</script>

<div class="capsule-root" class:docked onmouseenter={docked ? onBarEnter : null} onmouseleave={onPillLeave}>
  {#if docked}
    <!-- 收起态：贴边细条 -->
    <button
      class="dock-bar"
      class:left={appState.capsuleDocked === "left"}
      title="吾道笔记 · 鼠标划过展开，按住可拖动" aria-label="吾道笔记胶囊收起条"
    ></button>
  {:else}
    <!-- 展开态：胶囊 -->
    <div
      class="pill"
      class:dragging
      onmousedown={onBodyMouseDown}
      role="button"
      tabindex="0"
      title="单击打开简洁窗，双击打开完整窗口；按住拖动，拖到屏幕边缘自动收起"
      onkeydown={(e) => e.key === "Enter" && void openCompact()}
    >
      <span class="pill-icon"><FeatherIcon size={18} color="#fff" /></span>
      <span class="pill-text">待办 · {activeCount} 项</span>
      <button
        class="pill-dock"
        class:flip={anchoredSide === "left"}
        title="收起贴边（也可拖到屏幕边缘吸附）" aria-label="收起贴边"
        onclick={(e) => {
          e.stopPropagation();
          void dock();
        }}
        onmousedown={(e) => e.stopPropagation()}
      >
        ⇥
      </button>
    </div>
  {/if}
</div>

<style>
  .capsule-root {
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    overflow: visible;
    background: transparent;
  }
  .capsule-root.docked:has(.dock-bar.left) {
    justify-content: flex-start;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 190px;
    margin: 0 6px;
    padding: 8px 8px 8px 12px;
    background: linear-gradient(150deg, #5883ff, #3a58d4);
    border-radius: 999px;
    cursor: grab;
    user-select: none;
  }
  .pill.dragging {
    cursor: grabbing;
  }
  .pill-icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.18);
    flex: none;
  }
  .pill-text {
    flex: 1;
    min-width: 0;
    color: #fff;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pill-dock {
    flex: none;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
    font-size: 13px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .pill-dock:hover {
    background: rgba(255, 255, 255, 0.32);
  }
  .pill-dock.flip {
    transform: scaleX(-1);
  }

  .dock-bar {
    width: 10px;
    height: 52px;
    margin: 0 2px;
    border: 0;
    border-radius: 6px;
    background: linear-gradient(180deg, #5883ff, #3a58d4);
    cursor: pointer;
    padding: 0;
  }
  .dock-bar:hover {
    background: linear-gradient(180deg, #6c92ff, #4a67e2);
  }
</style>
