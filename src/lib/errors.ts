import i18n from "./i18n";

/** Readable message for an IPC failure: `AppError` values carry an i18n code and params. */
export function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "code" in err) {
    const e = err as { code: string; params?: Record<string, unknown> };
    return i18n.t(e.code, { ...(e.params ?? {}), defaultValue: e.code });
  }
  return String(err);
}
