<script setup lang='ts'>
const props = defineProps<{
  token: string;
  imgSrc: string;
}>();
</script>

<template>
  <div class="flex items-center gap-2">
    <img :src="props.imgSrc" class="size-5">
    <span>{{ props.token }}</span>
  </div>
</template>
