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
  brightness,
  ct,
  moonlight,
  busy,
  connectionReady,
  transition,
  setBrightness,
  setCt,
  setMoonlight,
  onCommand,
}: Props) {
  return (
    <CollapsibleCard title="Main controls" defaultFolded={false}>
      <div className="space-y-4">
        <div className="grid gap-2 grid-cols-3">
          <Button
            className="w-full"
            disabled={busy || !connectionReady}
            onClick={() => onCommand("toggle", [])}
            title="Toggle power"
          >
            <Power className="h-4 w-4" />
            <span className="hidden sm:inline ml-2">Toggle</span>
          </Button>
          <Button
            variant="ghost"
            disabled={busy || !connectionReady}
            onClick={() =>
              onCommand("get_prop", [
                "power",
                "bright",
                "ct",
                "color_mode",
                "nl_br",
                "active_mode",
                "name",
              ])
            }
            title="Refresh state"
          >
            <RefreshCcw className="h-4 w-4" />
            <span className="hidden sm:inline ml-2">Refresh</span>
          </Button>
          <Button
            variant={moonlight ? "default" : "secondary"}
            disabled={busy || !connectionReady}
            onClick={() => {
              setMoonlight(!moonlight);
              onCommand("set_power", [
                "on",
                transition.effect,
                transition.duration,
                moonlight ? 1 : 5,
              ]);
            }}
            title={moonlight ? "Switch to full light" : "Switch to moonlight"}
          >
            <Moon className="h-4 w-4" />
            <span className="hidden sm:inline ml-2">{moonlight ? "Moonlight" : "Full Light"}</span>
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
            onMouseUp={() =>
              !busy &&
              connectionReady &&
              onCommand("set_bright", [brightness, transition.effect, transition.duration])
            }
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
            onMouseUp={() =>
              !busy &&
              connectionReady &&
              onCommand("set_ct_abx", [ct, transition.effect, transition.duration])
            }
          />
        </div>
      </div>
    </CollapsibleCard>
  );
}
