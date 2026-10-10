import React from "react";
import styles from "./Kbd.module.css";

export interface KbdProps extends React.HTMLAttributes<HTMLElement> {
  children: React.ReactNode;
}

export const Kbd: React.FC<KbdProps> = ({ children, className, ...props }) => {
  return (
    <kbd
      className={[styles.kbd, className ?? ""].filter(Boolean).join(" ")}
      {...props}
    >
      {children}
    </kbd>
  );
};
