<script setup lang="ts">
import { apply, busy, hide, quit, save } from "../composables/useApp";
</script>

<template>
  <footer class="flex gap-2 mt-auto pt-1">
    <button class="btn-ghost" @click="apply(true)">立即深色</button>
    <button class="btn-ghost" @click="apply(false)">立即浅色</button>
    <span class="flex-1" />
    <button
      class="btn-ghost"
      title="关闭窗口只是隐藏到托盘；需要彻底结束进程请用这个"
      @click="quit"
    >
      退出程序
    </button>
    <button class="btn-ghost" @click="hide">隐藏到托盘</button>
    <button class="btn-primary" :disabled="busy" @click="save">保存</button>
  </footer>
</template>
