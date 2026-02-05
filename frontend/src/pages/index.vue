<script setup lang='ts'>
import MainLayout from '@/layouts/MainLayout.vue';
import { useGetPairs } from '@/shared/api/pairs';
import { DataTable } from '@/shared/ui/data-table';
import { columns } from './index/columns';

const { pairs } = useGetPairs();
</script>

<template>
  <MainLayout>
    <div class="my-8">
      <h1 class="text-xl font-bold mb-6">Scanner</h1>

      <DataTable :columns :data="pairs.data ?? []" />
    </div>
  </MainLayout>
</template>
