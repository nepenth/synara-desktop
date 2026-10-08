/**
 * Relationships between Core's generated wire types and the shapes renderer
 * parsers return.
 *
 * Rust serializes an absent `Option<T>` as `null` unless it skips the field.
 * Renderer parsers validate untrusted IPC and normalize those `null`s to
 * absent fields. Deriving the parsed shape from the generated wire type keeps
 * Rust the single definition of the fields.
 */
type Simplify<T> = { [K in keyof T]: T[K] } & {};

/** `T` with every `X | null` field made optional `X`. */
export type NullsToOptional<T> = Simplify<
  { [K in keyof T as null extends T[K] ? never : K]: T[K] } & {
    [K in keyof T as null extends T[K] ? K : never]?: Exclude<T[K], null>;
  }
>;
