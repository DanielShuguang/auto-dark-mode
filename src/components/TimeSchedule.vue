<script setup lang="ts">
import { cfg } from "../composables/useApp";
</script>

<template>
  <div v-if="cfg" class="flex flex-col gap-3">
    <div class="row">
      <label class="lbl">变暗时间</label>
      <input type="time" class="field w-[108px]" v-model="cfg.dark_at" />
      <label class="lbl">变亮时间</label>
      <input type="time" class="field w-[108px]" v-model="cfg.light_at" />
    </div>

    <label class="check">
      <input type="checkbox" v-model="cfg.weekend_enabled" />
      周末使用不同时间
    </label>

    <div v-if="cfg.weekend_enabled" class="row">
      <label class="lbl">周末变暗</label>
      <input type="time" class="field w-[108px]" v-model="cfg.weekend_dark_at" />
      <label class="lbl">周末变亮</label>
      <input type="time" class="field w-[108px]" v-model="cfg.weekend_light_at" />
    </div>
  </div>
</template>
