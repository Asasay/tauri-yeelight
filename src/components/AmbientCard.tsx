import { Label } from "./ui/label";
import { Switch } from "./ui/switch";
import { CollapsibleCard } from "./CollapsibleCard";

type Props = {
  ambientOn: boolean;
  ambientBrightness: number;
  busy: boolean;
  connectionReady: boolean;
  transition: { effect: string; duration: number };
  setAmbientOn: (on: boolean) => void;
  setAmbientBrightness: (b: number) => void;
  onCommand: (method: string, params: unknown[]) => void;
};

export function AmbientCard({ ambientOn, ambientBrightness, busy, connectionReady, transition, setAmbientOn, setAmbientBrightness, onCommand }: Props) {
  return (
    <CollapsibleCard title="Ambient light" defaultFolded={false}>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <span className="text-sm text-slate-300">Ambient</span>
          <Switch
            checked={ambientOn}
            onCheckedChange={(checked) => {
              setAmbientOn(checked);
              onCommand("bg_set_power", [checked ? "on" : "off", transition.effect, transition.duration]);
            }}
          />
        </div>
        <div className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <Label htmlFor="ambient-brightness">Brightness</Label>
            <span className="text-slate-400">{ambientBrightness}%</span>
          </div>
          <input
            id="ambient-brightness"
            className="w-full accent-purple-500"
            type="range"
            min={1}
            max={100}
            value={ambientBrightness}
            onChange={(e) => setAmbientBrightness(Number(e.target.value))}
            onMouseUp={() => !busy && connectionReady && ambientOn && onCommand("bg_set_bright", [ambientBrightness, transition.effect, transition.duration])}
          />
        </div>
      </div>
    </CollapsibleCard>
  );
}