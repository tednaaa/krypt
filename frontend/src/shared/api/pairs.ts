import { z } from "zod"

import { http } from "./http"

export const pairSchema = z.object({
  icon: z.string().url(),
  pair: z.string(),
  mfi_1h: z.number(),
  mfi_4h: z.number(),
  mfi_1d: z.number(),
  mfi_1w: z.number(),
  is_favorite: z.boolean(),
  comments: z.array(z.string()),
})

export type Pair = z.infer<typeof pairSchema>

const pairsResponseSchema = z.array(pairSchema)

export type PairsQuery = {
  search?: string
  favorites?: boolean
  hasComments?: boolean
  sort?: string
}

export async function fetchPairs(query: PairsQuery) {
  const response = await http.get("/pairs", { params: query })
  return pairsResponseSchema.parse(response.data)
}
