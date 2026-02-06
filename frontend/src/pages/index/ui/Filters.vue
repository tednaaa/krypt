<script setup lang='ts'>
import { StarIcon } from 'lucide-vue-next';
import { computed } from 'vue';
import { useGetPairs } from '@/shared/api/pairs';
import { Button } from '@/shared/ui/button';
import { SearchInput } from '@/shared/ui/input';

const { isShowingFavorites, searchQuery, foundPairs } = useGetPairs();

const foundPairsLabel = computed(() => `${foundPairs.value.length} pairs`);

const fill = computed(() => isShowingFavorites.value ? 'currentColor' : 'none');
</script>

<template>
  <div class="flex items-center gap-4">
    <SearchInput v-model="searchQuery" :label="foundPairsLabel" />

    <Button size="sm" @click="isShowingFavorites = !isShowingFavorites">
      <StarIcon class="size-5" :fill />
      Show Favorites
    </Button>
  </div>
</template>
