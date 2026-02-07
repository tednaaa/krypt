<script setup lang='ts'>
import { computed } from 'vue';

const props = defineProps<{
  mfi: number;
}>();

const circleColor = computed(() => {
  if (props.mfi < 20)
    return 'bg-green-500';
  if (props.mfi < 50)
    return 'bg-yellow-500';
  return 'bg-red-500';
});
</script>

<template>
  <div class="flex items-center gap-2">
    <span>{{ props.mfi }}</span>
    <div class="size-4 rounded-full" :class="circleColor" />
  </div>
</template>
