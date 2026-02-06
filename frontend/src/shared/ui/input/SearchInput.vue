<script setup lang='ts'>
import { SearchIcon } from 'lucide-vue-next';
import { InputGroup, InputGroupAddon, InputGroupInput } from '../input-group';

const props = defineProps<{
  label: string;
}>();

const value = defineModel<string>({ required: true });
</script>

<template>
  <InputGroup>
    <InputGroupInput v-model="value" placeholder="Search..." />
    <InputGroupAddon>
      <SearchIcon class="size-5" />
    </InputGroupAddon>
    <InputGroupAddon align="inline-end">
      {{ props.label }}
    </InputGroupAddon>
  </InputGroup>
</template>
