<script lang="ts">
  import { toastState } from "../stores/toast.svelte";
</script>

<div class="toast-wrap" aria-live="polite">
  {#each toastState.items as t (t.id)}
    <div class="toast {t.kind}">{t.text}</div>
  {/each}
</div>
