import { Settings2 } from "lucide-react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { CollapsibleCard } from "./CollapsibleCard";

type Props = {
  ip: string;
  token: string;
  port: number;
  busy: boolean;
  setIp: (ip: string) => void;
  setToken: (token: string) => void;
  setPort: (port: number) => void;
  onRunDiagnostics: () => void;
};

export function ConnectionCard({ ip, token, port, busy, setIp, setToken, setPort, onRunDiagnostics }: Props) {
  const isConnected = ip.trim().length > 0 && token.trim().length > 0;

  return (
    <CollapsibleCard title="Connection" defaultFolded={isConnected}>
      <div className="grid gap-3 md:grid-cols-3">
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
            onClick={onRunDiagnostics}
          >
            <Settings2 className="mr-2 h-4 w-4" />
            Run diagnostics
          </Button>
        </div>
      </div>
    </CollapsibleCard>
  );
}