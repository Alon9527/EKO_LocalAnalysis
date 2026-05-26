<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  data: { label: string; value: number }[];
  size?: number;
}>();

const size = computed(() => props.size || 200);
const center = computed(() => size.value / 2);
const radius = computed(() => size.value * 0.38);
const count = computed(() => props.data.length);

function polarToXY(angle: number, r: number) {
  const a = (angle - 90) * (Math.PI / 180);
  return {
    x: center.value + r * Math.cos(a),
    y: center.value + r * Math.sin(a),
  };
}

function angleFor(i: number) {
  return (360 / count.value) * i;
}

const levels = [0.2, 0.4, 0.6, 0.8, 1.0];

const gridPaths = computed(() =>
  levels.map((lv) => {
    const r = radius.value * lv;
    const pts = Array.from({ length: count.value }, (_, i) => {
      const p = polarToXY(angleFor(i), r);
      return `${p.x},${p.y}`;
    });
    return `M${pts.join("L")}Z`;
  })
);

const axisPaths = computed(() =>
  Array.from({ length: count.value }, (_, i) => {
    const p = polarToXY(angleFor(i), radius.value);
    return { x1: center.value, y1: center.value, x2: p.x, y2: p.y };
  })
);

const dataPath = computed(() => {
  const pts = props.data.map((d, i) => {
    const r = radius.value * (d.value / 100);
    const p = polarToXY(angleFor(i), r);
    return `${p.x},${p.y}`;
  });
  return `M${pts.join("L")}Z`;
});

const labelPositions = computed(() =>
  props.data.map((d, i) => {
    const p = polarToXY(angleFor(i), radius.value + 18);
    return { ...p, label: d.label, value: d.value };
  })
);

const iconPositions = computed(() =>
  props.data.map((_, i) => {
    const p = polarToXY(angleFor(i), radius.value + 6);
    return p;
  })
);
</script>

<template>
  <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`" class="select-none">
    <defs>
      <linearGradient id="radar-fill" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="rgb(45, 212, 191)" stop-opacity="0.25" />
        <stop offset="100%" stop-color="rgb(59, 130, 246)" stop-opacity="0.15" />
      </linearGradient>
      <linearGradient id="radar-stroke" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="rgb(45, 212, 191)" stop-opacity="0.9" />
        <stop offset="100%" stop-color="rgb(59, 130, 246)" stop-opacity="0.7" />
      </linearGradient>
      <filter id="radar-glow">
        <feGaussianBlur stdDeviation="3" result="blur" />
        <feMerge>
          <feMergeNode in="blur" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
    </defs>

    <!-- Grid -->
    <path
      v-for="(path, i) in gridPaths"
      :key="'grid-' + i"
      :d="path"
      fill="none"
      :stroke="i === gridPaths.length - 1 ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.05)'"
      stroke-width="1"
    />

    <!-- Axes -->
    <line
      v-for="(axis, i) in axisPaths"
      :key="'axis-' + i"
      :x1="axis.x1"
      :y1="axis.y1"
      :x2="axis.x2"
      :y2="axis.y2"
      stroke="rgba(255,255,255,0.06)"
      stroke-width="1"
    />

    <!-- Data area -->
    <path :d="dataPath" fill="url(#radar-fill)" stroke="url(#radar-stroke)" stroke-width="1.5" filter="url(#radar-glow)" />

    <!-- Data points -->
    <circle
      v-for="(d, i) in props.data"
      :key="'dot-' + i"
      :cx="polarToXY(angleFor(i), radius * (d.value / 100)).x"
      :cy="polarToXY(angleFor(i), radius * (d.value / 100)).y"
      r="3"
      fill="rgb(45, 212, 191)"
      stroke="rgba(10, 10, 15, 0.6)"
      stroke-width="1.5"
    />

    <!-- Labels -->
    <text
      v-for="lp in labelPositions"
      :key="'label-' + lp.label"
      :x="lp.x"
      :y="lp.y"
      text-anchor="middle"
      dominant-baseline="central"
      class="text-[9px] fill-white/40 font-medium"
    >{{ lp.label }}</text>
  </svg>
</template>
