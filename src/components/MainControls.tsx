import { Moon, Power, RefreshCcw } from "lucide-react";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { CollapsibleCard } from "./CollapsibleCard";

type Props = {
  brightness: number;
  ct: number;
  moonlight: boolean;
  busy: boolean;
  connectionReady: boolean;
  transition: { effect: string; duration: number };
  setBrightness: (b: number) => void;
  setCt: (ct: number) => void;
  setMoonlight: (m: boolean) => void;
  onCommand: (method: string, params: unknown[]) => void;
};

export function MainControls({
  brightness, ct, moonlight, busy, connectionReady, transition,
  setBrightness, setCt, setMoonlight, onCommand,
}: Props) {
  return (
    <CollapsibleCard title="Main controls" defaultFolded={false}>
      <div className="space-y-4">
        <div className="grid gap-2 sm:grid-cols-2">
          <Button className="w-full" disabled={busy || !connectionReady} onClick={() => onCommand("toggle", [])}>
            <Power className="mr-2 h-4 w-4" /> Toggle
          </Button>
          <Button variant="ghost" disabled={busy || !connectionReady} onClick={() => onCommand("get_prop", ["power", "bright", "ct", "color_mode", "nl_br", "active_mode", "name"])}>
            <RefreshCcw className="mr-2 h-4 w-4" /> Refresh
          </Button>
        </div>

        <div className="flex">
          <Button
            variant={moonlight ? "default" : "secondary"}
            disabled={busy || !connectionReady}
            onClick={() => {
              setMoonlight(!moonlight);
              onCommand("set_power", ["on", transition.effect, transition.duration, moonlight ? 1 : 5]);
            }}
          >
            <Moon className="mr-2 h-4 w-4" />
            {moonlight ? "Moonlight" : "Full Light"}
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
            onMouseUp={() => !busy && connectionReady && onCommand("set_bright", [brightness, transition.effect, transition.duration])}
          />
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
            onMouseUp={() => !busy && connectionReady && onCommand("set_ct_abx", [ct, transition.effect, transition.duration])}
          />
        </div>
      </div>
    </CollapsibleCard>
  );
}