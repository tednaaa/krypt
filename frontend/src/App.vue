<script setup lang="ts">
import { useColorMode } from '@vueuse/core';
import { RouterView } from 'vue-router';

useColorMode({ initialValue: 'auto' });
</script>

<template>
  <RouterView />
</template>
