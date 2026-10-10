import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import styles from "./WaitingCounter.module.css";

export interface WaitingCounterProps {
  tokensReceived?: number;
  initialSeconds?: number;
  isActive?: boolean;
}

export const WaitingCounter: React.FC<WaitingCounterProps> = ({
  tokensReceived = 0,
  initialSeconds = 0,
  isActive = true,
}) => {
  const { t } = useTranslation();
  const [seconds, setSeconds] = useState(initialSeconds);

  useEffect(() => {
    if (!isActive) return;
    const interval = setInterval(() => {
      setSeconds((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [isActive]);

  const hasTokens = tokensReceived > 0;
  const label = hasTokens
    ? t("workspace.run_tab.received_tokens", {
        tokens: tokensReceived.toLocaleString(),
        seconds,
      })
    : t("workspace.run_tab.waiting_response", {
        seconds,
      });

  return (
    <span className={styles.counter} role="timer" aria-live="off">
      {label}
    </span>
  );
};
