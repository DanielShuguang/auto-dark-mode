<script setup lang="ts">
import { cfg } from "../composables/useApp";
import SunSchedule from "./SunSchedule.vue";
import TimeSchedule from "./TimeSchedule.vue";
</script>

<template>
  <section v-if="cfg && cfg.mode !== 'off'" class="panel">
    <TimeSchedule v-if="cfg.mode === 'time'" />
    <SunSchedule v-else />

    <label class="check">
      <input type="checkbox" v-model="cfg.system_ui" />
      同时切换任务栏与系统界面
    </label>
  </section>
</template>
