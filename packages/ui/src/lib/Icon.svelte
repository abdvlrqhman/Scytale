<script lang="ts" module>
  // Every shape is a path so the icons render in the SVG namespace without special handling.
  const rect = (x: number, y: number, w: number, h: number, r: number) =>
    `M${x + r} ${y}h${w - 2 * r}a${r} ${r} 0 0 1 ${r} ${r}v${h - 2 * r}a${r} ${r} 0 0 1 -${r} ${r}` +
    `h-${w - 2 * r}a${r} ${r} 0 0 1 -${r} -${r}v-${h - 2 * r}a${r} ${r} 0 0 1 ${r} -${r}z`;
  const circle = (cx: number, cy: number, r: number) =>
    `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 -${2 * r} 0`;
  const dot = (cx: number, cy: number) => circle(cx, cy, 0.6);

  const icons = {
    rod: [rect(9, 2, 6, 20, 3), 'M6 7l12-3M6 12l12-3M6 17l12-3'],
    lock: [rect(5, 11, 14, 10, 2), 'M8 11V7a4 4 0 0 1 8 0v4'],
    unlock: [rect(5, 11, 14, 10, 2), 'M8 11V7a4 4 0 0 1 7.5-1.9'],
    search: [circle(11, 11, 7), 'm20 20-3.5-3.5'],
    copy: [rect(9, 9, 11, 11, 2), 'M5 15V6a2 2 0 0 1 2-2h8'],
    eye: ['M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z', circle(12, 12, 3)],
    eyeOff: ['M3 3l18 18', 'M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-2.6 3.4M6.6 6.6C3.7 8.4 2 12 2 12s3.5 7 10 7a9.6 9.6 0 0 0 4.4-1', 'M9.9 9.9a3 3 0 0 0 4.2 4.2'],
    plus: ['M12 5v14M5 12h14'],
    settings: ['M4 7h9M17 7h3M4 17h3M11 17h9', circle(15, 7, 2), circle(9, 17, 2)],
    dice: [rect(4, 4, 16, 16, 3), dot(9, 9), dot(15, 15), dot(15, 9), dot(9, 15)],
    cloud: ['M7 18a4 4 0 0 1-.5-7.97A6 6 0 0 1 18 9a4.5 4.5 0 0 1-.5 9z'],
    external: ['M14 4h6v6M20 4l-9 9', 'M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5'],
    star: ['m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z'],
    card: [rect(3, 6, 18, 13, 2), 'M3 10h18'],
    note: ['M6 3h9l4 4v14H6z', 'M9 12h7M9 16h5'],
    identity: [rect(3, 5, 18, 14, 2), circle(9, 12, 2.5), 'M14 10h4M14 14h3'],
    grid: [rect(4, 4, 7, 7, 1.5), rect(13, 4, 7, 7, 1.5), rect(4, 13, 7, 7, 1.5), rect(13, 13, 7, 7, 1.5)],
    key: [circle(8, 15, 4), 'm11 12 9-9M17 6l3 3M15 8l2 2'],
    clock: [circle(12, 12, 9), 'M12 7v5l3 2'],
    laptop: [rect(4, 5, 16, 11, 1.5), 'M2 19h20'],
    check: ['m5 12 5 5 9-10'],
    close: ['M6 6l12 12M18 6 6 18'],
    minimize: ['M6 12h12'],
    maximize: [rect(6, 6, 12, 12, 1.5)],
    restore: [rect(5, 9, 10, 10, 1.5), 'M9 9V6.5A1.5 1.5 0 0 1 10.5 5h7A1.5 1.5 0 0 1 19 6.5v7a1.5 1.5 0 0 1-1.5 1.5H15'],
    back: ['m15 5-7 7 7 7'],
    forward: ['m9 5 7 7-7 7'],
    trash: ['M4 7h16M10 11v6M14 11v6', 'M6 7l1 13h10l1-13M9 7V4h6v3'],
    edit: ['M4 20h4L19 9l-4-4L4 16z', 'm13 7 4 4'],
    shield: ['M12 3 4 6v6c0 4.5 3.4 8 8 9 4.6-1 8-4.5 8-9V6z', 'm9 12 2 2 4-4'],
    download: ['M12 4v11M7 10l5 5 5-5', 'M5 20h14'],
    upload: ['M12 20V9M7 14l5-5 5 5', 'M5 4h14'],
    refresh: ['M20 11a8 8 0 0 0-14.6-4.5L4 8', 'M4 4v4h4', 'M4 13a8 8 0 0 0 14.6 4.5L20 16', 'M20 20v-4h-4'],
    print: ['M7 9V4h10v5', rect(4, 9, 16, 8, 2), 'M7 14h10v6H7z'],
    sun: [circle(12, 12, 4), 'M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4'],
    moon: ['M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z'],
  } satisfies Record<string, string[]>;

  export type IconName = keyof typeof icons;
</script>

<script lang="ts">
  let { name, size = 18, stroke = 1.75 }: { name: IconName; size?: number; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#each icons[name] as d (d)}
    <path {d} />
  {/each}
</svg>

<style>
  svg {
    flex-shrink: 0;
  }
</style>
