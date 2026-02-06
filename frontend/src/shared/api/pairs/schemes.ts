import z from 'zod';

export const PairSchema = z.object({
  icon: z.string(),
  pair: z.string(),
  mfi_1h: z.float64(),
  mfi_4h: z.float64(),
  mfi_1d: z.float64(),
  mfi_1w: z.float64(),
  is_favorite: z.boolean(),
  comments: z.array(z.string()),
});
export type Pair = z.infer<typeof PairSchema>;

export const PairsResponseSchema = z.array(PairSchema);
export type PairsResponse = z.infer<typeof PairsResponseSchema>;

export interface PairQueryParams {
  sort?: string;
  favorite?: boolean;
}
