<script setup lang='ts'>
import { StarIcon } from 'lucide-vue-next';
import { computed } from 'vue';
import { useMarkFavorite, useUnmarkFavorite } from '@/shared/api/favorites';
import { Button } from '@/shared/ui/button';

const props = defineProps<{
  isFavorite: boolean;
  pair: string;
}>();

const { mutate: markFavorite } = useMarkFavorite();
const { mutate: unmarkFavorite } = useUnmarkFavorite();

const fill = computed(() => props.isFavorite ? 'currentColor' : 'none');

function handleClick() {
  if (props.isFavorite) {
    unmarkFavorite(props.pair);
    return;
  }

  markFavorite(props.pair);
}
</script>

<template>
  <Button variant="ghost" size="icon" @click="handleClick">
    <StarIcon class="size-5" :fill />
  </Button>
</template>
