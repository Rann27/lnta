import React, { useId } from "react";
import { WarningCircle } from "@phosphor-icons/react";
import styles from "./TextField.module.css";

export interface TextFieldProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size"> {
  label?: string;
  hint?: string;
  error?: string | boolean;
  leftElement?: React.ReactNode;
  rightElement?: React.ReactNode;
}

export const TextField = React.forwardRef<HTMLInputElement, TextFieldProps>(
  (
    {
      id: customId,
      label,
      hint,
      error,
      leftElement,
      rightElement,
      className,
      disabled,
      ...props
    },
    ref,
  ) => {
    const generatedId = useId();
    const inputId = customId ?? generatedId;
    const hintId = `${inputId}-hint`;
    const errorId = `${inputId}-error`;

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
          leftElement ? styles.hasLeft : "",
          rightElement ? styles.hasRight : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        {label && (
          <label htmlFor={inputId} className={styles.label}>
            {label}
          </label>
        )}
        <div className={styles.inputWrap}>
          {leftElement && <span className={styles.leftSlot}>{leftElement}</span>}
          <input
            ref={ref}
            id={inputId}
            disabled={disabled}
            aria-invalid={Boolean(error)}
            aria-describedby={describedBy || undefined}
            className={styles.input}
            {...props}
          />
          {rightElement && <span className={styles.rightSlot}>{rightElement}</span>}
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

TextField.displayName = "TextField";
