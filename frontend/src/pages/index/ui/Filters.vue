<script setup lang='ts'>
import { StarIcon } from 'lucide-vue-next';
import { computed } from 'vue';
import { useGetPairs } from '@/shared/api/pairs';
import { Button } from '@/shared/ui/button';

const { isShowingFavorites } = useGetPairs();

const fill = computed(() => isShowingFavorites.value ? 'currentColor' : 'none');
</script>

<template>
  <Button size="sm" @click="isShowingFavorites = !isShowingFavorites">
    <StarIcon class="size-5" :fill />
    Show Favorites
  </Button>
</template>
