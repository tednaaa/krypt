<script setup lang='ts'>
const props = defineProps<{
  href: string;
  imgSrc: string;
  label: string;
}>();
</script>

<template>
  <a
    :href="props.href"
    target="_blank"
    rel="noopener noreferrer"
    class="flex items-center gap-2 text-blue-300 hover:text-blue-500"
  >
    <img :src="props.imgSrc" class="size-5">
    {{ props.label }}
  </a>
</template>
