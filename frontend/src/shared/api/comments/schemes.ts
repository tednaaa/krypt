import z from 'zod';

export const AddCommentPayloadSchema = z.object({
  comment: z.string(),
});
export type AddCommentPayload = z.infer<typeof AddCommentPayloadSchema>;

export const RemoveCommentPayloadSchema = z.object({
  comment: z.string(),
});
export type RemoveCommentPayload = z.infer<typeof RemoveCommentPayloadSchema>;
