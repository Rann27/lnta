import React from "react";
import { ArrowSquareOut } from "@phosphor-icons/react";
import styles from "./Link.module.css";

export interface LinkProps extends React.AnchorHTMLAttributes<HTMLAnchorElement> {
  href: string;
  external?: boolean;
  children: React.ReactNode;
}

export const Link = React.forwardRef<HTMLAnchorElement, LinkProps>(
  ({ href, external = false, children, className, ...props }, ref) => {
    return (
      <a
        ref={ref}
        href={href}
        className={[styles.link, className ?? ""].filter(Boolean).join(" ")}
        target={external ? "_blank" : undefined}
        rel={external ? "noopener noreferrer" : undefined}
        aria-label={
          external && !props["aria-label"]
            ? `${typeof children === "string" ? children : ""} (membuka aplikasi lain)`
            : props["aria-label"]
        }
        {...props}
      >
        <span>{children}</span>
        {external && (
          <span className={styles.icon} aria-hidden="true">
            <ArrowSquareOut size={16} />
          </span>
        )}
      </a>
    );
  },
);

Link.displayName = "Link";
