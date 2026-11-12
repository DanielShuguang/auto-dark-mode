<script setup lang="ts">
import { cfg } from "../composables/useApp";

const modes = [
  { value: "off", label: "关闭" },
  { value: "time", label: "固定时间" },
  { value: "sun", label: "日出日落" },
] as const;

function pick(mode: (typeof modes)[number]["value"]) {
  if (cfg.value) cfg.value.mode = mode;
}
</script>

<template>
  <section class="panel">
    <div class="flex gap-1 bg-app border border-line rounded-lg p-[3px]">
      <button
        v-for="m in modes"
        :key="m.value"
        class="flex-1 border-0 py-[7px] rounded-md cursor-pointer"
        :class="
          cfg?.mode === m.value
            ? 'bg-card text-accent font-semibold shadow-[0_1px_2px_rgba(0,0,0,0.08)]'
            : 'bg-transparent text-muted hover:text-fg'
        "
        @click="pick(m.value)"
      >
        {{ m.label }}
      </button>
    </div>
  </section>
</template>
