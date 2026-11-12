<script setup lang="ts">
import { autostartLabel, status } from "../composables/useApp";
</script>

<template>
  <section class="panel">
    <div class="flex gap-3 text-[13px]">
      <span class="stat-key">当前</span>
      <span class="font-medium">
        {{ status?.dark === null || status?.dark === undefined ? "未知" : status.dark ? "深色" : "浅色" }}
        <span v-if="status && status.system_dark !== null" class="text-muted font-normal">
          / 任务栏 {{ status.system_dark ? "深色" : "浅色" }}
        </span>
      </span>
    </div>

    <div class="flex gap-3 text-[13px]">
      <span class="stat-key">下次切换</span>
      <span class="font-medium">{{ status?.next ?? "无" }}</span>
    </div>

    <div class="flex gap-3 text-[13px]">
      <span class="stat-key">开机自启</span>
      <span class="font-medium">{{ autostartLabel }}</span>
    </div>

    <p v-if="status?.integrity === 4096" class="warn">
      当前进程以「低完整性」（Low IL）运行，Windows 会拒绝一切主题写入，
      因此切换必然失败。这通常是被沙箱或受限终端启动导致的：
      请在文件资源管理器里双击 auto-dark-mode.exe，或从开始菜单新开终端启动。
    </p>

    <p v-if="status?.high_contrast" class="warn">
      系统处于高对比度模式，自动切换已暂停（避免破坏无障碍设置）。
    </p>
  </section>
</template>
