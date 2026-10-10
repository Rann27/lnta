import React from "react";
import styles from "./TallyLamp.module.css";

export interface TallyLampProps extends React.HTMLAttributes<HTMLSpanElement> {
  "aria-hidden"?: boolean;
}

export const TallyLamp: React.FC<TallyLampProps> = ({
  "aria-hidden": ariaHidden = true,
  className,
  ...props
}) => {
  return (
    <span
      className={[styles.lamp, className ?? ""].filter(Boolean).join(" ")}
      aria-hidden={ariaHidden}
      {...props}
    />
  );
};
