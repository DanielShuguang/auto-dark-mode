<script setup lang="ts">
import { busy, cfg, doGeo } from "../composables/useApp";
</script>

<template>
  <div v-if="cfg" class="flex flex-col gap-3">
    <div class="row">
      <label class="lbl">日出偏移</label>
      <input type="number" class="field w-[62px] text-center" v-model.number="cfg.sunrise_offset" />
      <span class="unit">分钟</span>
      <label class="lbl">日落偏移</label>
      <input type="number" class="field w-[62px] text-center" v-model.number="cfg.sunset_offset" />
      <span class="unit">分钟</span>
    </div>

    <div class="row">
      <label class="lbl">定位</label>
      <select class="field w-[130px]" v-model="cfg.location">
        <option value="auto">自动（IP）</option>
        <option value="manual">手动</option>
      </select>
      <button class="btn-ghost" :disabled="busy" @click="doGeo">获取定位</button>
    </div>

    <div class="row">
      <label class="lbl">纬度</label>
      <input type="number" class="field w-24" step="0.0001" v-model.number="cfg.lat" />
      <label class="lbl">经度</label>
      <input type="number" class="field w-24" step="0.0001" v-model.number="cfg.lon" />
    </div>

    <p class="hint">定位为城市级精度（误差 ±20~50 km），对日出日落影响小于 2 分钟。</p>
  </div>
</template>
