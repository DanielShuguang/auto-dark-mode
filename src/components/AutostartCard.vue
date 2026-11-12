<script setup lang="ts">
import { busy, cfg, enableAdmin } from "../composables/useApp";
</script>

<template>
  <section v-if="cfg" class="panel">
    <label class="check">
      <input type="checkbox" v-model="cfg.autostart" />
      开机自动启动
    </label>

    <label v-if="cfg.autostart" class="check">
      <input type="checkbox" v-model="cfg.autostart_admin" />
      以管理员权限启动（计划任务，开机不弹 UAC）
    </label>

    <p v-if="cfg.autostart && cfg.autostart_admin" class="hint">
      首次需要一次管理员授权；之后每次开机由计划任务静默提权，不再弹窗。
      <button class="btn-ghost ml-1" :disabled="busy" @click="enableAdmin">授权并启用</button>
    </p>
    <p v-else-if="cfg.autostart" class="hint">
      普通启动项，不提权。若开机时系统仍询问是否允许运行，请改用上面的管理员模式。
    </p>
  </section>
</template>
