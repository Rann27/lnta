import React from "react";
import styles from "./IconButton.module.css";

export interface IconButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "aria-label"> {
  "aria-label": string;
  icon: React.ReactNode;
  variant?: "ghost" | "secondary" | "primary" | "danger";
  size?: "sm" | "md";
  disabledReason?: string;
}

export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  (
    {
      "aria-label": ariaLabel,
      icon,
      variant = "ghost",
      size = "md",
      disabled,
      disabledReason,
      className,
      ...props
    },
    ref,
  ) => {
    const isActuallyDisabled = Boolean(disabled || disabledReason);

    return (
      <button
        ref={ref}
        type="button"
        aria-label={ariaLabel}
        title={disabledReason || ariaLabel}
        disabled={isActuallyDisabled}
        aria-disabled={isActuallyDisabled}
        className={[
          styles.iconButton,
          styles[variant],
          size === "sm" ? styles.sm : "",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
        {...props}
      >
        <span aria-hidden="true">{icon}</span>
      </button>
    );
  },
);

IconButton.displayName = "IconButton";
