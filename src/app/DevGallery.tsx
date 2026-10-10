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
import { Panel } from "../components/ui/Panel";
import { SplitDivider } from "../components/ui/SplitDivider";
import { Banner } from "../components/ui/Banner";
import { Dialog } from "../components/ui/Dialog";
import { Popover } from "../components/ui/Popover";
import { ContextMenu } from "../components/ui/ContextMenu";
import { Toast } from "../components/ui/Toast";
import { Disclosure } from "../components/ui/Disclosure";
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
  const [splitWidth, setSplitWidth] = useState(300);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isDestructiveOpen, setIsDestructiveOpen] = useState(false);
  const [isToastOpen, setIsToastOpen] = useState(false);

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
        <h2 style={{ font: "var(--text-title)" }}>Banner</h2>
        <div style={{ display: "grid", gap: "var(--space-4)", maxWidth: "var(--rail-width-max)" }}>
          <Banner
            variant="blocker"
            title="Translations.docx sedang terkunci"
            description="Tutup file tersebut di Microsoft Word, lalu coba render lagi."
            action={<Button size="sm" variant="secondary">Coba lagi</Button>}
          />
          <Banner
            variant="attention"
            title="Glosarium memiliki 3 entri konflik"
            description="Tinjau usulan istilah baru di tab Glosarium sebelum melanjutkan."
            onClose={() => alert("Banner ditutup")}
          />
        </div>
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>Containment & Feedback</h2>
        <div style={{ display: "flex", gap: "var(--space-4)", flexWrap: "wrap", alignItems: "center" }}>
          <Button variant="secondary" onClick={() => setIsDialogOpen(true)}>
            Buka Dialog Default
          </Button>
          <Button variant="danger" onClick={() => setIsDestructiveOpen(true)}>
            Buka Dialog Destruktif
          </Button>
          <Button variant="secondary" onClick={() => setIsToastOpen(true)}>
            Tampilkan Toast (6s)
          </Button>
          <Popover
            trigger={<Button variant="secondary">Buka Popover</Button>}
            content={
              <div>
                <h4 style={{ margin: "0 0 var(--space-2)", font: "var(--text-body-strong)" }}>
                  Detail Panggilan
                </h4>
                <p style={{ margin: 0, font: "var(--text-small)", color: "var(--color-text-2)" }}>
                  Model: claude-3-7-sonnet<br />
                  Token masuk: 4.120<br />
                  Latensi: 2,4 s
                </p>
              </div>
            }
          />
        </div>

        <Dialog
          isOpen={isDialogOpen}
          onClose={() => setIsDialogOpen(false)}
          title="Konfirmasi pengaturan"
          description="Apakah Anda ingin menerapkan perubahan ke seluruh bab?"
          actions={
            <>
              <Button variant="secondary" onClick={() => setIsDialogOpen(false)}>
                Batal
              </Button>
              <Button variant="primary" onClick={() => setIsDialogOpen(false)}>
                Terapkan
              </Button>
            </>
          }
        />

        <Dialog
          isOpen={isDestructiveOpen}
          onClose={() => setIsDestructiveOpen(false)}
          variant="destructive"
          title="Hapus cache terjemahan bab 04?"
          description="Semua segmen hasil model akan direset kembali ke teks mentah."
          actions={
            <>
              <Button variant="secondary" onClick={() => setIsDestructiveOpen(false)}>
                Batal
              </Button>
              <Button variant="danger" onClick={() => setIsDestructiveOpen(false)}>
                Hapus cache
              </Button>
            </>
          }
        />

        <Toast
          isOpen={isToastOpen}
          onClose={() => setIsToastOpen(false)}
          variant="success"
          title="Glosarium tersimpan"
          message="Perubahan istilah telah ditulis ke file glossary.md."
        />
      </section>

      <section style={{ marginBottom: "var(--space-8)" }}>
        <h2 style={{ font: "var(--text-title)" }}>ContextMenu, Disclosure & SplitDivider</h2>
        <div style={{ display: "grid", gap: "var(--space-4)", maxWidth: "var(--rail-width-max)" }}>
          <ContextMenu
            items={[
              { label: "Buka file raw", onClick: () => alert("Buka file") },
              { label: "Salin path", onClick: () => alert("Salin path"), shortcut: "Ctrl+C" },
              {
                label: "Hapus segmen",
                destructive: true,
                separatorBefore: true,
                onClick: () => alert("Hapus"),
              },
            ]}
          >
            <Panel style={{ textAlign: "center", cursor: "context-menu" }}>
              Klik kanan area panel ini untuk membuka menu konteks (atau Shift+F10)
            </Panel>
          </ContextMenu>

          <Disclosure title="Informasi teknis dan token">
            <p style={{ margin: 0, font: "var(--text-mono)", color: "var(--color-text-2)" }}>
              Context: 128k · Cache read: 3.200 · Latency: 1.4s
            </p>
          </Disclosure>

          <div style={{ display: "flex", alignItems: "center", height: "var(--space-10)", background: "var(--color-sunken)", padding: "0 var(--space-4)" }}>
            <span style={{ font: "var(--text-small)", color: "var(--color-text-2)" }}>Lebar panel: {splitWidth}px</span>
            <SplitDivider
              value={splitWidth}
              min={240}
              max={420}
              onChange={setSplitWidth}
            />
          </div>
        </div>
      </section>

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
