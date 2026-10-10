import React from "react";
import styles from "./Switch.module.css";

export interface SwitchProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type" | "role"> {
  label?: React.ReactNode;
}

export const Switch = React.forwardRef<HTMLInputElement, SwitchProps>(
  ({ label, checked, disabled, className, onChange, ...props }, ref) => {
    return (
      <label
        className={[
          styles.container,
          checked ? styles.checked : "",
          disabled ? styles.disabled : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        <input
          ref={ref}
          type="checkbox"
          role="switch"
          aria-checked={Boolean(checked)}
          checked={checked}
          disabled={disabled}
          onChange={onChange}
          className={styles.input}
          {...props}
        />
        <span className={styles.track} aria-hidden="true">
          <span className={styles.knob} />
        </span>
        {label && <span className={styles.label}>{label}</span>}
      </label>
    );
  },
);

Switch.displayName = "Switch";
