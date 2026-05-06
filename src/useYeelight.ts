import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type MiioResponse = {
  raw: string;
  json?: unknown;
};

export type CommandPayload = {
  ip: string;
  token: string;
  port: number;
  method: string;
  params: unknown[];
};

export type DiagnosticsPayload = {
  ip: string;
  port: number;
};

export type LightState = {
  ip: string;
  token: string;
  port: number;
  brightness: number;
  ct: number;
  gentleTransitions: boolean;
  moonlight: boolean;
  ambientOn: boolean;
  ambientBrightness: number;
  status: string;
  busy: boolean;
};

export type LightActions = {
  setIp: (ip: string) => void;
  setToken: (token: string) => void;
  setPort: (port: number) => void;
  setBrightness: (brightness: number) => void;
  setCt: (ct: number) => void;
  setGentleTransitions: (gentle: boolean) => void;
  setMoonlight: (moonlight: boolean) => void;
  setAmbientOn: (on: boolean) => void;
  setAmbientBrightness: (brightness: number) => void;
  setStatus: (status: string) => void;
  sendCommand: (method: string, params: unknown[]) => Promise<void>;
  runDiagnostics: () => Promise<void>;
  fetchState: () => Promise<void>;
};

const STORAGE_KEY = "yeelight-config";

function loadSavedConfig() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) return JSON.parse(saved);
  } catch {}
  return null;
}

export function useYeelight(): [LightState, LightActions] {
  const savedConfig = loadSavedConfig();

  const [ip, setIp] = useState(savedConfig?.ip ?? "");
  const [token, setToken] = useState(savedConfig?.token ?? "");
  const [port, setPort] = useState(savedConfig?.port ?? 54321);
  const [brightness, setBrightness] = useState(50);
  const [ct, setCt] = useState(4000);
  const [gentleTransitions, setGentleTransitions] = useState(true);
  const [moonlight, setMoonlight] = useState(false);
  const [ambientOn, setAmbientOn] = useState(false);
  const [ambientBrightness, setAmbientBrightness] = useState(50);
  const [status, setStatus] = useState("Ready.");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ip, token, port }));
  }, [ip, token, port]);

  const isTauriInvokeAvailable = () => {
    return typeof (window as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__
      ?.invoke === "function";
  };

  async function fetchState() {
    if (!ip.trim() || !token.trim()) return;
    if (!isTauriInvokeAvailable()) return;
    try {
      const response = await invoke<MiioResponse>("send_miio_command", {
        request: { ip: ip.trim(), token: token.trim(), port, method: "get_prop", params: ["power", "bright", "ct", "nl_br", "active_mode", "bg_power", "bg_bright"] },
      });
      if (response.json && typeof response.json === "object" && "result" in response.json) {
        const result = (response.json as { result: (string | null)[] }).result;
        const power = result[0];
        const bright = result[1] ? parseInt(result[1], 10) : 50;
        const ctVal = result[2] ? parseInt(result[2], 10) : 4000;
        const nlBr = result[3] ? parseInt(result[3], 10) : 0;
        const activeMode = result[4] ? parseInt(result[4], 10) : 0;
        const bgPower = result[5];
        const bgBright = result[6] ? parseInt(result[6], 10) : 50;
        setBrightness(nlBr > 0 ? nlBr : bright);
        setCt(ctVal);
        setMoonlight(activeMode === 1 || nlBr > 0);
        setAmbientOn(bgPower === "on");
        setAmbientBrightness(bgBright);
        setStatus(power === "on" ? "Connected. Light is on." : "Connected. Light is off.");
      }
    } catch (error) {
      console.error("Failed to fetch state:", error);
    }
  }

  useEffect(() => {
    if (ip.trim() && token.trim()) {
      void fetchState();
    }
  }, [ip, token, port]);

  const STATE_CHANGING_METHODS = [
    "set_power", "set_bright", "set_ct_abx", "set_scene", "toggle",
    "bg_set_power", "bg_set_bright",
  ];

  async function sendCommand(method: string, params: unknown[]) {
    if (!isTauriInvokeAvailable()) {
      setStatus("Tauri IPC unavailable. Open from Tauri window (npm run tauri dev), not a regular browser.");
      return;
    }
    if (!ip.trim() || !token.trim() || !Number.isFinite(port)) {
      setStatus("Please provide valid IP, token, and port.");
      return;
    }

    const payload: CommandPayload = { ip: ip.trim(), token: token.trim(), port, method, params };
    setBusy(true);
    setStatus(`Sending: ${method} ...`);
    try {
      const response = await invoke<MiioResponse>("send_miio_command", { request: payload });
      if (response.json) {
        setStatus(JSON.stringify(response.json, null, 2));
      } else if (response.raw.length > 0) {
        setStatus(response.raw);
      } else {
        setStatus("Command sent. Empty response payload.");
      }
      if (STATE_CHANGING_METHODS.includes(method)) {
        await fetchState();
      }
    } catch (error) {
      setStatus(`Error: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function runDiagnostics() {
    if (!isTauriInvokeAvailable()) {
      setStatus("Tauri IPC unavailable. Open from Tauri window (npm run tauri dev), not a regular browser.");
      return;
    }
    if (!ip.trim() || !Number.isFinite(port)) {
      setStatus("Please provide a valid IP and port for diagnostics.");
      return;
    }

    const payload: DiagnosticsPayload = { ip: ip.trim(), port };
    setBusy(true);
    setStatus("Running diagnostics...");
    try {
      const report = await invoke<unknown>("diagnose_connection", { request: payload });
      setStatus(JSON.stringify(report, null, 2));
    } catch (error) {
      setStatus(`Error: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  const state: LightState = {
    ip, token, port, brightness, ct, gentleTransitions, moonlight, ambientOn, ambientBrightness, status, busy,
  };

  const actions: LightActions = {
    setIp, setToken, setPort, setBrightness, setCt, setGentleTransitions, setMoonlight, setAmbientOn, setAmbientBrightness, setStatus, sendCommand, runDiagnostics, fetchState,
  };

  return [state, actions];
}