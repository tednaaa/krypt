<script setup lang='ts'>
import MainLayout from '@/layouts/MainLayout.vue';
import { DataTable } from '@/shared/ui/data-table';
import { columns } from './index/columns';
import { data } from './index/data';
</script>

<template>
  <MainLayout>
    <div class="my-8">
      <h1 class="text-xl font-bold mb-6">Scanner</h1>

      <DataTable :columns :data="data" />
    </div>
  </MainLayout>
</template>
