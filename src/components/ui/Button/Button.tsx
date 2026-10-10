import React, { useId } from "react";
import styles from "./Button.module.css";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
  icon?: React.ReactNode;
  disabledReason?: string;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = "secondary",
      size = "md",
      icon,
      disabled,
      disabledReason,
      children,
      className,
      ...props
    },
    ref,
  ) => {
    const hintId = useId();
    const isActuallyDisabled = Boolean(disabled || disabledReason);

    const buttonElement = (
      <button
        ref={ref}
        type="button"
        disabled={isActuallyDisabled}
        aria-disabled={isActuallyDisabled}
        aria-describedby={disabledReason ? hintId : props["aria-describedby"]}
        className={[
          styles.button,
          styles[variant],
          size === "sm" ? styles.sm : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
        {...props}
      >
        {icon && <span aria-hidden="true">{icon}</span>}
        <span>{children}</span>
      </button>
    );

    if (disabledReason) {
      return (
        <span className={styles.buttonWrapper}>
          {buttonElement}
          <span id={hintId} className={styles.disabledHint} role="status">
            {disabledReason}
          </span>
        </span>
      );
    }

    return buttonElement;
  },
);

Button.displayName = "Button";
