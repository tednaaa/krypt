<script setup lang='ts'>
import MainLayout from '@/layouts/MainLayout.vue';
import { useGetPairs } from '@/shared/api/pairs';
import { DataTable } from '@/shared/ui/data-table';
import { columns } from './index/columns';
import Filters from './index/ui/Filters.vue';

const { pairs, sorting } = useGetPairs();
</script>

<template>
  <MainLayout>
    <div class="my-8">
      <div class="flex items-center justify-between gap-4">
        <h1 class="text-xl font-bold mb-6">Scanner</h1>

        <Filters />
      </div>

      <DataTable v-model:sorting="sorting" :columns :data="pairs.data ?? []" />
    </div>
  </MainLayout>
</template>
