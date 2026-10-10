import React, { useEffect, useRef } from "react";
import { Check, Minus } from "@phosphor-icons/react";
import styles from "./Checkbox.module.css";

export interface CheckboxProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type"> {
  label?: React.ReactNode;
  indeterminate?: boolean;
}

export const Checkbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, checked, indeterminate, disabled, className, onChange, ...props }, ref) => {
    const innerRef = useRef<HTMLInputElement>(null);

    // Merge forwarded ref with innerRef for indeterminate handling
    useEffect(() => {
      const el = (ref && "current" in ref ? ref.current : innerRef.current);
      if (el) {
        el.indeterminate = Boolean(indeterminate);
      }
    }, [indeterminate, ref]);

    return (
      <label
        className={[
          styles.container,
          checked ? styles.checked : "",
          indeterminate ? styles.indeterminate : "",
          disabled ? styles.disabled : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        <input
          ref={(node) => {
            innerRef.current = node;
            if (typeof ref === "function") {
              ref(node);
            } else if (ref) {
              ref.current = node;
            }
          }}
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={onChange}
          className={styles.input}
          {...props}
        />
        <span className={styles.box} aria-hidden="true">
          {checked && !indeterminate && (
            <span className={styles.icon}>
              <Check size={10} weight="bold" />
            </span>
          )}
          {indeterminate && (
            <span className={styles.icon}>
              <Minus size={10} weight="bold" />
            </span>
          )}
        </span>
        {label && <span className={styles.label}>{label}</span>}
      </label>
    );
  },
);

Checkbox.displayName = "Checkbox";
