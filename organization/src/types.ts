import z from 'zod';

export function parse<S extends z.ZodType<any, any, any>>(content: string, schema: S): z.infer<S> {
  return z
    .string()
    .transform((c, ctx) => {
      try {
        return JSON.parse(c);
      } catch (error) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: error instanceof Error ? error.message : 'Invalid JSON',
        });

        return z.never;
      }
    })
    .pipe(schema)
    .parse(content);
}

export function safeParse<S extends z.ZodType<any, any, any>>(
  content: string,
  schema: S,
): z.SafeParseReturnType<string, z.infer<S>> {
  return z
    .string()
    .transform((c, ctx) => {
      try {
        return JSON.parse(c);
      } catch (error) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: error instanceof Error ? error.message : 'Invalid JSON',
        });

        return z.never;
      }
    })
    .pipe(schema)
    .safeParse(content);
}

export const organizationIdentifiersSchema = z.object({
  root: z.string(),
  organization: z.string(),
  organizationalUnits: z.object({
    operations: z.string(),
    applications: z.string(),
  }),
  accounts: z.object({
    management: z.string(),
    cdkBootstrap: z.string(),
    musicStorage: z.string(),
  }),
});

export type OrganizationIdentifiers = z.infer<typeof organizationIdentifiersSchema>;
