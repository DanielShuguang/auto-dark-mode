<script setup lang="ts">
import { cfg, message, useAppLifecycle } from "./composables/useApp";
import AppFooter from "./components/AppFooter.vue";
import AppHeader from "./components/AppHeader.vue";
import AutostartCard from "./components/AutostartCard.vue";
import ModePicker from "./components/ModePicker.vue";
import ScheduleCard from "./components/ScheduleCard.vue";
import StatusCard from "./components/StatusCard.vue";

useAppLifecycle();
</script>

<template>
  <main
    v-if="cfg"
    class="flex flex-col gap-3 h-full overflow-y-auto px-5 pt-[18px] pb-4"
  >
    <AppHeader />
    <ModePicker />
    <ScheduleCard />
    <AutostartCard />
    <StatusCard />

    <p
      class="m-0 min-h-[18px] text-xs text-accent transition-opacity duration-150"
      :class="message ? 'opacity-100' : 'opacity-0'"
    >
      {{ message }}
    </p>

    <AppFooter />
  </main>

  <main v-else class="flex flex-col gap-3 h-full items-center justify-center text-muted">
    正在读取配置…
  </main>
</template>
