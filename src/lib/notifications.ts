import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { isTauri } from "@tauri-apps/api/core";

export interface NotificationPayload {
  title: string;
  body: string;
}

/**
 * Sends an OS-level notification per FR-RUN-11.
 * Supports Tauri plugin notifications with browser Notification API fallback.
 */
export async function notifyOS(payload: NotificationPayload): Promise<void> {
  try {
    if (isTauri()) {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      if (granted) {
        sendNotification({
          title: payload.title,
          body: payload.body,
        });
      }
    } else if (typeof window !== "undefined" && "Notification" in window) {
      if (Notification.permission === "granted") {
        new Notification(payload.title, { body: payload.body });
      } else if (Notification.permission !== "denied") {
        const perm = await Notification.requestPermission();
        if (perm === "granted") {
          new Notification(payload.title, { body: payload.body });
        }
      }
    }
  } catch {
    // Graceful silent fallback if notifications are blocked or unavailable
  }
}
