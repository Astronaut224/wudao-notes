<script lang="ts">
  /** 羽毛笔图标（基于 Lucide feather 造型，MIT），可着色 */
  interface Props {
    size?: number;
    color?: string;
  }
  let { size = 16, color = "currentColor" }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke={color}
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d="M20.24 12.24a6 6 0 0 0-8.49-8.49L5 10.5V19h8.5z" />
  <path d="M16 8 2 22" />
  <path d="M17.5 15H9" />
</svg>
