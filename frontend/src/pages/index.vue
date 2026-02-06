<script setup lang='ts'>
import MainLayout from '@/layouts/MainLayout.vue';
import { useGetPairs } from '@/shared/api/pairs';
import { DataTable } from '@/shared/ui/data-table';
import { columns } from './index/columns';
import Filters from './index/ui/Filters.vue';

const { foundPairs, sorting } = useGetPairs();
</script>

<template>
  <MainLayout>
    <div class="my-8">
      <div class="flex items-center justify-between gap-4 mb-6">
        <h1 class="text-xl font-bold">Scanner</h1>

        <Filters />
      </div>

      <DataTable v-model:sorting="sorting" :columns :data="foundPairs" />
    </div>
  </MainLayout>
</template>
