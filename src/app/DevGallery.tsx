import React from "react";
import { FolderOpen, Play, Pause, Trash } from "@phosphor-icons/react";
import { Button } from "../components/ui/Button";
import { IconButton } from "../components/ui/IconButton";
import { Link } from "../components/ui/Link";
import { Tooltip } from "../components/ui/Tooltip";
import { Kbd } from "../components/ui/Kbd";
import { StatusTag, type StatusType } from "../components/ui/StatusTag";
import { TallyLamp } from "../components/ui/TallyLamp";
import { ProgressBar } from "../components/ui/ProgressBar";
import { StateBlock } from "../components/ui/StateBlock";
import { setTheme } from "./theme";

export const DevGallery: React.FC = () => {
  const allStatuses: StatusType[] = [
    "queued",
    "running",
    "paused",
    "done",
    "needs_attention",
    "blocked",
    "frozen",
    "source_changed",
  ];

  return (
    <div style={{ padding: "var(--space-8)", maxWidth: "100%", margin: "0 auto" }}>
      <header style={{ marginBottom: "var(--space-8)" }}>
        <h1 style={{ font: "var(--text-display)", margin: "0 0 var(--space-4)" }}>
          Tally Component Gallery
        </h1>
        <div style={{ display: "flex", gap: "var(--space-4)" }}>
          <Button size="sm" variant="secondary" onClick={() => setTheme("system")}>
            Tema: Sistem
          </Button>
          <Button size="sm" variant="secondary" onClick={() => setTheme("light")}>
            Tema: Terang
          </Button>
          <Button size="sm" variant="secondary" onClick={() => setTheme("dark")}>
            Tema: Gelap
          </Button>
        </div>
      </header>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Button</h2>
        <div style={{ display: "flex", gap: "var(--space-4)", flexWrap: "wrap", alignItems: "center" }}>
          <Button variant="primary" icon={<Play size={16} />}>
            Jalankan volume
          </Button>
          <Button variant="secondary" icon={<Pause size={16} />}>
            Jeda
          </Button>
          <Button variant="ghost">Salin log</Button>
          <Button variant="danger" icon={<Trash size={16} />}>
            Reset proyek
          </Button>
          <Button variant="primary" size="sm">
            Kecil
          </Button>
          <Button
            variant="secondary"
            disabled
            disabledReason="File instruction belum ditemukan."
          >
            Jalankan volume
          </Button>
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Icon Button</h2>
        <div style={{ display: "flex", gap: "var(--space-4)", alignItems: "center" }}>
          <IconButton
            aria-label="Buka folder proyek di Explorer"
            icon={<FolderOpen size={16} />}
          />
          <IconButton
            aria-label="Jalankan volume"
            variant="primary"
            icon={<Play size={16} />}
          />
          <IconButton
            aria-label="Jeda run"
            variant="secondary"
            icon={<Pause size={16} />}
          />
          <IconButton
            aria-label="Hapus cache"
            variant="danger"
            icon={<Trash size={16} />}
          />
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Link</h2>
        <div style={{ display: "flex", gap: "var(--space-6)" }}>
          <Link href="#section">Tautan lokal</Link>
          <Link href="https://example.com" external>
            Buka di Explorer
          </Link>
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Tooltip & Kbd</h2>
        <div style={{ display: "flex", gap: "var(--space-6)", alignItems: "center" }}>
          <Tooltip content="Membuka folder proyek yang sedang aktif">
            <Button variant="secondary">Arahkan kursor ke sini</Button>
          </Tooltip>
          <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
            <Kbd>Ctrl</Kbd>
            <Kbd>Shift</Kbd>
            <Kbd>R</Kbd>
            <span style={{ font: "var(--text-small)", color: "var(--color-text-2)" }}>
              Jalankan atau lanjutkan
            </span>
          </div>
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>StatusTag & TallyLamp</h2>
        <div style={{ display: "flex", gap: "var(--space-4)", flexWrap: "wrap", alignItems: "center" }}>
          <TallyLamp />
          {allStatuses.map((st) => (
            <StatusTag key={st} status={st} />
          ))}
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>ProgressBar</h2>
        <div style={{ display: "grid", gap: "var(--space-4)", maxWidth: "var(--rail-width-max)" }}>
          <ProgressBar label="Progress default" value={45} max={100} />
          <ProgressBar label="Progress live" variant="live" value={78} max={100} />
          <ProgressBar label="Progress paused" variant="paused" value={20} max={100} />
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>StateBlock</h2>
        <div style={{ display: "grid", gap: "var(--space-4)" }}>
          <StateBlock
            title="Belum ada proyek"
            description="Pilih folder yang berisi file instruction dan file bab berprefix angka."
            action={<Button variant="primary">Buka folder proyek</Button>}
          />
          <StateBlock
            title="Memuat 1.240 segmen bab 04"
            description="Mengambil data bab dari database lokal."
          >
            <ProgressBar label="Memuat bab" variant="live" value={60} max={100} />
          </StateBlock>
        </div>
      </section>
    </div>
  );
};
