import { Zap } from "lucide-react";
import { Button } from "./ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Switch } from "./ui/switch";

type Props = {
  gentleTransitions: boolean;
  busy: boolean;
  connectionReady: boolean;
  onToggleTransitions: (value: boolean) => void;
  onCommand: (method: string, params: unknown[]) => void;
};

export function PresetsCard({ gentleTransitions, busy, connectionReady, onToggleTransitions, onCommand }: Props) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Quick presets</CardTitle>
        <CardDescription>One-click everyday scenes.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-2">
        <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => onCommand("set_scene", ["ct", 2700, 20])}>
          Relax (warm)
        </Button>
        <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => onCommand("set_scene", ["ct", 3500, 65])}>
          Reading
        </Button>
        <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => onCommand("set_scene", ["ct", 4300, 80])}>
          Focus
        </Button>
        <Button variant="secondary" className="w-full" disabled={busy || !connectionReady} onClick={() => onCommand("set_scene", ["ct", 5500, 100])}>
          Daylight
        </Button>

        <div className="mt-4 flex items-center justify-between rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2">
          <div className="flex items-center gap-2">
            <Zap className="h-4 w-4 text-blue-300" />
            <span className="text-sm text-slate-200">Gentle transitions</span>
          </div>
          <Switch checked={gentleTransitions} onCheckedChange={onToggleTransitions} />
        </div>
      </CardContent>
    </Card>
  );
}