import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Moon, Power, RefreshCcw, Settings2, SunMedium, ThermometerSun, Zap } from "lucide-react";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { Input } from "./components/ui/input";
import { Label } from "./components/ui/label";
import { Switch } from "./components/ui/switch";

type MiioResponse = {
  raw: string;
  json?: unknown;
};

type CommandPayload = {
  ip: string;
  token: string;
  port: number;
  method: string;
  params: unknown[];
};

type DiagnosticsPayload = {
  ip: string;
  port: number;
};

function isTauriInvokeAvailable() {
  return typeof (window as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__
    ?.invoke === "function";
}

export function App() {
  const [ip, setIp] = useState("");
  const [token, setToken] = useState("");
  const [port, setPort] = useState(54321);
  const [brightness, setBrightness] = useState(50);
  const [ct, setCt] = useState(4000);
  const [gentleTransitions, setGentleTransitions] = useState(true);
  const [status, setStatus] = useState("Ready.");
  const [busy, setBusy] = useState(false);

  const transition = useMemo(
    () => ({
      effect: gentleTransitions ? "smooth" : "sudden",
      duration: gentleTransitions ? 500 : 30,
    }),
    [gentleTransitions]
  );

  async function sendCommand(method: string, params: unknown[]) {
    if (!isTauriInvokeAvailable()) {
      setStatus(
        "Tauri IPC unavailable. Open from Tauri window (npm run tauri dev), not a regular browser."
      );
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
    } catch (error) {
      setStatus(`Error: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function runDiagnostics() {
    if (!isTauriInvokeAvailable()) {
      setStatus(
        "Tauri IPC unavailable. Open from Tauri window (npm run tauri dev), not a regular browser."
      );
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

  const connectionReady = ip.trim().length > 0 && token.trim().length > 0;

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-5xl flex-col gap-4 p-4 md:p-8">
      <section className="mb-1">
        <h1 className="text-3xl font-semibold tracking-tight text-slate-100">Yeelight Controller</h1>
        <p className="mt-1 text-sm text-slate-400">
          Fast, friendly controls for your ceiling light via miIO.
        </p>
      </section>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings2 className="h-4 w-4 text-blue-300" />
            Connection
          </CardTitle>
          <CardDescription>Set this once, then use the controls below.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-3">
          <div className="space-y-2">
            <Label htmlFor="ip">Light IP</Label>
            <Input id="ip" placeholder="192.168.1.120" value={ip} onChange={(e) => setIp(e.target.value)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="token">Token</Label>
            <Input
              id="token"
              placeholder="0123456789abcdef0123456789abcdef"
              value={token}
              onChange={(e) => setToken(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="port">Port</Label>
            <Input
              id="port"
              type="number"
              min={1}
              max={65535}
              value={port}
              onChange={(e) => setPort(Number(e.target.value || 54321))}
            />
          </div>
          <div className="md:col-span-3">
            <Button
              type="button"
              variant="secondary"
              disabled={busy || !ip.trim()}
              onClick={() => void runDiagnostics()}
            >
              Run network diagnostics
            </Button>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>Main controls</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-2 sm:grid-cols-3">
              <Button disabled={busy || !connectionReady} onClick={() => void sendCommand("set_power", ["on", transition.effect, transition.duration])}>
                <Power className="mr-2 h-4 w-4" /> On
              </Button>
              <Button
                variant="secondary"
                disabled={busy || !connectionReady}
                onClick={() => void sendCommand("set_power", ["off", transition.effect, transition.duration])}
              >
                Off
              </Button>
              <Button variant="secondary" disabled={busy || !connectionReady} onClick={() => void sendCommand("toggle", [])}>
                Toggle
              </Button>
            </div>

            <div className="grid gap-2 sm:grid-cols-3">
              <Button
                variant="secondary"
                disabled={busy || !connectionReady}
                onClick={() => void sendCommand("set_power", ["on", transition.effect, transition.duration, 5])}
              >
                <Moon className="mr-2 h-4 w-4" /> Moonlight On
              </Button>
              <Button
                variant="secondary"
                disabled={busy || !connectionReady}
                onClick={() => void sendCommand("set_power", ["on", transition.effect, transition.duration, 1])}
              >
                Moonlight Off
              </Button>
              <Button
                variant="ghost"
                disabled={busy || !connectionReady}
                onClick={() =>
                  void sendCommand("get_prop", [
                    "power",
                    "bright",
                    "ct",
                    "color_mode",
                    "nl_br",
                    "active_mode",
                    "name",
                  ])
                }
              >
                <RefreshCcw className="mr-2 h-4 w-4" /> Refresh state
              </Button>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <Label htmlFor="brightness">Brightness</Label>
                <span className="text-slate-400">{brightness}%</span>
              </div>
              <input
                id="brightness"
                className="w-full accent-blue-500"
                type="range"
                min={1}
                max={100}
                value={brightness}
                onChange={(e) => setBrightness(Number(e.target.value))}
              />
              <Button
                variant="secondary"
                disabled={busy || !connectionReady}
                onClick={() => void sendCommand("set_bright", [brightness, transition.effect, transition.duration])}
              >
                <SunMedium className="mr-2 h-4 w-4" /> Apply brightness
              </Button>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <Label htmlFor="ct">Color temperature</Label>
                <span className="text-slate-400">{ct}K</span>
              </div>
              <input
                id="ct"
                className="w-full accent-amber-400"
                type="range"
                min={1700}
                max={6500}
                value={ct}
                onChange={(e) => setCt(Number(e.target.value))}
              />
              <Button
                variant="secondary"
                disabled={busy || !connectionReady}
                onClick={() => void sendCommand("set_ct_abx", [ct, transition.effect, transition.duration])}
              >
                <ThermometerSun className="mr-2 h-4 w-4" /> Apply temperature
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Quick presets</CardTitle>
            <CardDescription>One-click everyday scenes.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => void sendCommand("set_scene", ["ct", 2700, 20])}>
              Relax (warm)
            </Button>
            <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => void sendCommand("set_scene", ["ct", 3500, 65])}>
              Reading
            </Button>
            <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => void sendCommand("set_scene", ["ct", 4300, 80])}>
              Focus
            </Button>
            <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => void sendCommand("set_scene", ["ct", 5500, 100])}>
              Daylight
            </Button>

            <div className="mt-4 flex items-center justify-between rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2">
              <div className="flex items-center gap-2">
                <Zap className="h-4 w-4 text-blue-300" />
                <span className="text-sm text-slate-200">Gentle transitions</span>
              </div>
              <Switch checked={gentleTransitions} onCheckedChange={setGentleTransitions} />
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Device response</CardTitle>
          <CardDescription>Last response from your light.</CardDescription>
        </CardHeader>
        <CardContent>
          <pre className="max-h-96 overflow-auto rounded-lg border border-slate-800 bg-slate-950 p-4 text-xs text-slate-200">
            {status}
          </pre>
        </CardContent>
      </Card>
    </main>
  );
}
