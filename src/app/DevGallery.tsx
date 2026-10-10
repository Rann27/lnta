import React, { useState } from "react";
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
import { TextField } from "../components/ui/TextField";
import { SecretField } from "../components/ui/SecretField";
import { NumberField } from "../components/ui/NumberField";
import { PathField } from "../components/ui/PathField";
import { Select } from "../components/ui/Select";
import { Checkbox } from "../components/ui/Checkbox";
import { Switch } from "../components/ui/Switch";
import { SegmentedControl } from "../components/ui/SegmentedControl";
import { SearchField } from "../components/ui/SearchField";
import { setTheme, type Theme } from "./theme";

export const DevGallery: React.FC = () => {
  const [currentTheme, setCurrentTheme] = useState<Theme>("system");
  const [textVal, setTextVal] = useState("Karamawari no Hibi");
  const [secretVal, setSecretVal] = useState("");
  const [isKeyStored, setIsKeyStored] = useState(true);
  const [numVal, setNumVal] = useState<number | undefined>(64000);
  const [pathVal, setPathVal] = useState("D:/LN/Karamawari 1");
  const [selectVal, setSelectVal] = useState("id");
  const [checkVal, setCheckVal] = useState(true);
  const [switchVal, setSwitchVal] = useState(false);
  const [searchVal, setSearchVal] = useState("");

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

  const handleThemeChange = (t: Theme) => {
    setCurrentTheme(t);
    setTheme(t);
  };

  return (
    <div style={{ padding: "var(--space-8)", maxWidth: "100%", margin: "0 auto" }}>
      <header style={{ marginBottom: "var(--space-8)" }}>
        <h1 style={{ font: "var(--text-display)", margin: "0 0 var(--space-4)" }}>
          Tally Component Gallery
        </h1>
        <SegmentedControl<Theme>
          options={[
            { value: "system", label: "Tema: Sistem" },
            { value: "light", label: "Tema: Terang" },
            { value: "dark", label: "Tema: Gelap" },
          ]}
          value={currentTheme}
          onChange={handleThemeChange}
        />
      </header>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Button & IconButton</h2>
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
          <IconButton
            aria-label="Buka folder proyek di Explorer"
            icon={<FolderOpen size={16} />}
          />
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Link, Tooltip & Kbd</h2>
        <div style={{ display: "flex", gap: "var(--space-6)", alignItems: "center" }}>
          <Link href="#section">Tautan lokal</Link>
          <Link href="https://example.com" external>
            Buka di Explorer
          </Link>
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
        <h2 style={{ font: "var(--text-title)" }}>Form & Input Primitives</h2>
        <div style={{ display: "grid", gap: "var(--space-6)", maxWidth: "var(--rail-width-max)" }}>
          <TextField
            label="Nama proyek"
            value={textVal}
            onChange={(e) => setTextVal(e.target.value)}
            hint="Nama volume LN yang sedang dikerjakan"
          />

          <TextField
            label="Nama proyek dengan error"
            value=""
            error="Nama proyek wajib diisi."
          />

          <SecretField
            label="API Key Anthropic (Tersimpan)"
            isStored={isKeyStored}
            onClear={() => setIsKeyStored(false)}
            onTestConnection={() => alert("Koneksi berhasil!")}
          />

          <SecretField
            label="API Key OpenAI (Belum tersimpan)"
            isStored={false}
            value={secretVal}
            onChange={setSecretVal}
            placeholder="contoh: sk-..."
          />

          <NumberField
            label="Batas konteks token"
            value={numVal}
            onChange={setNumVal}
            unit="token"
            hint="Maksimal token per panggilan model"
          />

          <PathField
            label="Folder proyek"
            value={pathVal}
            onChange={setPathVal}
            onBrowse={() => alert("Dialog browse dipicu")}
          />

          <Select
            label="Bahasa UI"
            value={selectVal}
            onChange={(e) => setSelectVal(e.target.value)}
            options={[
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
          />

          <div style={{ display: "flex", gap: "var(--space-6)", alignItems: "center" }}>
            <Checkbox
              label="Kirim bab sebelumnya"
              checked={checkVal}
              onChange={(e) => setCheckVal(e.target.checked)}
            />
            <Switch
              label="Notifikasi suara"
              checked={switchVal}
              onChange={(e) => setSwitchVal(e.target.checked)}
            />
          </div>

          <SearchField
            value={searchVal}
            onChange={setSearchVal}
            resultCountText={searchVal ? "5 dari 42 entri ditemukan" : undefined}
          />
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>ProgressBar & StateBlock</h2>
        <div style={{ display: "grid", gap: "var(--space-4)", maxWidth: "var(--rail-width-max)" }}>
          <ProgressBar label="Progress default" value={45} max={100} />
          <ProgressBar label="Progress live" variant="live" value={78} max={100} />
          <ProgressBar label="Progress paused" variant="paused" value={20} max={100} />
          <StateBlock
            title="Belum ada proyek"
            description="Pilih folder yang berisi file instruction dan file bab berprefix angka."
            action={<Button variant="primary">Buka folder proyek</Button>}
          />
        </div>
      </section>
    </div>
  );
};
