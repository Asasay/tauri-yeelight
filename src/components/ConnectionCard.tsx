import { Settings2 } from "lucide-react";
import { Button } from "./ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

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
  return (
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
            onClick={onRunDiagnostics}
          >
            Run network diagnostics
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}