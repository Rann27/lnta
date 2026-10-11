import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { formatNumber } from "../../../lib/format";
import styles from "./WaitingCounter.module.css";

export interface WaitingCounterProps {
  /** Epoch ms when the current model call started; null shows nothing. */
  startedAt: number | null;
  /** Characters streamed so far by the current call. */
  charsReceived?: number;
  isActive?: boolean;
}

/** Elapsed time of the running model call, used instead of a spinner (MOTION 1). */
export const WaitingCounter: React.FC<WaitingCounterProps> = ({
  startedAt,
  charsReceived = 0,
  isActive = true,
}) => {
  const { t } = useTranslation();
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!isActive || startedAt === null) return;
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, [isActive, startedAt]);

  if (startedAt === null) return null;
  const seconds = Math.max(0, Math.floor((now - startedAt) / 1000));
  const label =
    charsReceived > 0
      ? t("workspace.run_tab.received_chars", { chars: formatNumber(charsReceived), seconds })
      : t("workspace.run_tab.waiting_response", { seconds });

  return (
    <span className={styles.counter} role="timer" aria-live="off">
      {label}
    </span>
  );
};
