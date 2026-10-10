import React, { useId } from "react";
import { CaretDown, WarningCircle } from "@phosphor-icons/react";
import styles from "./Select.module.css";

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps
  extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  label?: string;
  options?: SelectOption[];
  hint?: string;
  error?: string | boolean;
}

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  (
    {
      id: customId,
      label,
      options,
      hint,
      error,
      children,
      className,
      disabled,
      ...props
    },
    ref,
  ) => {
    const generatedId = useId();
    const selectId = customId ?? generatedId;
    const hintId = `${selectId}-hint`;
    const errorId = `${selectId}-error`;

    const describedBy = [
      typeof error === "string" && error ? errorId : undefined,
      hint ? hintId : undefined,
    ]
      .filter(Boolean)
      .join(" ");

    return (
      <div
        className={[
          styles.container,
          error ? styles.hasError : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        {label && (
          <label htmlFor={selectId} className={styles.label}>
            {label}
          </label>
        )}
        <div className={styles.selectWrap}>
          <select
            ref={ref}
            id={selectId}
            disabled={disabled}
            aria-invalid={Boolean(error)}
            aria-describedby={describedBy || undefined}
            className={styles.select}
            {...props}
          >
            {options
              ? options.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))
              : children}
          </select>
          <span className={styles.chevron} aria-hidden="true">
            <CaretDown size={14} />
          </span>
        </div>
        {typeof error === "string" && error && (
          <span id={errorId} role="alert" className={styles.errorMsg}>
            <WarningCircle size={14} />
            <span>{error}</span>
          </span>
        )}
        {hint && (
          <span id={hintId} className={styles.hint}>
            {hint}
          </span>
        )}
      </div>
    );
  },
);

Select.displayName = "Select";
