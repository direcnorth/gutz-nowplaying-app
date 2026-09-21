import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Config = {
  serverUrl: string;
  stageFullscreen: boolean;
  autostart: boolean;
  startView: string;
};

type View = "main" | "stage" | "requests" | "overlay" | "admin";

const VIEWS: { id: View; label: string }[] = [
  { id: "main", label: "Startseite" },
  { id: "stage", label: "Bühne" },
  { id: "requests", label: "Anfragen" },
  { id: "overlay", label: "Wunsch-Overlay" },
  { id: "admin", label: "Admin" },
];

export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState<boolean | null>(null);
  const [message, setMessage] = useState<string>("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    invoke<Config>("get_config").then(setConfig);
    invoke<boolean | null>("server_status").then(setStatus);
    const unlisten = listen<{ ok: boolean }>("server-status", (e) => setStatus(e.payload.ok));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  if (!config) return <main className="container">Lade…</main>;

  const update = (patch: Partial<Config>) => setConfig({ ...config, ...patch });

  async function save() {
    setBusy(true);
    setMessage("");
    try {
      await invoke("set_config", { config });
      setMessage("Gespeichert.");
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function test() {
    setBusy(true);
    setMessage("");
    try {
      const ok = await invoke<boolean>("check_server", { config });
      setMessage(ok ? "Server erreichbar." : "Server antwortet nicht (GET /healthz).");
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function open(view: View) {
    try {
      await invoke("open_view_cmd", { view });
    } catch (err) {
      setMessage(String(err));
    }
  }

  return (
    <main className="container">
      <h1>Gutz Now Playing</h1>

      <p className={`status ${status === null ? "" : status ? "ok" : "down"}`}>
        Server: {status === null ? "noch nicht geprüft" : status ? "erreichbar" : "nicht erreichbar"}
      </p>

      <label>
        Server-URL
        <input
          type="url"
          value={config.serverUrl}
          placeholder="http://gutz-bmax:8080"
          onChange={(e) => update({ serverUrl: e.currentTarget.value })}
        />
      </label>

      <label className="check">
        <input
          type="checkbox"
          checked={config.stageFullscreen}
          onChange={(e) => update({ stageFullscreen: e.currentTarget.checked })}
        />
        Bühne im Vollbild (Kiosk) öffnen
      </label>

      <label className="check">
        <input
          type="checkbox"
          checked={config.autostart}
          onChange={(e) => update({ autostart: e.currentTarget.checked })}
        />
        Beim Anmelden automatisch starten
      </label>

      <label>
        Beim Start automatisch öffnen
        <select
          value={config.startView}
          onChange={(e) => update({ startView: e.currentTarget.value })}
        >
          <option value="main">Startseite (Normalfall)</option>
          <option value="requests">Anfragen (nur BARPC)</option>
        </select>
      </label>

      <div className="row">
        <button onClick={test} disabled={busy}>
          Verbindung testen
        </button>
        <button onClick={save} disabled={busy} className="primary">
          Speichern
        </button>
      </div>

      {message && <p className="message">{message}</p>}

      <h2>Fenster öffnen</h2>
      <div className="row wrap">
        {VIEWS.map((v) => (
          <button key={v.id} onClick={() => open(v.id)}>
            {v.label}
          </button>
        ))}
      </div>

      <p className="hint">Alle Ansichten kommen vom Server. Das Tray-Menü bietet dieselben Einträge.</p>
    </main>
  );
}
