<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue"

import type { Pair } from "@/shared/api/pairs"
import { fetchPairs } from "@/shared/api/pairs"
import { Badge } from "@/shared/ui/badge"
import { Button } from "@/shared/ui/button"
import { Checkbox } from "@/shared/ui/checkbox"
import { Input } from "@/shared/ui/input"
import { Label } from "@/shared/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableEmpty,
  TableHead,
  TableHeader,
  TableRow,
} from "@/shared/ui/table"

const pairs = ref<Pair[]>([])
const isLoading = ref(false)
const errorMessage = ref<string | null>(null)

const filters = reactive({
  search: "",
  favoritesOnly: false,
  commentsOnly: false,
  sort: "mfi_1h:desc,mfi_4h:desc,mfi_1d:desc,mfi_1w:desc",
})

const sortOptions = [
  { label: "MFI 1h ↓", value: "mfi_1h:desc" },
  { label: "MFI 1h ↑", value: "mfi_1h:asc" },
  { label: "MFI 4h ↓", value: "mfi_4h:desc" },
  { label: "MFI 4h ↑", value: "mfi_4h:asc" },
  { label: "MFI 1d ↓", value: "mfi_1d:desc" },
  { label: "MFI 1d ↑", value: "mfi_1d:asc" },
  { label: "MFI 1w ↓", value: "mfi_1w:desc" },
  { label: "MFI 1w ↑", value: "mfi_1w:asc" },
  { label: "Multi (1h, 4h, 1d, 1w) ↓", value: "mfi_1h:desc,mfi_4h:desc,mfi_1d:desc,mfi_1w:desc" },
  { label: "Multi (1h, 4h, 1d, 1w) ↑", value: "mfi_1h:asc,mfi_4h:asc,mfi_1d:asc,mfi_1w:asc" },
]

const filteredPairs = computed(() => {
  const search = filters.search.trim().toUpperCase()
  return pairs.value.filter((pair) => {
    if (filters.favoritesOnly && !pair.is_favorite) {
      return false
    }
    if (filters.commentsOnly && pair.comments.length === 0) {
      return false
    }
    if (search && !pair.pair.toUpperCase().includes(search)) {
      return false
    }
    return true
  })
})

const lastUpdatedLabel = computed(() => {
  if (!pairs.value.length) {
    return "No data yet"
  }
  const timestamp = new Intl.DateTimeFormat("en", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date())
  return `Last refresh ${timestamp}`
})

async function loadPairs() {
  isLoading.value = true
  errorMessage.value = null
  try {
    const data = await fetchPairs({
      search: filters.search.trim() || undefined,
      favorites: filters.favoritesOnly || undefined,
      hasComments: filters.commentsOnly || undefined,
      sort: filters.sort || undefined,
    })
    pairs.value = data
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to load pairs"
    errorMessage.value = message
  } finally {
    isLoading.value = false
  }
}

function resetFilters() {
  filters.search = ""
  filters.favoritesOnly = false
  filters.commentsOnly = false
  filters.sort = "mfi_1h:desc,mfi_4h:desc,mfi_1d:desc,mfi_1w:desc"
  void loadPairs()
}

watch(
  () => filters.sort,
  () => {
    void loadPairs()
  },
)

onMounted(() => {
  void loadPairs()
})
</script>

<template>
  <main class="min-h-screen bg-background text-foreground">
    <section class="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-10">
      <header class="flex flex-col gap-4">
        <div class="flex flex-wrap items-center justify-between gap-4">
          <div>
            <p class="text-sm text-muted-foreground">Scanner</p>
            <h1 class="text-2xl font-semibold">USDT Pairs</h1>
          </div>
          <Badge variant="secondary">{{ lastUpdatedLabel }}</Badge>
        </div>
        <div class="grid gap-4 md:grid-cols-[minmax(0,1fr)_auto_auto_auto]">
          <div class="flex items-center gap-3 rounded-lg border bg-card px-4 py-3 shadow-sm">
            <Input
              v-model="filters.search"
              placeholder="Search by symbol (e.g. ETHUSDT)"
            />
            <Button variant="secondary" size="sm" @click="loadPairs">Search</Button>
          </div>
          <div class="flex items-center gap-2 rounded-lg border bg-card px-4 py-3 shadow-sm">
            <Checkbox id="favorites-only" v-model:checked="filters.favoritesOnly" />
            <Label for="favorites-only">Favorites</Label>
          </div>
          <div class="flex items-center gap-2 rounded-lg border bg-card px-4 py-3 shadow-sm">
            <Checkbox id="comments-only" v-model:checked="filters.commentsOnly" />
            <Label for="comments-only">Has comments</Label>
          </div>
          <div class="flex items-center gap-2 rounded-lg border bg-card px-4 py-3 shadow-sm">
            <Select v-model="filters.sort">
              <SelectTrigger class="w-[220px]">
                <SelectValue placeholder="Sort by" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in sortOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <Button @click="loadPairs" :disabled="isLoading">Refresh</Button>
          <Button variant="secondary" @click="resetFilters" :disabled="isLoading">
            Reset
          </Button>
          <p v-if="errorMessage" class="text-sm text-destructive">{{ errorMessage }}</p>
        </div>
      </header>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Pair</TableHead>
            <TableHead>Favorite</TableHead>
            <TableHead>Comments</TableHead>
            <TableHead>MFI 1h</TableHead>
            <TableHead>MFI 4h</TableHead>
            <TableHead>MFI 1d</TableHead>
            <TableHead>MFI 1w</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableEmpty v-if="!filteredPairs.length && !isLoading" :colspan="7">
            No pairs yet. Try refresh.
          </TableEmpty>
          <TableRow v-for="pair in filteredPairs" :key="pair.pair">
            <TableCell>
              <div class="flex items-center gap-3">
                <!-- <img :src="pair.icon" :alt="pair.pair" class="size-6 rounded-full" /> -->
                <span class="font-medium">{{ pair.pair }}</span>
                <a class="text-blue-500 font-semibold" :href="`https://www.tradingview.com/chart?symbol=Binance:${pair.pair}.P`" target="_blank">TV</a>
              </div>
            </TableCell>
            <TableCell>
              <Badge :variant="pair.is_favorite ? 'default' : 'secondary'">
                {{ pair.is_favorite ? "Yes" : "No" }}
              </Badge>
            </TableCell>
            <TableCell>
              <div class="flex flex-wrap gap-2">
                <Badge v-if="!pair.comments.length" variant="secondary">None</Badge>
                <Badge
                  v-for="(comment, index) in pair.comments"
                  :key="`${pair.pair}-${index}`"
                  variant="outline"
                >
                  {{ comment }}
                </Badge>
              </div>
            </TableCell>
            <TableCell>{{ pair.mfi_1h.toFixed(2) }}</TableCell>
            <TableCell>{{ pair.mfi_4h.toFixed(2) }}</TableCell>
            <TableCell>{{ pair.mfi_1d.toFixed(2) }}</TableCell>
            <TableCell>{{ pair.mfi_1w.toFixed(2) }}</TableCell>
          </TableRow>
          <TableRow v-if="isLoading">
            <TableCell :colspan="7">Loading pairs…</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </section>
  </main>
</template>
